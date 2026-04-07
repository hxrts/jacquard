//! `MeshEngine` implementation of the shared routing engine contract.
//!
//! The engine is generic over topology, transport, retention, runtime
//! effects, hashing, and committee selector so the same state machine runs
//! under production adapters, tests, and simulation. Planning is a pure
//! read against the latest topology observation cached through
//! `engine_tick`. Runtime mutation is confined to `materialize_route`,
//! `maintain_route`, and `teardown`. Canonical route identity, handles,
//! and leases flow in from the router; this crate never invents them.

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
};

use jacquard_core::{
    AdaptiveRoutingProfile, AdmissionAssumptions, AdmissionDecision, BackendRouteId, Belief,
    Blake3Digest, ByteCount, CommitteeSelection, Configuration, ContentId, DegradationReason,
    DestinationId, DeterministicOrderKey, Estimate, Fact, FactBasis, HoldFallbackPolicy, Limit,
    LinkEndpoint, MaterializedRoute, MaterializedRouteIdentity, NodeId, Observation, OrderStamp,
    PenaltyPoints, ReachabilityState, ReceiptId, RouteAdmission, RouteAdmissionCheck,
    RouteAdmissionRejection, RouteBinding, RouteCandidate, RouteCommitment, RouteCommitmentId,
    RouteCommitmentResolution, RouteConnectivityProfile, RouteCost, RouteDegradation, RouteError,
    RouteEstimate, RouteEvent, RouteEventStamped, RouteHealth, RouteId, RouteInstallation,
    RouteInvalidationReason, RouteLifecycleEvent, RouteMaintenanceFailure, RouteMaintenanceOutcome,
    RouteMaintenanceResult, RouteMaintenanceTrigger, RouteMaterializationInput,
    RouteMaterializationProof, RouteOperationId, RoutePartitionClass, RouteProgressContract,
    RouteProgressState, RouteProtectionClass, RouteRepairClass, RouteRuntimeError,
    RouteSelectionError, RouteSemanticHandoff, RouteServiceKind, RouteSummary, RouteWitness,
    RoutingEngineCapabilities, RoutingEngineId, RoutingObjective, Tick, TimeWindow, TimeoutPolicy,
    TransportObservation,
};
use jacquard_traits::{
    CommitteeCoordinatedEngine, CommitteeSelector, Hashing, MeshRoutingEngine, MeshTopologyModel,
    MeshTransport, OrderEffects, RetentionStore, RouteEventLogEffects, RoutingEngine,
    RoutingEnginePlanner, StorageEffects, TimeEffects, TransportEffects,
};

use crate::{
    committee::{mesh_admission_assumptions, mesh_health_score, NoCommitteeSelector},
    topology::{
        adjacent_link_between, adjacent_node_ids, estimate_hop_link, objective_matches_node,
        route_capable_for_engine, MeshNeighborhoodEstimate, MeshPeerEstimate,
    },
};

pub const MESH_ENGINE_ID: RoutingEngineId = RoutingEngineId::Mesh;

// Mesh advertises link-level protection, explicit route shape, and full
// repair, hold, and decidable-admission support. This is the static
// capability envelope the router sees during engine registration.
pub const MESH_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: RoutingEngineId::Mesh,
    max_protection: RouteProtectionClass::LinkProtected,
    max_connectivity: RouteConnectivityProfile {
        repair: RouteRepairClass::Repairable,
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
    estimate: Estimate<RouteEstimate>,
    admission_check: RouteAdmissionCheck,
    witness: RouteWitness,
    path: MeshPath,
    committee: Option<CommitteeSelection>,
    route_cost: RouteCost,
    ordering_key: DeterministicOrderKey<RouteId>,
}

// `candidate_cache` memoizes planning work so `check_candidate` and
// `admit_route` can reuse the admission check, witness, and path derived
// during `candidate_routes` without reconstructing them. It is
// `RefCell<...>` because the planner trait methods take `&self`.
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
    candidate_cache: RefCell<BTreeMap<BackendRouteId, CachedCandidate>>,
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
    Topology: MeshTopologyModel<
        PeerEstimate = MeshPeerEstimate,
        NeighborhoodEstimate = MeshNeighborhoodEstimate,
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

    fn candidate_cache_key(&self, path_bytes: &[u8]) -> BackendRouteId {
        BackendRouteId(
            self.hashing
                .hash_tagged(b"mesh-backend-route", path_bytes)
                .0
                .to_vec(),
        )
    }

    fn route_id_for_backend(&self, backend_route_id: &BackendRouteId) -> RouteId {
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

    fn determine_route_class(
        &self,
        objective: &RoutingObjective,
        hop_count: usize,
        hold_capable: bool,
    ) -> MeshRouteClass {
        if matches!(objective.destination, DestinationId::Gateway(_)) {
            MeshRouteClass::Gateway
        } else if hold_capable
            && objective.hold_fallback_policy == HoldFallbackPolicy::Allowed
            && hop_count > 1
        {
            MeshRouteClass::DeferredDelivery
        } else if hop_count <= 1 {
            MeshRouteClass::Direct
        } else {
            MeshRouteClass::MultiHop
        }
    }

    fn route_connectivity_for_class(
        &self,
        objective: &RoutingObjective,
        route_class: &MeshRouteClass,
    ) -> RouteConnectivityProfile {
        match route_class {
            MeshRouteClass::DeferredDelivery => RouteConnectivityProfile {
                repair: RouteRepairClass::Repairable,
                partition: if objective.hold_fallback_policy == HoldFallbackPolicy::Allowed {
                    RoutePartitionClass::PartitionTolerant
                } else {
                    RoutePartitionClass::ConnectedOnly
                },
            },
            _ => RouteConnectivityProfile {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
        }
    }

    fn derive_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
        destination_node_id: NodeId,
        node_path: &[NodeId],
    ) -> Option<(BackendRouteId, CachedCandidate)> {
        let configuration = &topology.value;
        let destination_node = configuration.nodes.get(&destination_node_id)?;
        let segments = node_path
            .windows(2)
            .filter_map(|pair| {
                estimate_hop_link(&pair[0], &pair[1], configuration).map(|(endpoint, _)| {
                    MeshRouteSegment {
                        node_id: pair[1],
                        endpoint,
                    }
                })
            })
            .collect::<Vec<_>>();
        if segments.is_empty() {
            return None;
        }

        let hold_capable = destination_node.profile.services.iter().any(|service| {
            service.service_kind == RouteServiceKind::Hold
                && service.routing_engines.contains(&MESH_ENGINE_ID)
                && service.valid_for.contains(topology.observed_at_tick)
        });
        let route_class = self.determine_route_class(objective, segments.len(), hold_capable);
        let connectivity = self.route_connectivity_for_class(objective, &route_class);
        let valid_for = TimeWindow::new(
            topology.observed_at_tick,
            Tick(topology.observed_at_tick.0 + 12),
        )
        .expect("mesh candidates always use a positive validity window");
        let protocol_mix = unique_protocol_mix(&segments);
        let path_bytes = encode_path_bytes(node_path, &segments);
        let backend_route_id = self.candidate_cache_key(&path_bytes);
        let route_id = self.route_id_for_backend(&backend_route_id);
        let order_key = deterministic_order_key(route_id, &self.hashing, &path_bytes);
        let route_cost = route_cost_for_segments(&segments, &route_class);
        let degradation = degradation_for_candidate(configuration, &route_class);
        let estimate = Estimate {
            value: RouteEstimate {
                estimated_protection: RouteProtectionClass::LinkProtected,
                estimated_connectivity: connectivity,
                topology_epoch: configuration.epoch,
                degradation,
            },
            confidence_permille: confidence_for_segments(&segments, configuration),
            updated_at_tick: topology.observed_at_tick,
        };
        let summary = RouteSummary {
            engine: MESH_ENGINE_ID,
            protection: RouteProtectionClass::LinkProtected,
            connectivity,
            protocol_mix,
            hop_count_hint: Belief::Estimated(Estimate {
                value: u8::try_from(segments.len()).unwrap_or(u8::MAX),
                confidence_permille: jacquard_core::RatioPermille(1000),
                updated_at_tick: topology.observed_at_tick,
            }),
            valid_for,
        };
        let admission_assumptions = mesh_admission_assumptions(profile, configuration);
        let admission_check = mesh_admission_check(
            objective,
            profile,
            &summary,
            &route_cost,
            &admission_assumptions,
        );
        let witness = RouteWitness {
            objective_protection: objective.target_protection,
            delivered_protection: summary.protection,
            objective_connectivity: objective.target_connectivity,
            delivered_connectivity: summary.connectivity,
            admission_profile: admission_assumptions,
            topology_epoch: configuration.epoch,
            degradation: estimate.value.degradation,
        };
        let committee = self.selector.as_ref().and_then(|selector| {
            selector
                .select_committee(objective, profile, topology)
                .ok()
                .flatten()
        });
        let path = MeshPath {
            route_id,
            epoch: configuration.epoch,
            source: self.local_node_id,
            destination: objective.destination.clone(),
            segments,
            valid_for,
            route_class,
        };
        Some((
            backend_route_id.clone(),
            CachedCandidate {
                route_id,
                summary,
                estimate,
                admission_check,
                witness,
                path,
                committee,
                route_cost,
                ordering_key: order_key,
            },
        ))
    }

    fn find_cached_candidate_by_route_id(&self, route_id: &RouteId) -> Option<CachedCandidate> {
        self.candidate_cache
            .borrow()
            .values()
            .find(|candidate| &candidate.route_id == route_id)
            .cloned()
    }

    fn store_checkpoint(&mut self, active_route: &ActiveMeshRoute) -> Result<(), RouteError> {
        let key = route_storage_key(&active_route.path.route_id);
        let value = checkpoint_bytes(active_route);
        self.effects
            .store_bytes(&key, &value)
            .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))
    }

    fn remove_checkpoint(&mut self, route_id: &RouteId) -> Result<(), RouteError> {
        self.effects
            .remove_bytes(&route_storage_key(route_id))
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

impl<Topology, Transport, Retention, Effects, Hasher, Selector> RoutingEnginePlanner
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: MeshTopologyModel<
        PeerEstimate = MeshPeerEstimate,
        NeighborhoodEstimate = MeshNeighborhoodEstimate,
    >,
    Transport: MeshTransport + Send + Sync + 'static,
    Retention: RetentionStore,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
    Hasher: Hashing<Digest = Blake3Digest>,
    Selector: CommitteeSelector<TopologyView = Configuration>,
{
    fn engine_id(&self) -> RoutingEngineId {
        MESH_ENGINE_ID
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        MESH_CAPABILITIES.clone()
    }

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        // Five-step deterministic pipeline: BFS shortest paths, filter to
        // route-capable destinations matching the objective, derive a
        // cached candidate per path, sort by hop count and order key,
        // then publish the backend refs. The deterministic sort makes
        // candidate ordering stable across replays.
        let configuration = &topology.value;
        let paths = shortest_paths(&self.local_node_id, configuration);
        let mut cached = Vec::new();
        for (destination_node_id, node_path) in paths {
            if destination_node_id == self.local_node_id {
                continue;
            }
            let Some(destination_node) = configuration.nodes.get(&destination_node_id) else {
                continue;
            };
            if !route_capable_for_engine(destination_node, &MESH_ENGINE_ID, configuration.epoch) {
                continue;
            }
            if !objective_matches_node(
                &destination_node_id,
                destination_node,
                objective,
                &MESH_ENGINE_ID,
                topology.observed_at_tick,
            ) {
                continue;
            }
            if let Some((backend_route_id, candidate)) = self.derive_candidate(
                objective,
                profile,
                topology,
                destination_node_id,
                &node_path,
            ) {
                cached.push((backend_route_id, candidate));
            }
        }

        cached.sort_by_key(|(_backend_route_id, candidate)| {
            (
                candidate.path.segments.len(),
                candidate.ordering_key.stable_key,
                candidate.ordering_key.tie_break,
            )
        });

        let mut cache = self.candidate_cache.borrow_mut();
        cache.clear();

        cached
            .into_iter()
            .map(
                |(backend_route_id, candidate): (BackendRouteId, CachedCandidate)| {
                    cache.insert(backend_route_id.clone(), candidate.clone());
                    RouteCandidate {
                        summary: candidate.summary,
                        estimate: candidate.estimate,
                        backend_ref: jacquard_core::BackendRouteRef {
                            engine: MESH_ENGINE_ID,
                            backend_route_id,
                        },
                    }
                },
            )
            .collect()
    }

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: &RouteCandidate,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        if let Some(cached) = self
            .candidate_cache
            .borrow()
            .get(&candidate.backend_ref.backend_route_id)
        {
            return Ok(cached.admission_check.clone());
        }

        Ok(mesh_admission_check(
            objective,
            profile,
            &candidate.summary,
            &route_cost_for_summary(&candidate.summary),
            &mesh_admission_assumptions(
                profile,
                &Configuration {
                    epoch: candidate.estimate.value.topology_epoch,
                    nodes: BTreeMap::new(),
                    links: BTreeMap::new(),
                    environment: jacquard_core::Environment {
                        reachable_neighbor_count: 0,
                        churn_permille: jacquard_core::RatioPermille(0),
                        contention_permille: jacquard_core::RatioPermille(0),
                    },
                },
            ),
        ))
    }

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
    ) -> Result<RouteAdmission, RouteError> {
        let cached = self
            .candidate_cache
            .borrow()
            .get(&candidate.backend_ref.backend_route_id)
            .cloned()
            .ok_or(RouteSelectionError::NoCandidate)?;

        match cached.admission_check.decision {
            AdmissionDecision::Admissible => Ok(RouteAdmission {
                route_id: cached.route_id,
                objective: objective.clone(),
                profile: profile.clone(),
                admission_check: cached.admission_check,
                summary: cached.summary,
                witness: cached.witness,
            }),
            AdmissionDecision::Rejected(rejection) => {
                Err(RouteSelectionError::Inadmissible(rejection).into())
            }
        }
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector> RoutingEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: MeshTopologyModel<
        PeerEstimate = MeshPeerEstimate,
        NeighborhoodEstimate = MeshNeighborhoodEstimate,
    >,
    Transport: MeshTransport + Send + Sync + 'static,
    Retention: RetentionStore,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
    Hasher: Hashing<Digest = Blake3Digest>,
    Selector: CommitteeSelector<TopologyView = Configuration>,
{
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        self.materialize_route_inner(input)
    }

    fn route_commitments(&self, route: &MaterializedRoute) -> Vec<RouteCommitment> {
        self.route_commitments_inner(route)
    }

    fn engine_tick(&mut self, topology: &Observation<Configuration>) -> Result<(), RouteError> {
        self.engine_tick_inner(topology)
    }

    fn maintain_route(
        &mut self,
        identity: &MaterializedRouteIdentity,
        runtime: &mut jacquard_core::RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        self.maintain_route_inner(identity, runtime, trigger)
    }

    fn teardown(&mut self, route_id: &RouteId) {
        let _ = self.remove_checkpoint(route_id);
        self.active_routes.remove(route_id);
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: MeshTopologyModel<
        PeerEstimate = MeshPeerEstimate,
        NeighborhoodEstimate = MeshNeighborhoodEstimate,
    >,
    Transport: MeshTransport + Send + Sync + 'static,
    Retention: RetentionStore,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
    Hasher: Hashing<Digest = Blake3Digest>,
    Selector: CommitteeSelector<TopologyView = Configuration>,
{
    fn materialize_route_inner(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        // Refuse materialization if the router-owned lease has already
        // expired. This is a typed runtime failure, not a silent fallthrough.
        let cached = self
            .find_cached_candidate_by_route_id(&input.admission.route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        let now = self.effects.now_tick();
        input.lease.ensure_valid_at(now)?;

        let proof = RouteMaterializationProof {
            route_id: input.handle.route_id,
            topology_epoch: input.handle.topology_epoch,
            materialized_at_tick: now,
            publication_id: input.handle.publication_id,
            witness: Fact {
                value: input.admission.witness.clone(),
                basis: FactBasis::Admitted,
                established_at_tick: now,
            },
        };
        let installation = RouteInstallation {
            materialization_proof: proof.clone(),
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: RouteHealth {
                reachability_state: ReachabilityState::Reachable,
                stability_score: mesh_health_score(&self.latest_topology.as_ref().map_or(
                    Configuration {
                        epoch: cached.path.epoch,
                        nodes: BTreeMap::new(),
                        links: BTreeMap::new(),
                        environment: jacquard_core::Environment {
                            reachable_neighbor_count: 0,
                            churn_permille: jacquard_core::RatioPermille(0),
                            contention_permille: jacquard_core::RatioPermille(0),
                        },
                    },
                    |topology| topology.value.clone(),
                )),
                congestion_penalty_points: PenaltyPoints(0),
                last_validated_at_tick: now,
            },
            progress: RouteProgressContract {
                productive_step_count_max: cached.admission_check.productive_step_bound,
                total_step_count_max: cached.admission_check.total_step_bound,
                last_progress_at_tick: now,
                state: RouteProgressState::Satisfied,
            },
        };
        let active_route = ActiveMeshRoute {
            path: cached.path.clone(),
            committee: cached.committee.clone(),
            current_epoch: cached.path.epoch,
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            in_flight_frames: 0,
            last_ack_at_tick: None,
            repair_steps_remaining: limit_u32(cached.admission_check.productive_step_bound),
            route_cost: cached.route_cost.clone(),
            partition_mode: false,
            retained_objects: BTreeSet::new(),
            ordering_key: cached.ordering_key.clone(),
        };
        self.store_checkpoint(&active_route)?;
        self.active_routes
            .insert(input.handle.route_id, active_route);
        self.record_event(RouteEvent::RouteMaterialized {
            handle: input.handle,
            proof,
        })?;
        Ok(installation)
    }

    fn route_commitments_inner(&self, route: &MaterializedRoute) -> Vec<RouteCommitment> {
        let resolution = if route.identity.lease.is_valid_at(self.effects.now_tick()) {
            RouteCommitmentResolution::Pending
        } else {
            RouteCommitmentResolution::Invalidated(RouteInvalidationReason::LeaseExpired)
        };
        vec![RouteCommitment {
            commitment_id: self.commitment_id_for_route(&route.identity.handle.route_id),
            operation_id: RouteOperationId(route.identity.handle.route_id.0),
            route_binding: RouteBinding::Bound(route.identity.handle.route_id),
            owner_node_id: route.identity.lease.owner_node_id,
            deadline_tick: route.identity.lease.valid_for.end_tick(),
            retry_policy: TimeoutPolicy {
                attempt_count_max: 2,
                initial_backoff_ms: jacquard_core::DurationMs(25),
                backoff_multiplier_permille: jacquard_core::RatioPermille(1000),
                backoff_ms_max: jacquard_core::DurationMs(25),
                overall_timeout_ms: jacquard_core::DurationMs(50),
            },
            resolution,
        }]
    }

    fn engine_tick_inner(
        &mut self,
        topology: &Observation<Configuration>,
    ) -> Result<(), RouteError> {
        // Engine-internal middleware loop: refresh the cached topology
        // observation, evict the stale candidate cache, checkpoint the
        // current epoch, and poll transport ingress. Route activation,
        // maintenance, and teardown still happen through the shared
        // trait methods rather than inside this tick.
        self.latest_topology = Some(topology.clone());
        self.candidate_cache.borrow_mut().clear();
        let epoch_bytes = topology.value.epoch.0.to_le_bytes();
        self.effects
            .store_bytes(b"mesh/topology-epoch", &epoch_bytes)
            .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))?;
        let _observations = self.transport.poll_transport()?;
        Ok(())
    }

    fn maintain_route_inner(
        &mut self,
        identity: &MaterializedRouteIdentity,
        runtime: &mut jacquard_core::RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        // Trigger dispatch: LinkDegraded attempts local repair or escalates
        // to replacement when the repair budget is gone; CapacityExceeded
        // and PartitionDetected enter hold-fallback; PolicyShift hands off;
        // EpochAdvanced bumps the epoch and treats it as a repair step;
        // LeaseExpiring and RouteExpired terminate; AntiEntropyRequired
        // only refreshes progress tracking. Lease expiry is checked first
        // as a typed failure regardless of which trigger arrived.
        let now = self.effects.now_tick();
        let handoff_receipt_id = self.receipt_id_for_route(&identity.handle.route_id);
        if !identity.lease.is_valid_at(now) {
            runtime.last_lifecycle_event = RouteLifecycleEvent::Expired;
            runtime.progress.state = RouteProgressState::Failed;
            let result = RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LeaseExpired),
            };
            self.record_event(RouteEvent::RouteMaintenanceCompleted {
                route_id: identity.handle.route_id,
                result: result.clone(),
            })?;
            return Ok(result);
        }

        let active_route_snapshot;
        let result = {
            let active_route = self
                .active_routes
                .get_mut(&identity.handle.route_id)
                .ok_or(RouteSelectionError::NoCandidate)?;
            let result = match trigger {
                RouteMaintenanceTrigger::LinkDegraded => {
                    if active_route.repair_steps_remaining == 0 {
                        RouteMaintenanceResult {
                            event: RouteLifecycleEvent::Replaced,
                            outcome: RouteMaintenanceOutcome::ReplacementRequired { trigger },
                        }
                    } else {
                        active_route.repair_steps_remaining =
                            active_route.repair_steps_remaining.saturating_sub(1);
                        active_route.last_lifecycle_event = RouteLifecycleEvent::Repaired;
                        runtime.last_lifecycle_event = RouteLifecycleEvent::Repaired;
                        runtime.progress.last_progress_at_tick = now;
                        RouteMaintenanceResult {
                            event: RouteLifecycleEvent::Repaired,
                            outcome: RouteMaintenanceOutcome::Repaired,
                        }
                    }
                }
                RouteMaintenanceTrigger::CapacityExceeded
                | RouteMaintenanceTrigger::PartitionDetected => {
                    active_route.partition_mode = true;
                    active_route.last_lifecycle_event = RouteLifecycleEvent::EnteredPartitionMode;
                    runtime.last_lifecycle_event = RouteLifecycleEvent::EnteredPartitionMode;
                    runtime.progress.state = RouteProgressState::Blocked;
                    RouteMaintenanceResult {
                        event: RouteLifecycleEvent::EnteredPartitionMode,
                        outcome: RouteMaintenanceOutcome::HoldFallback { trigger },
                    }
                }
                RouteMaintenanceTrigger::PolicyShift => {
                    let handoff = RouteSemanticHandoff {
                        route_id: identity.handle.route_id,
                        from_node_id: identity.lease.owner_node_id,
                        to_node_id: Self::handoff_target(
                            active_route,
                            identity.lease.owner_node_id,
                        ),
                        handoff_epoch: active_route.current_epoch,
                        receipt_id: handoff_receipt_id,
                    };
                    active_route.last_lifecycle_event = RouteLifecycleEvent::HandedOff;
                    runtime.last_lifecycle_event = RouteLifecycleEvent::HandedOff;
                    RouteMaintenanceResult {
                        event: RouteLifecycleEvent::HandedOff,
                        outcome: RouteMaintenanceOutcome::HandedOff(handoff),
                    }
                }
                RouteMaintenanceTrigger::EpochAdvanced => {
                    if let Some(topology) = &self.latest_topology {
                        active_route.current_epoch = topology.value.epoch;
                    }
                    if active_route.repair_steps_remaining > 0 {
                        active_route.repair_steps_remaining =
                            active_route.repair_steps_remaining.saturating_sub(1);
                        active_route.last_lifecycle_event = RouteLifecycleEvent::Repaired;
                        runtime.last_lifecycle_event = RouteLifecycleEvent::Repaired;
                        RouteMaintenanceResult {
                            event: RouteLifecycleEvent::Repaired,
                            outcome: RouteMaintenanceOutcome::Repaired,
                        }
                    } else {
                        RouteMaintenanceResult {
                            event: RouteLifecycleEvent::Replaced,
                            outcome: RouteMaintenanceOutcome::ReplacementRequired { trigger },
                        }
                    }
                }
                RouteMaintenanceTrigger::LeaseExpiring => RouteMaintenanceResult {
                    event: active_route.last_lifecycle_event,
                    outcome: RouteMaintenanceOutcome::ReplacementRequired { trigger },
                },
                RouteMaintenanceTrigger::RouteExpired => {
                    active_route.last_lifecycle_event = RouteLifecycleEvent::Expired;
                    runtime.last_lifecycle_event = RouteLifecycleEvent::Expired;
                    runtime.progress.state = RouteProgressState::Failed;
                    RouteMaintenanceResult {
                        event: RouteLifecycleEvent::Expired,
                        outcome: RouteMaintenanceOutcome::Failed(
                            RouteMaintenanceFailure::LeaseExpired,
                        ),
                    }
                }
                RouteMaintenanceTrigger::AntiEntropyRequired => {
                    runtime.progress.last_progress_at_tick = now;
                    RouteMaintenanceResult {
                        event: active_route.last_lifecycle_event,
                        outcome: RouteMaintenanceOutcome::Continued,
                    }
                }
            };
            active_route_snapshot = active_route.clone();
            result
        };

        runtime.health.last_validated_at_tick = now;
        self.store_checkpoint(&active_route_snapshot)?;
        self.record_event(RouteEvent::RouteMaintenanceCompleted {
            route_id: identity.handle.route_id,
            result: result.clone(),
        })?;
        Ok(result)
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector> MeshRoutingEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: MeshTopologyModel<
        PeerEstimate = MeshPeerEstimate,
        NeighborhoodEstimate = MeshNeighborhoodEstimate,
    >,
    Transport: MeshTransport + Send + Sync + 'static,
    Retention: RetentionStore,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
    Hasher: Hashing<Digest = Blake3Digest>,
    Selector: CommitteeSelector<TopologyView = Configuration>,
{
    type TopologyModel = Topology;
    type Transport = Transport;
    type Retention = Retention;

    fn topology_model(&self) -> &Self::TopologyModel {
        &self.topology_model
    }

    fn transport(&self) -> &Self::Transport {
        &self.transport
    }

    fn transport_mut(&mut self) -> &mut Self::Transport {
        &mut self.transport
    }

    fn retention_store(&self) -> &Self::Retention {
        &self.retention
    }

    fn retention_store_mut(&mut self) -> &mut Self::Retention {
        &mut self.retention
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector> CommitteeCoordinatedEngine
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: MeshTopologyModel<
        PeerEstimate = MeshPeerEstimate,
        NeighborhoodEstimate = MeshNeighborhoodEstimate,
    >,
    Transport: MeshTransport + Send + Sync + 'static,
    Retention: RetentionStore,
    Selector: CommitteeSelector<TopologyView = Configuration>,
{
    type Selector = Selector;

    fn committee_selector(&self) -> Option<&Self::Selector> {
        self.selector.as_ref()
    }
}

// Unweighted BFS. Returns the shortest node path from the local node to
// every reachable node, using the sorted neighbor order from the
// topology helper so the result is fully deterministic.
fn shortest_paths(
    local_node_id: &NodeId,
    configuration: &Configuration,
) -> BTreeMap<NodeId, Vec<NodeId>> {
    let mut visited = BTreeMap::new();
    let mut queue = VecDeque::new();

    visited.insert(*local_node_id, vec![*local_node_id]);
    queue.push_back(*local_node_id);

    while let Some(current) = queue.pop_front() {
        let Some(current_path) = visited.get(&current).cloned() else {
            continue;
        };
        for neighbor in adjacent_node_ids(&current, configuration) {
            if visited.contains_key(&neighbor) {
                continue;
            }
            let mut next_path = current_path.clone();
            next_path.push(neighbor);
            visited.insert(neighbor, next_path);
            queue.push_back(neighbor);
        }
    }

    visited
}

fn unique_protocol_mix(segments: &[MeshRouteSegment]) -> Vec<jacquard_core::TransportProtocol> {
    let mut protocols = segments
        .iter()
        .map(|segment| segment.endpoint.protocol.clone())
        .collect::<Vec<_>>();
    protocols.sort();
    protocols.dedup();
    protocols
}

fn encode_path_bytes(path: &[NodeId], segments: &[MeshRouteSegment]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for node_id in path {
        bytes.extend_from_slice(&node_id.0);
    }
    for segment in segments {
        bytes.extend_from_slice(&segment.node_id.0);
        bytes.extend_from_slice(&segment.endpoint.mtu_bytes.0.to_le_bytes());
    }
    bytes
}

fn deterministic_order_key<H: Hashing<Digest = Blake3Digest>>(
    route_id: RouteId,
    hashing: &H,
    path_bytes: &[u8],
) -> DeterministicOrderKey<RouteId> {
    let digest = hashing.hash_tagged(b"mesh-order-key", path_bytes);
    let mut tie_break_bytes = [0_u8; 8];
    tie_break_bytes.copy_from_slice(&digest.0[..8]);
    DeterministicOrderKey {
        stable_key: route_id,
        tie_break: OrderStamp(u64::from_le_bytes(tie_break_bytes)),
    }
}

fn confidence_for_segments(
    segments: &[MeshRouteSegment],
    configuration: &Configuration,
) -> jacquard_core::RatioPermille {
    let mut confidence = 1000_u16;
    let mut previous = None;
    for segment in segments {
        if let Some(from) = previous {
            if let Some(link) = adjacent_link_between(&from, &segment.node_id, configuration) {
                confidence = confidence.min(
                    link.state
                        .delivery_confidence_permille
                        .into_estimate()
                        .map_or(jacquard_core::RatioPermille(0), |estimate| estimate.value)
                        .get(),
                );
            }
        }
        previous = Some(segment.node_id);
    }
    jacquard_core::RatioPermille(confidence)
}

fn degradation_for_candidate(
    configuration: &Configuration,
    route_class: &MeshRouteClass,
) -> RouteDegradation {
    if matches!(route_class, MeshRouteClass::DeferredDelivery) {
        RouteDegradation::Degraded(DegradationReason::PartitionRisk)
    } else if configuration.environment.contention_permille.get() > 600 {
        RouteDegradation::Degraded(DegradationReason::CapacityPressure)
    } else if configuration.environment.churn_permille.get() > 600 {
        RouteDegradation::Degraded(DegradationReason::LinkInstability)
    } else {
        RouteDegradation::None
    }
}

// Admission has three rejection paths and one admissible path. Order
// matters: the protection floor is the hard security invariant, so it is
// checked first; repair and partition are profile-driven connectivity
// requirements checked only after protection passes.
fn mesh_admission_check(
    objective: &RoutingObjective,
    profile: &AdaptiveRoutingProfile,
    summary: &RouteSummary,
    route_cost: &RouteCost,
    assumptions: &AdmissionAssumptions,
) -> RouteAdmissionCheck {
    let decision = if summary.protection < objective.protection_floor {
        AdmissionDecision::Rejected(RouteAdmissionRejection::ProtectionFloorUnsatisfied)
    } else if profile.selected_connectivity.repair == RouteRepairClass::Repairable
        && summary.connectivity.repair != RouteRepairClass::Repairable
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::BranchingInfeasible)
    } else if profile.selected_connectivity.partition == RoutePartitionClass::PartitionTolerant
        && summary.connectivity.partition != RoutePartitionClass::PartitionTolerant
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable)
    } else {
        AdmissionDecision::Admissible
    };

    RouteAdmissionCheck {
        decision,
        profile: assumptions.clone(),
        productive_step_bound: Limit::Bounded(route_cost.hop_count.into()),
        total_step_bound: Limit::Bounded(route_cost.hop_count.into()),
        route_cost: route_cost.clone(),
    }
}

fn route_cost_for_segments(
    segments: &[MeshRouteSegment],
    route_class: &MeshRouteClass,
) -> RouteCost {
    let hop_count = u8::try_from(segments.len()).unwrap_or(u8::MAX);
    let hold_reserved = match route_class {
        MeshRouteClass::DeferredDelivery => ByteCount(1024),
        _ => ByteCount(0),
    };
    RouteCost {
        message_count_max: Limit::Bounded(u32::from(hop_count)),
        byte_count_max: Limit::Bounded(ByteCount(u64::from(hop_count) * 1024)),
        hop_count,
        repair_attempt_count_max: Limit::Bounded(u32::from(hop_count)),
        hold_bytes_reserved: Limit::Bounded(hold_reserved),
        work_step_count_max: Limit::Bounded(u32::from(hop_count) + 1),
    }
}

fn route_cost_for_summary(summary: &RouteSummary) -> RouteCost {
    let hop_count = match summary.hop_count_hint {
        Belief::Absent => 1,
        Belief::Estimated(estimate) => estimate.value,
    };
    let hold_reserved = if summary.connectivity.partition == RoutePartitionClass::PartitionTolerant
    {
        ByteCount(1024)
    } else {
        ByteCount(0)
    };
    RouteCost {
        message_count_max: Limit::Bounded(u32::from(hop_count)),
        byte_count_max: Limit::Bounded(ByteCount(u64::from(hop_count) * 1024)),
        hop_count,
        repair_attempt_count_max: Limit::Bounded(u32::from(hop_count)),
        hold_bytes_reserved: Limit::Bounded(hold_reserved),
        work_step_count_max: Limit::Bounded(u32::from(hop_count) + 1),
    }
}

fn checkpoint_bytes(active_route: &ActiveMeshRoute) -> Vec<u8> {
    let mut bytes = active_route.path.route_id.0.to_vec();
    bytes.extend_from_slice(&active_route.current_epoch.0.to_le_bytes());
    bytes.extend_from_slice(&active_route.route_cost.hop_count.to_le_bytes());
    bytes.extend_from_slice(&active_route.repair_steps_remaining.to_le_bytes());
    bytes.push(u8::from(active_route.partition_mode));
    bytes
}

fn route_storage_key(route_id: &RouteId) -> Vec<u8> {
    let mut key = b"mesh/route/".to_vec();
    key.extend_from_slice(&route_id.0);
    key
}

fn limit_u32(limit: Limit<u32>) -> u32 {
    match limit {
        Limit::Unbounded => u32::MAX,
        Limit::Bounded(value) => value,
    }
}

trait BeliefExt<T> {
    fn into_estimate(self) -> Option<jacquard_core::Estimate<T>>;
}

impl<T> BeliefExt<T> for Belief<T> {
    fn into_estimate(self) -> Option<jacquard_core::Estimate<T>> {
        match self {
            Belief::Absent => None,
            Belief::Estimated(estimate) => Some(estimate),
        }
    }
}
