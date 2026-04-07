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
    Blake3Digest, Configuration, ContentId, NodeId, Observation, ReceiptId, RouteCommitmentId,
    RouteConnectivityProfile, RouteEpoch, RouteError, RouteEvent, RouteEventStamped, RouteId,
    RoutePartitionClass, RouteRuntimeError, RouteSelectionError, RoutingEngineCapabilities,
    RoutingEngineId, TransportObservation,
};

use crate::committee::NoCommitteeSelector;

use trait_bounds::{
    MeshEffectsBounds, MeshHasherBounds, MeshRetentionBounds, MeshSelectorBounds,
    MeshTopologyBounds, MeshTransportBounds,
};
use types::{ActiveMeshRoute, CachedCandidate};

pub use types::{MeshPath, MeshRouteClass, MeshRouteSegment, MeshTransportObservationSummary};

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

// Route-commitment retry budget. Mesh uses a short, fixed policy because
// commitments represent already-admitted routes; exceeding the budget is
// a teardown signal rather than a reason to keep retrying.
const MESH_COMMITMENT_ATTEMPT_COUNT_MAX: u32 = 2;
const MESH_COMMITMENT_INITIAL_BACKOFF_MS: u32 = 25;
const MESH_COMMITMENT_BACKOFF_MS_MAX: u32 = 25;
const MESH_COMMITMENT_OVERALL_TIMEOUT_MS: u32 = 50;

// Mesh advertises link-level protection, explicit route shape, and full
// repair, hold, and decidable-admission support. This is the static
// capability envelope the router sees during engine registration.
pub const MESH_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: RoutingEngineId::Mesh,
    max_protection: jacquard_core::RouteProtectionClass::LinkProtected,
    max_connectivity: RouteConnectivityProfile {
        repair: jacquard_core::RouteRepairClass::Repairable,
        partition: RoutePartitionClass::PartitionTolerant,
    },
    repair_support: jacquard_core::RepairSupport::Supported,
    hold_support: jacquard_core::HoldSupport::Supported,
    decidable_admission: jacquard_core::DecidableSupport::Supported,
    quantitative_bounds: jacquard_core::QuantitativeBoundSupport::ProductiveAndSchedulerLifted,
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
    selector: Option<Selector>,
    latest_topology: Option<Observation<Configuration>>,
    last_transport_summary: Option<MeshTransportObservationSummary>,
    candidate_cache: RefCell<BTreeMap<jacquard_core::BackendRouteId, CachedCandidate>>,
    active_routes: BTreeMap<RouteId, ActiveMeshRoute>,
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
            selector: Some(NoCommitteeSelector),
            latest_topology: None,
            last_transport_summary: None,
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
            selector: Some(selector),
            latest_topology: None,
            last_transport_summary: None,
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
    pub fn transport_observation_summary(&self) -> Option<&MeshTransportObservationSummary> {
        self.last_transport_summary.as_ref()
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

// Transport-Facing Helpers

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Transport: MeshTransportBounds,
    Effects: MeshEffectsBounds,
{
    pub fn forward_payload(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        // Forwarding is owner-relative. The active route stores the
        // current owner and the index of the next segment from that
        // owner's point of view. Old owners fail closed after handoff,
        // and the next owner sees the remaining suffix.
        let active_route = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        if active_route.current_owner_node_id != self.local_node_id {
            return Err(RouteRuntimeError::StaleOwner.into());
        }
        let next_segment = active_route
            .path
            .segments
            .get(usize::from(active_route.next_hop_index))
            .ok_or(RouteRuntimeError::Invalidated)?;
        self.transport
            .send_transport(&next_segment.endpoint, payload)?;
        active_route.in_flight_frames = active_route.in_flight_frames.saturating_add(1);
        active_route.last_ack_at_tick = Some(self.effects.now_tick());
        Ok(())
    }

    pub fn poll_transport_observations(&mut self) -> Result<Vec<TransportObservation>, RouteError> {
        self.transport.poll_transport().map_err(RouteError::from)
    }
}

// Retention-Facing Helpers

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Retention: MeshRetentionBounds,
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
            if active_route.retained_objects.len() >= MESH_RETAINED_PER_ROUTE_COUNT_MAX {
                return Err(jacquard_core::RetentionError::Full);
            }
        }
        let object_id = self.retention_object_id(route_id, payload);
        self.retention.retain_payload(object_id, payload.to_vec())?;
        if let Some(active_route) = self.active_routes.get_mut(route_id) {
            active_route.retained_objects.insert(object_id);
        }
        Ok(object_id)
    }

    pub fn recover_retained_payload(
        &mut self,
        route_id: &RouteId,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, jacquard_core::RetentionError> {
        let payload = self.retention.take_retained_payload(object_id)?;
        if payload.is_some() {
            if let Some(active_route) = self.active_routes.get_mut(route_id) {
                active_route.retained_objects.remove(object_id);
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
    fn route_id_for_backend(&self, backend_route_id: &jacquard_core::BackendRouteId) -> RouteId {
        let route_key_bytes = support::decode_backend_token(backend_route_id)
            .map(|plan| support::encode_route_identity_bytes(&plan))
            .unwrap_or_else(|| backend_route_id.0.clone());
        let digest = self.hashing.hash_tagged(b"mesh-route-id", &route_key_bytes);
        let mut route_id = [0_u8; 16];
        route_id.copy_from_slice(&digest.0[..16]);
        RouteId(route_id)
    }

    fn commitment_id_for_route(&self, route_id: &RouteId) -> RouteCommitmentId {
        let digest = self.hashing.hash_tagged(b"mesh-commitment", &route_id.0);
        let mut commitment_id = [0_u8; 16];
        commitment_id.copy_from_slice(&digest.0[..16]);
        RouteCommitmentId(commitment_id)
    }

    fn receipt_id_for_route(&self, route_id: &RouteId) -> ReceiptId {
        let digest = self
            .hashing
            .hash_tagged(b"mesh-handoff-receipt", &route_id.0);
        let mut receipt_id = [0_u8; 16];
        receipt_id.copy_from_slice(&digest.0[..16]);
        ReceiptId(receipt_id)
    }

    fn retention_object_id(&self, route_id: &RouteId, payload: &[u8]) -> ContentId<Blake3Digest> {
        let mut tagged = route_id.0.to_vec();
        tagged.extend_from_slice(payload);
        ContentId {
            digest: self.hashing.hash_tagged(b"mesh-retention", &tagged),
        }
    }
}

// Checkpointing, Event Recording, And Route Lookup

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Effects: MeshEffectsBounds,
{
    fn store_checkpoint(&mut self, active_route: &ActiveMeshRoute) -> Result<(), RouteError> {
        let key = support::route_storage_key(&self.local_node_id, &active_route.path.route_id);
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

    fn record_event(&mut self, event: RouteEvent) -> Result<(), RouteError> {
        let stamped = RouteEventStamped {
            order_stamp: self.effects.next_order_stamp(),
            emitted_at_tick: self.effects.now_tick(),
            event,
        };
        self.effects
            .record_route_event(stamped)
            .map_err(|_| RouteError::Runtime(RouteRuntimeError::MaintenanceFailed))
    }

    // Handoff target is the next owner in the owner-relative path view.
    // Once the cursor reaches the end of the path, no further handoff
    // is valid and the caller must fail closed.
    fn handoff_target(active_route: &ActiveMeshRoute) -> Option<NodeId> {
        active_route
            .path
            .segments
            .get(usize::from(active_route.next_hop_index))
            .map(|segment| segment.node_id)
    }
}
