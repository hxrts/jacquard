//! `MeshEngine` implementation of the shared routing engine contract.
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
//! `runtime` implements `RoutingEngine`/`MeshRoutingEngine`, `support`
//! holds the pure helpers shared between them, `types` defines the
//! mesh-private object model, and `trait_bounds` defines the internal
//! bound aliases used in `impl` headers.

#![allow(private_bounds)]

mod planner;
mod runtime;
mod support;
mod trait_bounds;
mod types;

use std::{cell::RefCell, collections::BTreeMap};

use jacquard_core::{
    Blake3Digest, Configuration, ContentId, NodeId, Observation, ReceiptId,
    RouteCommitmentId, ConnectivityPosture, RouteEpoch, RouteError, RouteId,
    RoutePartitionClass, RouteRuntimeError, RouteSelectionError,
    RoutingEngineCapabilities, RoutingEngineId,
};
use jacquard_traits::{Blake3Hashing, HashDigestBytes, Hashing, RouterManagedEngine};
pub(crate) use support::DOMAIN_TAG_COMMITTEE_ID;
use trait_bounds::{
    MeshEffectsBounds, MeshHasherBounds, MeshRetentionBounds, MeshSelectorBounds,
    MeshTopologyBounds, MeshTransportBounds,
};
use types::{ActiveMeshRoute, CachedCandidate};
pub use types::{
    MeshCommitteeStatus, MeshControlState, MeshForwardingState, MeshHandoffState,
    MeshObservedRemoteLink, MeshPath, MeshRepairState, MeshRouteAntiEntropyState,
    MeshRouteClass, MeshRouteSegment, MeshTransportFreshness,
    MeshTransportObservationSummary,
};

use crate::{
    choreography::{MeshGuestRuntime, MeshProtocolRuntimeAdapter},
    committee::NoCommitteeSelector,
};

// Public Engine Identity And Capability Surface

pub const MESH_ENGINE_ID: RoutingEngineId = RoutingEngineId::Mesh;

/// Maximum number of concurrently active materialized routes this engine
/// will hold. New materializations past this point fail with
/// `RoutePolicyError::BudgetExceeded`.
pub const MESH_ACTIVE_ROUTE_COUNT_MAX: usize = 64;

/// Maximum number of candidate entries the planner emits per tick.
/// Sorting and truncation happen after BFS so the cap is deterministic.
pub const MESH_CANDIDATE_COUNT_MAX: usize = 32;

/// Maximum number of retained payload objects tracked per active route.
pub const MESH_RETAINED_PER_ROUTE_COUNT_MAX: usize = 32;

/// Validity window applied to newly derived mesh candidates, in ticks.
pub const MESH_CANDIDATE_VALIDITY_TICKS: u64 = 12;

/// Per-hop byte cost used in `RouteCost` derivation.
pub const MESH_PER_HOP_BYTE_COST: u64 = 1024;

/// Hold capacity reserved for deferred-delivery routes, in bytes.
pub const MESH_HOLD_RESERVED_BYTES: u64 = 1024;

/// Maximum canonical byte length for a v1 mesh backend plan token.
pub const MESH_BACKEND_ROUTE_ID_BYTES_MAX: usize = 2048;

// Route-commitment retry budget. Mesh uses a short, fixed policy because
// commitments represent already-admitted routes; exceeding the budget is
// a teardown signal rather than a reason to keep retrying.
const MESH_COMMITMENT_ATTEMPT_COUNT_MAX: u32 = 2;
// Fixed-delay by design: commitments cover already-admitted routes, so
// the retry ceiling intentionally equals the initial backoff.
const MESH_COMMITMENT_INITIAL_BACKOFF_MS: u32 = 25;
const MESH_COMMITMENT_BACKOFF_MS_MAX: u32 = 25;
const MESH_COMMITMENT_OVERALL_TIMEOUT_MS: u32 = 50;

// Mesh advertises link-level protection, explicit route shape, and full
// repair, hold, and decidable-admission support. This is the static
// capability envelope the router sees during engine registration.
pub const MESH_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: RoutingEngineId::Mesh,
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
    route_shape_visibility: jacquard_core::RouteShapeVisibility::Explicit,
};

// `candidate_cache` memoizes planning work so `check_candidate` and
// `admit_route` can reuse the admission check and witness derived during
// `candidate_routes` without recomputing them. The cache is an
// optimization only: `BackendRouteRef` is a self-contained opaque plan
// token, so planner cache misses and materialization must still work.
// It is `RefCell<...>` because the planner trait methods take `&self`.
// `active_routes` holds the mesh-private runtime state for each
// materialized route. Canonical identity lives on the router side.
pub struct MeshEngine<
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
    latest_topology: Option<Observation<Configuration>>,
    last_transport_summary: Option<MeshTransportObservationSummary>,
    control_state: Option<types::MeshControlState>,
    last_checkpointed_topology_epoch: Option<RouteEpoch>,
    candidate_cache: RefCell<BTreeMap<jacquard_core::BackendRouteId, CachedCandidate>>,
    active_routes: BTreeMap<RouteId, ActiveMeshRoute>,
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector> RouterManagedEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: MeshTopologyBounds,
    Topology::PeerEstimate: jacquard_traits::MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: jacquard_traits::MeshNeighborhoodEstimateAccess,
    Transport: MeshTransportBounds,
    Retention: MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id()
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
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, NoCommitteeSelector>
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
            latest_topology: None,
            last_transport_summary: None,
            control_state: None,
            last_checkpointed_topology_epoch: None,
            candidate_cache: RefCell::new(BTreeMap::new()),
            active_routes: BTreeMap::new(),
        }
    }
}

// Engine State Access

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
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
            latest_topology: None,
            last_transport_summary: None,
            control_state: None,
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
    pub fn runtime_effects(&self) -> &Effects {
        &self.effects
    }

    pub fn runtime_effects_mut(&mut self) -> &mut Effects {
        &mut self.effects
    }

    #[must_use]
    pub fn latest_topology(&self) -> Option<&Observation<Configuration>> {
        self.latest_topology.as_ref()
    }

    #[must_use]
    pub fn active_route(&self, route_id: &RouteId) -> Option<&ActiveMeshRoute> {
        self.active_routes.get(route_id)
    }

    #[must_use]
    pub fn active_route_count(&self) -> usize {
        self.active_routes.len()
    }

    #[must_use]
    pub fn transport_observation_summary(
        &self,
    ) -> Option<&MeshTransportObservationSummary> {
        self.last_transport_summary.as_ref()
    }

    #[must_use]
    pub fn control_state(&self) -> Option<&types::MeshControlState> {
        self.control_state.as_ref()
    }

    pub fn checkpointed_topology_epoch(&self) -> Result<Option<RouteEpoch>, RouteError>
    where
        Effects: MeshEffectsBounds,
    {
        let key = support::topology_epoch_storage_key(&self.local_node_id);
        let Some(bytes) = self
            .effects
            .load_bytes(&key)
            .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))?
        else {
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

    pub fn restore_checkpointed_route(
        &mut self,
        route_id: &RouteId,
    ) -> Result<Option<ActiveMeshRoute>, RouteError>
    where
        Effects: MeshEffectsBounds,
    {
        let key = support::route_storage_key(&self.local_node_id, route_id);
        let Some(bytes) = self
            .effects
            .load_bytes(&key)
            .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))?
        else {
            return Ok(None);
        };
        let active_route = support::decode_checkpoint_bytes(&bytes)
            .ok_or(RouteError::Runtime(RouteRuntimeError::Invalidated))?;
        self.active_routes.insert(*route_id, active_route.clone());
        Ok(Some(active_route))
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Transport: MeshTransportBounds,
    Retention: MeshRetentionBounds,
    Effects: MeshEffectsBounds,
{
    fn choreography_runtime(
        &mut self,
    ) -> MeshGuestRuntime<MeshProtocolRuntimeAdapter<'_, Transport, Retention, Effects>>
    {
        MeshGuestRuntime::new(MeshProtocolRuntimeAdapter {
            transport: &mut self.transport,
            retention: &mut self.retention,
            effects: &mut self.effects,
        })
    }
}

// Transport-Facing Helpers

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Transport: MeshTransportBounds,
    Retention: MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
{
    pub fn forward_payload(
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
        let partition_mode = active_route.anti_entropy.partition_mode;
        let next_segment = active_route
            .path
            .segments
            .get(usize::from(active_route.forwarding.next_hop_index))
            .cloned()
            .ok_or(RouteRuntimeError::Invalidated)?;
        if partition_mode {
            self.retain_for_route(route_id, payload).map_err(|_| {
                RouteError::Runtime(RouteRuntimeError::MaintenanceFailed)
            })?;
            return Ok(());
        }

        self.choreography_runtime().forwarding_hop(
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
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Transport: MeshTransportBounds,
    Retention: MeshRetentionBounds,
    Effects: MeshEffectsBounds,
    Hasher: MeshHasherBounds,
{
    pub fn retain_for_route(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<ContentId<Blake3Digest>, jacquard_core::RetentionError> {
        // Reject retention when the per-route object budget is exhausted.
        // This runs before the store write so we never leak bytes into
        // the retention store that the engine cannot track.
        if let Some(active_route) = self.active_routes.get(route_id) {
            if active_route.anti_entropy.retained_objects.len()
                >= MESH_RETAINED_PER_ROUTE_COUNT_MAX
            {
                return Err(jacquard_core::RetentionError::Full);
            }
        }
        let object_id = self.retention_object_id(route_id, payload);
        self.choreography_runtime()
            .retain_for_replay(route_id, object_id, payload)?;
        if let Some(active_route) = self.active_routes.get_mut(route_id) {
            active_route.anti_entropy.retained_objects.insert(object_id);
        }
        Ok(object_id)
    }

    pub fn recover_retained_payload(
        &mut self,
        route_id: &RouteId,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, jacquard_core::RetentionError> {
        let payload = self
            .choreography_runtime()
            .recover_held_payload(route_id, object_id)?;
        if payload.is_some() {
            if let Some(active_route) = self.active_routes.get_mut(route_id) {
                active_route.anti_entropy.retained_objects.remove(object_id);
            }
        }
        Ok(payload)
    }
}

// Hash-Derived Mesh Identifiers

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Hasher: MeshHasherBounds,
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
        let mut route_id = [0_u8; 16];
        route_id.copy_from_slice(&digest.as_bytes()[..16]);
        Ok(RouteId(route_id))
    }

    fn commitment_id_for_route(&self, route_id: &RouteId) -> RouteCommitmentId {
        let digest = self
            .hashing
            .hash_tagged(support::DOMAIN_TAG_COMMITMENT, &route_id.0);
        let mut commitment_id = [0_u8; 16];
        commitment_id.copy_from_slice(&digest.as_bytes()[..16]);
        RouteCommitmentId(commitment_id)
    }

    fn receipt_id_for_route(&self, route_id: &RouteId) -> ReceiptId {
        let digest = self
            .hashing
            .hash_tagged(support::DOMAIN_TAG_HANDOFF_RECEIPT, &route_id.0);
        let mut receipt_id = [0_u8; 16];
        receipt_id.copy_from_slice(&digest.as_bytes()[..16]);
        ReceiptId(receipt_id)
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
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Transport: MeshTransportBounds,
    Retention: MeshRetentionBounds,
    Effects: MeshEffectsBounds,
{
    fn store_checkpoint(
        &mut self,
        active_route: &ActiveMeshRoute,
    ) -> Result<(), RouteError> {
        let key = support::route_storage_key(
            &self.local_node_id,
            &active_route.path.route_id,
        );
        let value = support::checkpoint_bytes(active_route);
        self.effects
            .store_bytes(&key, &value)
            .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))
    }

    fn remove_checkpoint(&mut self, route_id: &RouteId) -> Result<(), RouteError> {
        self.effects
            .remove_bytes(&support::route_storage_key(&self.local_node_id, route_id))
            .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))
    }

    // Handoff target is the next owner in the owner-relative path view.
    // Once the cursor reaches the end of the path, no further handoff
    // is valid and the caller must fail closed.
    fn handoff_target(active_route: &ActiveMeshRoute) -> Option<NodeId> {
        active_route
            .path
            .segments
            .get(usize::from(active_route.forwarding.next_hop_index))
            .map(|segment| segment.node_id)
    }
}
