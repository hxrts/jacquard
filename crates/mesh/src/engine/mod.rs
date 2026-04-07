//! `MeshEngine` implementation of the shared routing engine contract.
//!
//! The engine is generic over topology, transport, retention, runtime
//! effects, hashing, and committee selector so the same state machine runs
//! under production adapters, tests, and simulation. Planning is a pure
//! read against explicit topology observations. Runtime mutation is
//! confined to `materialize_route`, `maintain_route`, and `teardown`.
//! Canonical route identity, handles, and leases flow in from the router;
//! this crate never invents them.

mod planner;
mod runtime;
mod support;

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

use jacquard_core::{
    Blake3Digest, CommitteeSelection, Configuration, ContentId, DestinationId,
    DeterministicOrderKey, LinkEndpoint, NodeId, Observation, ReceiptId, RouteCommitmentId,
    RouteConnectivityProfile, RouteCost, RouteError, RouteEvent, RouteEventStamped, RouteId,
    RouteLifecycleEvent, RoutePartitionClass, RouteRuntimeError, RouteSelectionError, RouteSummary,
    RoutingEngineCapabilities, RoutingEngineId, Tick, TimeWindow, TransportObservation,
};
use jacquard_traits::{
    CommitteeSelector, Hashing, MeshTransport, OrderEffects, RetentionStore, RouteEventLogEffects,
    StorageEffects, TimeEffects, TransportEffects,
};

use crate::committee::NoCommitteeSelector;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeshRouteClass {
    Direct,
    MultiHop,
    Gateway,
    DeferredDelivery,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshRouteSegment {
    pub node_id: NodeId,
    pub endpoint: LinkEndpoint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshPath {
    pub route_id: RouteId,
    pub epoch: jacquard_core::RouteEpoch,
    pub source: NodeId,
    pub destination: DestinationId,
    pub segments: Vec<MeshRouteSegment>,
    pub valid_for: TimeWindow,
    pub route_class: MeshRouteClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveMeshRoute {
    pub path: MeshPath,
    pub committee: Option<CommitteeSelection>,
    pub current_epoch: jacquard_core::RouteEpoch,
    pub last_lifecycle_event: RouteLifecycleEvent,
    pub in_flight_frames: u32,
    pub last_ack_at_tick: Option<Tick>,
    pub repair_steps_remaining: u32,
    pub route_cost: RouteCost,
    pub partition_mode: bool,
    pub retained_objects: BTreeSet<ContentId<Blake3Digest>>,
    pub ordering_key: DeterministicOrderKey<RouteId>,
}

#[derive(Clone, Debug)]
struct CachedCandidate {
    route_id: RouteId,
    summary: RouteSummary,
    estimate: jacquard_core::Estimate<jacquard_core::RouteEstimate>,
    admission_check: jacquard_core::RouteAdmissionCheck,
    witness: jacquard_core::RouteWitness,
    path: MeshPath,
    committee: Option<CommitteeSelection>,
    route_cost: RouteCost,
    ordering_key: DeterministicOrderKey<RouteId>,
}

// `candidate_cache` memoizes planning work so `check_candidate` and
// `admit_route` can reuse the admission check, witness, and path derived
// during `candidate_routes` without reconstructing them. The cache is an
// optimization only: `BackendRouteRef` is a self-contained opaque plan
// token, so mesh can re-derive candidate state from an explicit topology
// observation on cache miss. It is `RefCell<...>` because the planner
// trait methods take `&self`.
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
    candidate_cache: RefCell<BTreeMap<jacquard_core::BackendRouteId, CachedCandidate>>,
    active_routes: BTreeMap<RouteId, ActiveMeshRoute>,
}

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
            candidate_cache: RefCell::new(BTreeMap::new()),
            active_routes: BTreeMap::new(),
        }
    }
}

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
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: jacquard_traits::MeshTopologyModel<
        PeerEstimate = crate::topology::MeshPeerEstimate,
        NeighborhoodEstimate = crate::topology::MeshNeighborhoodEstimate,
    >,
    Transport: MeshTransport + Send + Sync + 'static,
    Retention: RetentionStore,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
    Hasher: Hashing<Digest = Blake3Digest>,
    Selector: CommitteeSelector<TopologyView = Configuration>,
{
    pub fn forward_payload(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        let active_route = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        let first_segment = active_route
            .path
            .segments
            .first()
            .ok_or(RouteSelectionError::NoCandidate)?;
        self.transport
            .send_transport(&first_segment.endpoint, payload)?;
        active_route.in_flight_frames = active_route.in_flight_frames.saturating_add(1);
        active_route.last_ack_at_tick = Some(self.effects.now_tick());
        Ok(())
    }

    pub fn poll_transport_observations(&mut self) -> Result<Vec<TransportObservation>, RouteError> {
        self.transport.poll_transport().map_err(RouteError::from)
    }

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

    fn candidate_plan_token(&self, node_path: &[NodeId]) -> jacquard_core::BackendRouteId {
        support::encode_backend_token(node_path)
    }

    fn route_id_for_backend(&self, backend_route_id: &jacquard_core::BackendRouteId) -> RouteId {
        let digest = self
            .hashing
            .hash_tagged(b"mesh-route-id", &backend_route_id.0);
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

    fn find_cached_candidate_by_route_id(&self, route_id: &RouteId) -> Option<CachedCandidate> {
        self.candidate_cache
            .borrow()
            .values()
            .find(|candidate| &candidate.route_id == route_id)
            .cloned()
    }

    fn store_checkpoint(&mut self, active_route: &ActiveMeshRoute) -> Result<(), RouteError> {
        let key = support::route_storage_key(&active_route.path.route_id);
        let value = support::checkpoint_bytes(active_route);
        self.effects
            .store_bytes(&key, &value)
            .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))
    }

    fn remove_checkpoint(&mut self, route_id: &RouteId) -> Result<(), RouteError> {
        self.effects
            .remove_bytes(&support::route_storage_key(route_id))
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

    fn handoff_target(active_route: &ActiveMeshRoute, owner_node_id: NodeId) -> NodeId {
        active_route
            .path
            .segments
            .first()
            .map_or(owner_node_id, |segment| segment.node_id)
    }
}
