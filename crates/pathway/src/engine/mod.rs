//! `PathwayEngine` implementation of the shared routing engine contract.
//!
//! The engine is generic over topology, transport, retention, runtime
//! effects, hashing, and committee selector so the same state machine runs
//! under production adapters, tests, and simulation. Planning is a pure
//! read against explicit topology observations. Runtime mutation is
//! confined to `materialize_route`, `maintain_route`, and `teardown`.
//! Canonical route identity, handles, and leases come exclusively
//! from the router.
//!
//! This module owns the engine type, workspace constants, and the
//! non-trait helper methods (`forward_payload`, `retain_for_route`, the
//! hash-derived ID helpers). `planner` implements `RoutingEnginePlanner`,
//! `runtime` implements `RoutingEngine`/`PathwayRoutingEngine`, `support`
//! holds the pure helpers shared between them, `types` defines the
//! pathway-private object model, and `trait_bounds` defines the internal
//! bound aliases used in `impl` headers.

#![allow(private_bounds)]

mod planner;
mod runtime;
mod support;
mod trait_bounds;
mod types;

use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
};

use jacquard_core::{
    Blake3Digest, Configuration, ConnectivityPosture, ContentId, NodeId, Observation,
    ReceiptId, RouteCommitmentId, RouteEpoch, RouteError, RouteId, RoutePartitionClass,
    RouteRuntimeError, RouteSelectionError, RoutingEngineCapabilities, RoutingEngineId,
    TransportObservation,
};
use jacquard_traits::{Blake3Hashing, HashDigestBytes, Hashing, RouterManagedEngine};
pub(crate) use support::{
    current_segment, digest_prefix, MaintenanceResultExt, StorageResultExt,
    DOMAIN_TAG_COMMITTEE_ID,
};
use trait_bounds::{
    PathwayEffectsBounds, PathwayHasherBounds, PathwayRetentionBounds,
    PathwaySelectorBounds, PathwayTopologyBounds, PathwayTransportBounds,
};
use types::{ActivePathwayRoute, CachedCandidate};
pub use types::{
    PathwayActiveRouteView, PathwayControlState, PathwayForwardingCursor,
    PathwayObservedRemoteLink, PathwayRoundProgress, PathwayRoundReport,
    PathwayRoundWaitState, PathwayRouteClass, PathwayRouteRetentionView,
    PathwayTransportFreshness, PathwayTransportObservationSummary,
};
pub(crate) use types::{
    PathwayForwardingState, PathwayHandoffState, PathwayPath, PathwayRepairState,
    PathwayRouteAntiEntropyState, PathwayRouteSegment,
};

use crate::{
    choreography, committee::NoCommitteeSelector, PathwayNeighborhoodEstimateAccess,
    PathwayPeerEstimateAccess,
};

// Public Engine Identity And Capability Surface

pub const PATHWAY_ENGINE_ID: RoutingEngineId =
    RoutingEngineId::from_contract_bytes(*b"jacquard.mesh.v1");

/// Maximum number of concurrently active materialized routes this engine
/// will hold. New materializations past this point fail with
/// `RoutePolicyError::BudgetExceeded`.
pub(crate) const PATHWAY_ACTIVE_ROUTE_COUNT_MAX: usize = 64;

/// Maximum number of candidate entries the planner emits per tick.
/// Sorting and truncation happen after BFS so the cap is deterministic.
pub(crate) const PATHWAY_CANDIDATE_COUNT_MAX: usize = 32;

/// Maximum number of retained payload objects tracked per active route.
pub(crate) const PATHWAY_RETAINED_PER_ROUTE_COUNT_MAX: usize = 32;
pub(crate) const PATHWAY_PENDING_TRANSPORT_INGRESS_COUNT_MAX: usize = 64;

/// Validity window applied to newly derived pathway candidates, in ticks.
pub(crate) const PATHWAY_CANDIDATE_VALIDITY_TICKS: u64 = 12;

/// Per-hop byte cost used in `RouteCost` derivation.
pub(crate) const PATHWAY_PER_HOP_BYTE_COST: u64 = 1024;

/// Hold capacity reserved for deferred-delivery routes, in bytes.
pub(crate) const PATHWAY_HOLD_RESERVED_BYTES: u64 = 1024;

// Route-commitment retry budget. Pathway uses a short, fixed policy because
// commitments represent already-admitted routes; exceeding the budget is
// a teardown signal rather than a reason to keep retrying.
const PATHWAY_COMMITMENT_ATTEMPT_COUNT_MAX: u32 = 2;
// Fixed-delay by design: commitments cover already-admitted routes, so
// the retry ceiling intentionally equals the initial backoff.
const PATHWAY_COMMITMENT_INITIAL_BACKOFF_MS: u32 = 25;
const PATHWAY_COMMITMENT_BACKOFF_MS_MAX: u32 = 25;
const PATHWAY_COMMITMENT_OVERALL_TIMEOUT_MS: u32 = 50;

// Pathway advertises link-level protection, explicit route shape, and full
// repair, hold, and decidable-admission support. This is the static
// capability envelope the router sees during engine registration.
pub const PATHWAY_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: PATHWAY_ENGINE_ID,
    max_protection: jacquard_core::RouteProtectionClass::LinkProtected,
    max_connectivity: ConnectivityPosture {
        repair: jacquard_core::RouteRepairClass::Repairable,
        partition: RoutePartitionClass::PartitionTolerant,
    },
    repair_support: jacquard_core::RepairSupport::Supported,
    hold_support: jacquard_core::HoldSupport::Supported,
    decidable_admission: jacquard_core::DecidableSupport::Supported,
    quantitative_bounds:
        jacquard_core::QuantitativeBoundSupport::ProductiveAndSchedulerLifted,
    reconfiguration_support: jacquard_core::ReconfigurationSupport::LinkAndDelegate,
    route_shape_visibility: jacquard_core::RouteShapeVisibility::ExplicitPath,
};

// `candidate_cache` memoizes planning work so `check_candidate` and
// `admit_route` can reuse the admission check and witness derived during
// `candidate_routes` without recomputing them. The cache is an
// optimization only: `BackendRouteRef` is a self-contained opaque plan
// token, so planner cache misses and materialization must still work.
// It is `RefCell<...>` because the planner trait methods take `&self`.
// `active_routes` holds the pathway-private runtime state for each
// materialized route. Canonical identity lives on the router side.
pub struct PathwayEngine<
    Topology,
    Transport,
    Retention,
    Effects,
    Hasher,
    Selector = NoCommitteeSelector,
> {
    local_node_id: NodeId,
    topology_model: Topology,
    transport: Transport,
    retention: Retention,
    effects: Effects,
    hashing: Hasher,
    selector: Selector,
    pending_transport_ingress: VecDeque<TransportObservation>,
    dropped_transport_ingress_since_last_tick: usize,
    latest_topology: Option<Observation<Configuration>>,
    last_transport_summary: Option<PathwayTransportObservationSummary>,
    control_state: Option<types::PathwayControlState>,
    last_round_progress: Option<types::PathwayRoundProgress>,
    last_checkpointed_topology_epoch: Option<RouteEpoch>,
    candidate_cache: RefCell<BTreeMap<jacquard_core::BackendRouteId, CachedCandidate>>,
    active_routes: BTreeMap<RouteId, ActivePathwayRoute>,
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector> RouterManagedEngine
    for PathwayEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: PathwayTopologyBounds,
    Topology::PeerEstimate: PathwayPeerEstimateAccess,
    Topology::NeighborhoodEstimate: PathwayNeighborhoodEstimateAccess,
    Transport: PathwayTransportBounds,
    Retention: PathwayRetentionBounds,
    Effects: PathwayEffectsBounds,
    Hasher: PathwayHasherBounds,
    Selector: PathwaySelectorBounds,
{
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id()
    }

    fn ingest_transport_observation_for_router(
        &mut self,
        observation: &TransportObservation,
    ) -> Result<(), RouteError> {
        if self.pending_transport_ingress.len()
            >= PATHWAY_PENDING_TRANSPORT_INGRESS_COUNT_MAX
        {
            self.dropped_transport_ingress_since_last_tick = self
                .dropped_transport_ingress_since_last_tick
                .saturating_add(1);
        } else {
            self.pending_transport_ingress
                .push_back(observation.clone());
        }
        Ok(())
    }

    fn forward_payload_for_router(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        self.forward_payload(route_id, payload)
    }

    fn restore_route_runtime_for_router(
        &mut self,
        route_id: &RouteId,
    ) -> Result<bool, RouteError> {
        Ok(self.restore_checkpointed_route(route_id)?.is_some())
    }
}

// Engine Construction

impl<Topology, Transport, Retention, Effects, Hasher>
    PathwayEngine<Topology, Transport, Retention, Effects, Hasher, NoCommitteeSelector>
{
    #[must_use]
    pub fn without_committee_selector(
        local_node_id: NodeId,
        topology_model: Topology,
        transport: Transport,
        retention: Retention,
        effects: Effects,
        hashing: Hasher,
    ) -> Self {
        Self {
            local_node_id,
            topology_model,
            transport,
            retention,
            effects,
            hashing,
            selector: NoCommitteeSelector,
            pending_transport_ingress: VecDeque::new(),
            dropped_transport_ingress_since_last_tick: 0,
            latest_topology: None,
            last_transport_summary: None,
            control_state: None,
            last_round_progress: None,
            last_checkpointed_topology_epoch: None,
            candidate_cache: RefCell::new(BTreeMap::new()),
            active_routes: BTreeMap::new(),
        }
    }
}

// Engine State Access

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    PathwayEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
{
    #[must_use]
    pub fn with_committee_selector(
        local_node_id: NodeId,
        topology_model: Topology,
        transport: Transport,
        retention: Retention,
        effects: Effects,
        hashing: Hasher,
        selector: Selector,
    ) -> Self {
        Self {
            local_node_id,
            topology_model,
            transport,
            retention,
            effects,
            hashing,
            selector,
            pending_transport_ingress: VecDeque::new(),
            dropped_transport_ingress_since_last_tick: 0,
            latest_topology: None,
            last_transport_summary: None,
            control_state: None,
            last_round_progress: None,
            last_checkpointed_topology_epoch: None,
            candidate_cache: RefCell::new(BTreeMap::new()),
            active_routes: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn local_node_id(&self) -> NodeId {
        self.local_node_id
    }

    #[must_use]
    pub fn latest_topology(&self) -> Option<&Observation<Configuration>> {
        self.latest_topology.as_ref()
    }

    #[must_use]
    pub fn last_round_progress(&self) -> Option<&types::PathwayRoundProgress> {
        self.last_round_progress.as_ref()
    }

    #[must_use]
    pub fn active_route(&self, route_id: &RouteId) -> Option<PathwayActiveRouteView> {
        self.active_routes
            .get(route_id)
            .map(types::PathwayActiveRouteView::from)
    }

    #[must_use]
    pub fn active_route_count(&self) -> usize {
        self.active_routes.len()
    }

    #[must_use]
    pub fn transport_observation_summary(
        &self,
    ) -> Option<&PathwayTransportObservationSummary> {
        self.last_transport_summary.as_ref()
    }

    #[must_use]
    pub fn control_state(&self) -> Option<&types::PathwayControlState> {
        self.control_state.as_ref()
    }

    pub fn checkpointed_topology_epoch(&self) -> Result<Option<RouteEpoch>, RouteError>
    where
        Effects: PathwayEffectsBounds,
    {
        let key = support::topology_epoch_storage_key(&self.local_node_id);
        let Some(bytes) = self.effects.load_bytes(&key).storage_invalid()? else {
            return Ok(None);
        };
        // Epoch is stored as raw LE bytes (not bincode) for cheap comparison.
        // A wrong length means storage corruption; fail closed.
        if bytes.len() != std::mem::size_of::<u64>() {
            return Err(RouteError::Runtime(RouteRuntimeError::Invalidated));
        }
        let mut epoch_bytes = [0_u8; 8];
        epoch_bytes.copy_from_slice(&bytes);
        Ok(Some(RouteEpoch(u64::from_le_bytes(epoch_bytes))))
    }

    pub(crate) fn restore_checkpointed_route(
        &mut self,
        route_id: &RouteId,
    ) -> Result<Option<ActivePathwayRoute>, RouteError>
    where
        Effects: PathwayEffectsBounds,
    {
        let key = support::route_storage_key(&self.local_node_id, route_id);
        let Some(bytes) = self.effects.load_bytes(&key).storage_invalid()? else {
            return Ok(None);
        };
        let active_route = support::decode_checkpoint_bytes(&bytes)
            .ok_or(RouteError::Runtime(RouteRuntimeError::Invalidated))?;
        self.active_routes.insert(*route_id, active_route.clone());
        Ok(Some(active_route))
    }
}

// Transport-Capability Helpers

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    PathwayEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Transport: PathwayTransportBounds,
    Retention: PathwayRetentionBounds,
    Effects: PathwayEffectsBounds,
    Hasher: PathwayHasherBounds,
{
    pub(crate) fn forward_payload(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        // Forwarding is owner-relative. The active route stores the
        // current owner and the index of the next segment from that
        // owner's point of view. Old owners fail closed after handoff,
        // and the next owner sees the remaining suffix. When the route
        // is in partition mode, forwarding becomes deferred delivery:
        // payloads are retained under the route instead of being sent.
        let active_route = self
            .active_routes
            .get(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        if active_route.forwarding.current_owner_node_id != self.local_node_id {
            return Err(RouteRuntimeError::StaleOwner.into());
        }
        let partition_mode = active_route.is_in_partition_mode();
        let next_segment = current_segment(active_route)
            .cloned()
            .ok_or(RouteRuntimeError::Invalidated)?;
        if partition_mode {
            self.retain_for_route(route_id, payload)
                .maintenance_failed()?;
            return Ok(());
        }

        choreography::forwarding_hop(
            &mut self.transport,
            &mut self.retention,
            &mut self.effects,
            route_id,
            next_segment.endpoint,
            payload,
        )?;
        // Re-borrow mutably after send: the earlier shared borrow for
        // `next_segment` must be fully released before taking &mut.
        let active_route = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        active_route.forwarding.in_flight_frames =
            active_route.forwarding.in_flight_frames.saturating_add(1);
        active_route.forwarding.last_ack_at_tick = Some(self.effects.now_tick());
        Ok(())
    }
}

// Retention-Facing Helpers

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    PathwayEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Transport: PathwayTransportBounds,
    Retention: PathwayRetentionBounds,
    Effects: PathwayEffectsBounds,
    Hasher: PathwayHasherBounds,
{
    pub(crate) fn retain_for_route(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<ContentId<Blake3Digest>, jacquard_core::RetentionError> {
        // Reject retention when the per-route object budget is exhausted.
        // This runs before the store write so we never leak bytes into
        // the retention store that the engine cannot track.
        if let Some(active_route) = self.active_routes.get(route_id) {
            if active_route.anti_entropy.retained_objects.len()
                >= PATHWAY_RETAINED_PER_ROUTE_COUNT_MAX
            {
                return Err(jacquard_core::RetentionError::Full);
            }
        }
        let object_id = self.retention_object_id(route_id, payload);
        choreography::retain_for_replay(
            &mut self.transport,
            &mut self.retention,
            &mut self.effects,
            route_id,
            object_id,
            payload,
        )?;
        if let Some(active_route) = self.active_routes.get_mut(route_id) {
            active_route.anti_entropy.retained_objects.insert(object_id);
        }
        Ok(object_id)
    }
}

// Hash-Derived Pathway Identifiers

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    PathwayEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Hasher: PathwayHasherBounds,
{
    // The four ID helpers below all derive stable content addresses from
    // a route-specific byte input using tagged hashing. Domain tags
    // ("mesh-route-id", "mesh-commitment", "mesh-handoff-receipt",
    // "mesh-retention") prevent cross-domain collisions even when two
    // derivations happen to share bytes.
    fn route_id_for_backend(
        &self,
        backend_route_id: &jacquard_core::BackendRouteId,
    ) -> Result<RouteId, RouteError> {
        let plan = support::decode_backend_token(backend_route_id)
            .ok_or(RouteError::Runtime(RouteRuntimeError::Invalidated))?;
        let route_key_bytes = support::encode_route_identity_bytes(&plan);
        let digest = self
            .hashing
            .hash_tagged(support::DOMAIN_TAG_ROUTE_ID, &route_key_bytes);
        Ok(RouteId(support::digest_prefix::<16>(digest.as_bytes())))
    }

    fn commitment_id_for_route(&self, route_id: &RouteId) -> RouteCommitmentId {
        let digest = self
            .hashing
            .hash_tagged(support::DOMAIN_TAG_COMMITMENT, &route_id.0);
        RouteCommitmentId(support::digest_prefix::<16>(digest.as_bytes()))
    }

    fn receipt_id_for_route(&self, route_id: &RouteId) -> ReceiptId {
        let digest = self
            .hashing
            .hash_tagged(support::DOMAIN_TAG_HANDOFF_RECEIPT, &route_id.0);
        ReceiptId(support::digest_prefix::<16>(digest.as_bytes()))
    }

    fn retention_object_id(
        &self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> ContentId<Blake3Digest> {
        let mut tagged = route_id.0.to_vec();
        tagged.extend_from_slice(payload);
        let digest = self
            .hashing
            .hash_tagged(support::DOMAIN_TAG_RETENTION, &tagged);
        // ContentId requires a concrete Blake3Digest. Hash the pluggable
        // digest bytes through Blake3Hashing to get the required concrete
        // type while still binding the result to the retention domain tag.
        let content_digest =
            Blake3Hashing.hash_tagged(support::DOMAIN_TAG_RETENTION, digest.as_bytes());
        ContentId { digest: content_digest }
    }
}

// Checkpointing, Event Recording, And Route Lookup

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    PathwayEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Transport: PathwayTransportBounds,
    Retention: PathwayRetentionBounds,
    Effects: PathwayEffectsBounds,
{
    fn store_checkpoint(
        &mut self,
        active_route: &ActivePathwayRoute,
    ) -> Result<(), RouteError> {
        let key = support::route_storage_key(
            &self.local_node_id,
            &active_route.path.route_id,
        );
        let value = support::checkpoint_bytes(active_route);
        self.effects.store_bytes(&key, &value).storage_invalid()
    }

    /// Store a checkpoint for `active_route`, intentionally discarding any
    /// storage error. Use only in rollback paths where the route is going
    /// away regardless and storage hygiene is best-effort.
    pub(super) fn checkpoint_best_effort(&mut self, active_route: &ActivePathwayRoute) {
        let _ = self.store_checkpoint(active_route);
    }

    fn remove_checkpoint(&mut self, route_id: &RouteId) -> Result<(), RouteError> {
        self.effects
            .remove_bytes(&support::route_storage_key(&self.local_node_id, route_id))
            .storage_invalid()
    }

    // Handoff target is the next owner in the owner-relative path view.
    // Once the cursor reaches the end of the path, no further handoff
    // is valid and the caller must fail closed.
    fn handoff_target(active_route: &ActivePathwayRoute) -> Option<NodeId> {
        active_route
            .path
            .segments
            .get(usize::from(active_route.forwarding.next_hop_index))
            .map(|segment| segment.node_id)
    }
}
