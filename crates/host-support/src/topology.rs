//! Host-side topology and canonical-route projection utilities.
//!
//! This module provides a pure observational projector that consumes explicit
//! host-observed topology and router-owned canonical route updates to build a
//! stable read model for diagnostics, tests, and host UIs.
//!
//! It is intentionally conservative:
//! - no async watch or broadcast surface
//! - no transport-specific decoding
//! - no router or engine logic
//! - no canonical publication

use alloc::{collections::BTreeMap, vec::Vec};

use jacquard_core::{
    Configuration, DestinationId, Link, MaterializedRoute, Node, NodeId, Observation, RouteEvent,
    RouteEventStamped, RouteHealth, RouteId, RouteLifecycleEvent, RouteShapeVisibility,
    RouterCanonicalMutation, RouterRoundOutcome, RoutingEngineCapabilities, RoutingEngineId, Tick,
    TransportDeliveryMode,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedNode {
    pub controller_id: jacquard_core::ControllerId,
    pub profile: jacquard_core::NodeProfile,
    pub state: jacquard_core::NodeState,
    pub observed_at_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedLink {
    pub endpoint: jacquard_core::LinkEndpoint,
    pub profile: jacquard_core::LinkProfile,
    pub state: jacquard_core::LinkState,
    pub observed_at_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservedRouteShape {
    ExplicitPath,
    CorridorEnvelope,
    NextHopOnly,
    Opaque,
}

impl ObservedRouteShape {
    #[must_use]
    pub fn from_visibility(visibility: RouteShapeVisibility) -> Self {
        match visibility {
            RouteShapeVisibility::ExplicitPath => Self::ExplicitPath,
            RouteShapeVisibility::CorridorEnvelope => Self::CorridorEnvelope,
            RouteShapeVisibility::NextHopOnly => Self::NextHopOnly,
            RouteShapeVisibility::Opaque => Self::Opaque,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedRoute {
    pub route_id: RouteId,
    pub destination: DestinationId,
    pub engine_id: RoutingEngineId,
    pub route_shape: ObservedRouteShape,
    pub delivery_mode: TransportDeliveryMode,
    pub hop_count_hint: jacquard_core::Belief<u8>,
    pub topology_epoch: jacquard_core::RouteEpoch,
    pub publication_id: jacquard_core::PublicationId,
    pub lease: jacquard_core::RouteLease,
    pub protection: jacquard_core::RouteProtectionClass,
    pub connectivity: jacquard_core::ConnectivityPosture,
    pub protocol_mix: Vec<jacquard_core::TransportKind>,
    pub lifecycle_event: RouteLifecycleEvent,
    pub lifecycle_updated_at_tick: Tick,
    pub health: RouteHealth,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySnapshot {
    pub local_node_id: NodeId,
    pub observed_at_tick: Tick,
    pub nodes: BTreeMap<NodeId, ObservedNode>,
    pub links: BTreeMap<(NodeId, NodeId), ObservedLink>,
    pub active_routes: BTreeMap<RouteId, ObservedRoute>,
}

pub struct TopologyProjector {
    snapshot: TopologySnapshot,
    engine_capabilities: BTreeMap<RoutingEngineId, RoutingEngineCapabilities>,
}

impl TopologyProjector {
    #[must_use]
    pub fn new(local_node_id: NodeId, initial: Observation<Configuration>) -> Self {
        let mut projector = Self {
            snapshot: TopologySnapshot {
                local_node_id,
                observed_at_tick: initial.observed_at_tick,
                nodes: BTreeMap::new(),
                links: BTreeMap::new(),
                active_routes: BTreeMap::new(),
            },
            engine_capabilities: BTreeMap::new(),
        };
        projector.ingest_topology(initial);
        projector
    }

    pub fn ingest_topology(&mut self, observation: Observation<Configuration>) {
        self.snapshot.observed_at_tick = observation.observed_at_tick;
        self.snapshot.nodes = observation
            .value
            .nodes
            .into_iter()
            .map(|(node_id, node)| {
                (
                    node_id,
                    ObservedNode::from_node(node, observation.observed_at_tick),
                )
            })
            .collect();
        self.snapshot.links = observation
            .value
            .links
            .into_iter()
            .map(|(endpoints, link)| {
                (
                    endpoints,
                    ObservedLink::from_link(link, observation.observed_at_tick),
                )
            })
            .collect();
    }

    pub fn ingest_engine_capabilities(&mut self, capabilities: RoutingEngineCapabilities) {
        let visibility = capabilities.route_shape_visibility;
        let engine_id = capabilities.engine.clone();
        self.engine_capabilities
            .insert(engine_id.clone(), capabilities);
        for route in self.snapshot.active_routes.values_mut() {
            if route.engine_id == engine_id {
                route.route_shape = ObservedRouteShape::from_visibility(visibility);
            }
        }
    }

    pub fn ingest_materialized_route(&mut self, route: &MaterializedRoute) {
        self.snapshot.observed_at_tick = self
            .snapshot
            .observed_at_tick
            .max(route.identity.materialized_at_tick());
        self.snapshot.active_routes.insert(
            *route.identity.route_id(),
            self.project_route(route, route.identity.materialized_at_tick()),
        );
    }

    pub fn ingest_route_event(&mut self, event: &RouteEventStamped) {
        self.snapshot.observed_at_tick = self.snapshot.observed_at_tick.max(event.emitted_at_tick);
        match &event.event {
            RouteEvent::RouteMaterialized { handle, .. } => {
                if let Some(route) = self.snapshot.active_routes.get_mut(handle.route_id()) {
                    route.lifecycle_event = RouteLifecycleEvent::Activated;
                    route.lifecycle_updated_at_tick = event.emitted_at_tick;
                }
            }
            RouteEvent::RouteMaintenanceCompleted { route_id, result } => {
                if let Some(route) = self.snapshot.active_routes.get_mut(route_id) {
                    route.lifecycle_event = result.event;
                    route.lifecycle_updated_at_tick = event.emitted_at_tick;
                }
            }
            RouteEvent::RouteCommitmentUpdated { .. } => {}
            RouteEvent::RouteHealthObserved { route_id, health } => {
                if let Some(route) = self.snapshot.active_routes.get_mut(route_id) {
                    route.health = health.value.clone();
                    route.lifecycle_updated_at_tick =
                        route.lifecycle_updated_at_tick.max(health.observed_at_tick);
                }
            }
        }
    }

    pub fn ingest_round_outcome(&mut self, outcome: &RouterRoundOutcome) {
        match &outcome.canonical_mutation {
            RouterCanonicalMutation::None => {}
            RouterCanonicalMutation::RouteReplaced {
                previous_route_id,
                route,
            } => {
                self.snapshot.active_routes.remove(previous_route_id);
                self.ingest_materialized_route(route);
                if let Some(current) = self
                    .snapshot
                    .active_routes
                    .get_mut(route.identity.route_id())
                {
                    current.lifecycle_event = RouteLifecycleEvent::Replaced;
                }
            }
            RouterCanonicalMutation::LeaseTransferred {
                route_id,
                handoff: _,
                lease,
            } => {
                if let Some(route) = self.snapshot.active_routes.get_mut(route_id) {
                    route.lease = lease.clone();
                }
            }
            RouterCanonicalMutation::RouteExpired { route_id } => {
                self.snapshot.active_routes.remove(route_id);
            }
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> &TopologySnapshot {
        &self.snapshot
    }

    fn project_route(
        &self,
        route: &MaterializedRoute,
        lifecycle_updated_at_tick: Tick,
    ) -> ObservedRoute {
        let summary = &route.identity.admission.summary;
        ObservedRoute {
            route_id: *route.identity.route_id(),
            destination: route.identity.admission.objective.destination.clone(),
            engine_id: summary.engine.clone(),
            route_shape: self.project_shape(&summary.engine),
            delivery_mode: delivery_mode_for_destination(
                &route.identity.admission.objective.destination,
            ),
            hop_count_hint: summary.hop_count_hint,
            topology_epoch: route.identity.topology_epoch(),
            publication_id: *route.identity.publication_id(),
            lease: route.identity.lease.clone(),
            protection: summary.protection,
            connectivity: summary.connectivity,
            protocol_mix: summary.protocol_mix.clone(),
            lifecycle_event: route.runtime.last_lifecycle_event,
            lifecycle_updated_at_tick,
            health: route.runtime.health.clone(),
        }
    }

    fn project_shape(&self, engine_id: &RoutingEngineId) -> ObservedRouteShape {
        let visibility = self
            .engine_capabilities
            .get(engine_id)
            .map_or(RouteShapeVisibility::Opaque, |caps| {
                caps.route_shape_visibility
            });
        ObservedRouteShape::from_visibility(visibility)
    }
}

fn delivery_mode_for_destination(destination: &DestinationId) -> TransportDeliveryMode {
    match destination {
        DestinationId::Node(_) => TransportDeliveryMode::Unicast,
        DestinationId::Service(_) | DestinationId::Gateway(_) => TransportDeliveryMode::Unicast,
    }
}

impl ObservedNode {
    fn from_node(node: Node, observed_at_tick: Tick) -> Self {
        Self {
            controller_id: node.controller_id,
            profile: node.profile,
            state: node.state,
            observed_at_tick,
        }
    }
}

impl ObservedLink {
    fn from_link(link: Link, observed_at_tick: Tick) -> Self {
        Self {
            endpoint: link.endpoint,
            profile: link.profile,
            state: link.state,
            observed_at_tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteId, Belief,
        ByteCount, CapacityHint, ClaimStrength, Configuration, ConnectivityPosture,
        ConnectivityRegime, ControllerId, DegradationReason, DestinationId, DurationMs,
        EndpointLocator, Environment, Estimate, Fact, FactBasis, FactSourceClass,
        FailureModelClass, HealthScore, HoldItemCount, Limit, Link, LinkEndpoint, LinkProfile,
        LinkRuntimeState, LinkState, MessageFlowAssumptionClass, NodeBuilder, NodeDensityClass,
        NodeId, NodeProfileBuilder, NodeStateBuilder, Observation, OperatingMode, OrderStamp,
        OriginAuthenticationClass, PartitionRecoveryClass, PenaltyPoints, PriorityPoints,
        PublicationId, QuantitativeBoundSupport, RatioPermille, ReachabilityState,
        ReconfigurationSupport, RelayWorkBudget, RepairCapability, RepairSupport, RouteAdmission,
        RouteAdmissionCheck, RouteCandidate, RouteCost, RouteDegradation, RouteEpoch,
        RouteEstimate, RouteEvent, RouteEventStamped, RouteHandle, RouteHealth, RouteId,
        RouteLease, RouteLifecycleEvent, RouteMaintenanceOutcome, RouteMaintenanceResult,
        RouteMaterializationInput, RouteMaterializationProof, RoutePartitionClass,
        RouteProgressContract, RouteProgressState, RouteProtectionClass, RouteRepairClass,
        RouteReplacementPolicy, RouteSemanticHandoff, RouteServiceKind, RouteShapeVisibility,
        RouteWitness, RouterCanonicalMutation, RouterRoundOutcome, RoutingEngineCapabilities,
        RoutingEngineFallbackPolicy, RoutingEngineId, RoutingEvidenceClass, RoutingObjective,
        RuntimeEnvelopeClass, SelectedRoutingParameters, ServiceDescriptorBuilder, Tick,
        TimeWindow, TransportKind,
    };

    use super::*;

    const LOCAL_NODE_ID: NodeId = NodeId([1; 32]);
    const REMOTE_NODE_ID: NodeId = NodeId([2; 32]);
    const ENGINE_A: RoutingEngineId = RoutingEngineId::from_contract_bytes(*b"test.engine.a...");
    const ENGINE_B: RoutingEngineId = RoutingEngineId::from_contract_bytes(*b"test.engine.b...");

    fn topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(2),
                nodes: BTreeMap::from([
                    (LOCAL_NODE_ID, node(LOCAL_NODE_ID, 1)),
                    (REMOTE_NODE_ID, node(REMOTE_NODE_ID, 2)),
                ]),
                links: BTreeMap::from([((LOCAL_NODE_ID, REMOTE_NODE_ID), link(7))]),
                environment: Environment {
                    reachable_neighbor_count: 1,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(5),
        }
    }

    fn endpoint(byte: u8) -> LinkEndpoint {
        LinkEndpoint::new(
            TransportKind::Custom("reference".to_owned()),
            EndpointLocator::Opaque(vec![byte]),
            ByteCount(128),
        )
    }

    fn node(node_id: NodeId, controller_byte: u8) -> jacquard_core::Node {
        let controller_id = ControllerId([controller_byte; 32]);
        let endpoint = endpoint(controller_byte);
        let service = ServiceDescriptorBuilder::new(node_id, controller_id, RouteServiceKind::Move)
            .with_endpoint(endpoint.clone())
            .with_routing_engine(&ENGINE_A)
            .with_valid_for(TimeWindow::new(Tick(1), Tick(10)).expect("window"))
            .with_capacity(
                CapacityHint::new(RatioPermille(200))
                    .with_hold_capacity_bytes(ByteCount(128), Tick(5)),
                Tick(5),
            )
            .build();
        let profile = NodeProfileBuilder::new()
            .with_service(service)
            .with_endpoint(endpoint)
            .with_connection_limits(2, 4, 2, 2)
            .with_work_budgets(
                RelayWorkBudget(10),
                jacquard_core::MaintenanceWorkBudget(10),
            )
            .with_hold_limits(HoldItemCount(2), ByteCount(512))
            .build();
        let state = NodeStateBuilder::new()
            .with_relay_budget(
                RelayWorkBudget(4),
                RatioPermille(250),
                DurationMs(1_000),
                Tick(5),
            )
            .with_available_connections(1, Tick(5))
            .with_hold_capacity(ByteCount(256), Tick(5))
            .with_information_summary(HoldItemCount(2), ByteCount(128), RatioPermille(10), Tick(5))
            .build();
        NodeBuilder::new(controller_id, profile, state).build()
    }

    fn link(byte: u8) -> Link {
        Link {
            endpoint: jacquard_core::LinkEndpoint::new(
                TransportKind::Custom("reference".to_owned()),
                EndpointLocator::Opaque(vec![byte]),
                ByteCount(128),
            ),
            profile: LinkProfile {
                latency_floor_ms: DurationMs(8),
                repair_capability: RepairCapability::TransportRetransmit,
                partition_recovery: PartitionRecoveryClass::LocalReconnect,
            },
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: Belief::certain(DurationMs(12), Tick(5)),
                transfer_rate_bytes_per_sec: Belief::certain(4_096, Tick(5)),
                stability_horizon_ms: Belief::certain(DurationMs(1_000), Tick(5)),
                loss_permille: RatioPermille(25),
                delivery_confidence_permille: Belief::certain(RatioPermille(960), Tick(5)),
                symmetry_permille: Belief::certain(RatioPermille(990), Tick(5)),
            },
        }
    }

    fn capabilities(
        engine: RoutingEngineId,
        route_shape_visibility: RouteShapeVisibility,
    ) -> RoutingEngineCapabilities {
        RoutingEngineCapabilities {
            engine,
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::PartitionTolerant,
            },
            repair_support: RepairSupport::Supported,
            hold_support: jacquard_core::HoldSupport::Supported,
            decidable_admission: jacquard_core::DecidableSupport::Supported,
            quantitative_bounds: QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support: ReconfigurationSupport::LinkAndDelegate,
            route_shape_visibility,
        }
    }

    fn materialized_route(engine: RoutingEngineId, route_byte: u8) -> MaterializedRoute {
        let objective = route_objective();
        let summary = route_summary(engine.clone());
        let candidate = route_candidate(engine, route_byte, &summary);
        let witness = route_witness(summary.connectivity);
        let input =
            route_materialization_input(route_byte, objective, summary, &candidate, &witness);
        MaterializedRoute::from_installation(input, route_installation(route_byte, witness))
    }

    fn route_objective() -> RoutingObjective {
        RoutingObjective {
            destination: DestinationId::Node(REMOTE_NODE_ID),
            service_kind: RouteServiceKind::Move,
            target_protection: RouteProtectionClass::LinkProtected,
            protection_floor: RouteProtectionClass::LinkProtected,
            target_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::PartitionTolerant,
            },
            hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
            latency_budget_ms: Limit::Bounded(DurationMs(250)),
            protection_priority: PriorityPoints(10),
            connectivity_priority: PriorityPoints(20),
        }
    }

    fn route_summary(engine: RoutingEngineId) -> jacquard_core::RouteSummary {
        jacquard_core::RouteSummary {
            engine,
            protection: RouteProtectionClass::LinkProtected,
            connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::PartitionTolerant,
            },
            protocol_mix: vec![TransportKind::Custom("reference".to_owned())],
            hop_count_hint: Belief::certain(2, Tick(5)),
            valid_for: TimeWindow::new(Tick(5), Tick(20)).expect("valid summary window"),
        }
    }

    fn route_candidate(
        engine: RoutingEngineId,
        route_byte: u8,
        summary: &jacquard_core::RouteSummary,
    ) -> RouteCandidate {
        RouteCandidate {
            route_id: RouteId([route_byte; 16]),
            summary: summary.clone(),
            estimate: Estimate {
                value: RouteEstimate {
                    estimated_protection: summary.protection,
                    estimated_connectivity: summary.connectivity,
                    topology_epoch: RouteEpoch(2),
                    degradation: RouteDegradation::Degraded(DegradationReason::LinkInstability),
                },
                confidence_permille: RatioPermille(950),
                updated_at_tick: Tick(5),
            },
            backend_ref: jacquard_core::BackendRouteRef {
                engine,
                backend_route_id: BackendRouteId(vec![1, 2, 3]),
            },
        }
    }

    fn route_witness(connectivity: ConnectivityPosture) -> RouteWitness {
        RouteWitness {
            protection: jacquard_core::ObjectiveVsDelivered {
                objective: RouteProtectionClass::LinkProtected,
                delivered: RouteProtectionClass::LinkProtected,
            },
            connectivity: jacquard_core::ObjectiveVsDelivered {
                objective: connectivity,
                delivered: connectivity,
            },
            admission_profile: AdmissionAssumptions {
                message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
                failure_model: FailureModelClass::Benign,
                runtime_envelope: RuntimeEnvelopeClass::Canonical,
                node_density_class: NodeDensityClass::Moderate,
                connectivity_regime: ConnectivityRegime::Stable,
                adversary_regime: AdversaryRegime::Cooperative,
                claim_strength: ClaimStrength::ConservativeUnderProfile,
            },
            topology_epoch: RouteEpoch(2),
            degradation: RouteDegradation::Degraded(DegradationReason::LinkInstability),
        }
    }

    fn route_materialization_input(
        route_byte: u8,
        objective: RoutingObjective,
        summary: jacquard_core::RouteSummary,
        candidate: &RouteCandidate,
        witness: &RouteWitness,
    ) -> RouteMaterializationInput {
        RouteMaterializationInput {
            handle: RouteHandle {
                stamp: route_identity_stamp(route_byte),
            },
            admission: RouteAdmission {
                backend_ref: candidate.backend_ref.clone(),
                objective,
                profile: SelectedRoutingParameters {
                    selected_protection: RouteProtectionClass::LinkProtected,
                    selected_connectivity: summary.connectivity,
                    deployment_profile: OperatingMode::DenseInteractive,
                    diversity_floor: jacquard_core::DiversityFloor(1),
                    routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
                    route_replacement_policy: RouteReplacementPolicy::Allowed,
                },
                admission_check: route_admission_check(),
                summary,
                witness: witness.clone(),
            },
            lease: RouteLease {
                owner_node_id: LOCAL_NODE_ID,
                lease_epoch: RouteEpoch(2),
                valid_for: TimeWindow::new(Tick(6), Tick(12)).expect("lease window"),
            },
        }
    }

    fn route_admission_check() -> RouteAdmissionCheck {
        RouteAdmissionCheck {
            decision: AdmissionDecision::Admissible,
            profile: AdmissionAssumptions {
                message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
                failure_model: FailureModelClass::Benign,
                runtime_envelope: RuntimeEnvelopeClass::Canonical,
                node_density_class: NodeDensityClass::Moderate,
                connectivity_regime: ConnectivityRegime::Stable,
                adversary_regime: AdversaryRegime::Cooperative,
                claim_strength: ClaimStrength::ConservativeUnderProfile,
            },
            productive_step_bound: Limit::Bounded(3),
            total_step_bound: Limit::Bounded(6),
            route_cost: RouteCost {
                message_count_max: Limit::Bounded(8),
                byte_count_max: Limit::Bounded(ByteCount(512)),
                hop_count: 2,
                repair_attempt_count_max: Limit::Bounded(2),
                hold_bytes_reserved: Limit::Bounded(ByteCount(128)),
                work_step_count_max: Limit::Bounded(12),
            },
        }
    }

    fn route_installation(
        route_byte: u8,
        witness: RouteWitness,
    ) -> jacquard_core::RouteInstallation {
        jacquard_core::RouteInstallation {
            materialization_proof: RouteMaterializationProof {
                stamp: route_identity_stamp(route_byte),
                witness: Fact {
                    value: witness,
                    basis: FactBasis::Published,
                    established_at_tick: Tick(6),
                },
            },
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: RouteHealth {
                reachability_state: ReachabilityState::Reachable,
                stability_score: HealthScore(900),
                congestion_penalty_points: PenaltyPoints(4),
                last_validated_at_tick: Tick(6),
            },
            progress: RouteProgressContract {
                productive_step_count_max: Limit::Bounded(4),
                total_step_count_max: Limit::Bounded(8),
                last_progress_at_tick: Tick(6),
                state: RouteProgressState::Satisfied,
            },
        }
    }

    fn route_identity_stamp(route_byte: u8) -> jacquard_core::RouteIdentityStamp {
        jacquard_core::RouteIdentityStamp {
            route_id: RouteId([route_byte; 16]),
            topology_epoch: RouteEpoch(2),
            materialized_at_tick: Tick(6),
            publication_id: PublicationId([route_byte; 16]),
        }
    }

    #[test]
    fn topology_ingestion_replaces_node_and_link_projection() {
        let projector = TopologyProjector::new(LOCAL_NODE_ID, topology());

        assert_eq!(projector.snapshot().local_node_id, LOCAL_NODE_ID);
        assert_eq!(projector.snapshot().observed_at_tick, Tick(5));
        assert_eq!(projector.snapshot().nodes.len(), 2);
        assert_eq!(projector.snapshot().links.len(), 1);
    }

    #[test]
    fn materialized_route_defaults_to_opaque_until_capabilities_are_known() {
        let mut projector = TopologyProjector::new(LOCAL_NODE_ID, topology());
        let route = materialized_route(ENGINE_A, 9);

        projector.ingest_materialized_route(&route);

        let projected = projector
            .snapshot()
            .active_routes
            .get(route.identity.route_id())
            .expect("projected route");
        assert_eq!(projected.engine_id, ENGINE_A);
        assert_eq!(projected.route_shape, ObservedRouteShape::Opaque);
        assert_eq!(projected.delivery_mode, TransportDeliveryMode::Unicast);
        assert_eq!(projected.hop_count_hint, Belief::certain(2, Tick(5)));
    }

    #[test]
    fn capabilities_upgrade_existing_route_shape_projection() {
        let mut projector = TopologyProjector::new(LOCAL_NODE_ID, topology());
        let route = materialized_route(ENGINE_A, 9);
        projector.ingest_materialized_route(&route);
        projector
            .ingest_engine_capabilities(capabilities(ENGINE_A, RouteShapeVisibility::ExplicitPath));

        let projected = projector
            .snapshot()
            .active_routes
            .get(route.identity.route_id())
            .expect("projected route");
        assert_eq!(projected.route_shape, ObservedRouteShape::ExplicitPath);
        assert_eq!(projected.hop_count_hint, Belief::certain(2, Tick(5)));
    }

    #[test]
    fn maintenance_events_update_projected_lifecycle_and_health() {
        let mut projector = TopologyProjector::new(LOCAL_NODE_ID, topology());
        let route = materialized_route(ENGINE_B, 9);
        projector.ingest_materialized_route(&route);
        projector
            .ingest_engine_capabilities(capabilities(ENGINE_B, RouteShapeVisibility::NextHopOnly));

        projector.ingest_route_event(&RouteEventStamped {
            order_stamp: OrderStamp(2),
            emitted_at_tick: Tick(7),
            event: RouteEvent::RouteMaintenanceCompleted {
                route_id: *route.identity.route_id(),
                result: RouteMaintenanceResult {
                    event: RouteLifecycleEvent::Repaired,
                    outcome: RouteMaintenanceOutcome::Repaired,
                },
            },
        });
        projector.ingest_route_event(&RouteEventStamped {
            order_stamp: OrderStamp(3),
            emitted_at_tick: Tick(8),
            event: RouteEvent::RouteHealthObserved {
                route_id: *route.identity.route_id(),
                health: Observation {
                    value: RouteHealth {
                        reachability_state: ReachabilityState::Reachable,
                        stability_score: HealthScore(850),
                        congestion_penalty_points: PenaltyPoints(7),
                        last_validated_at_tick: Tick(8),
                    },
                    source_class: FactSourceClass::Local,
                    evidence_class: RoutingEvidenceClass::DirectObservation,
                    origin_authentication: OriginAuthenticationClass::Controlled,
                    observed_at_tick: Tick(8),
                },
            },
        });

        let projected = projector
            .snapshot()
            .active_routes
            .get(route.identity.route_id())
            .expect("projected route");
        assert_eq!(projected.lifecycle_event, RouteLifecycleEvent::Repaired);
        assert_eq!(projected.lifecycle_updated_at_tick, Tick(8));
        assert_eq!(projected.health.stability_score, HealthScore(850));
        assert_eq!(projected.route_shape, ObservedRouteShape::NextHopOnly);
        assert_eq!(projected.hop_count_hint, Belief::certain(2, Tick(5)));
    }

    #[test]
    // long-block-exception: this test exercises the full canonical mutation
    // lifecycle in one sequence so lease transfer and expiry stay legible.
    fn round_outcomes_apply_router_owned_canonical_mutations() {
        let mut projector = TopologyProjector::new(LOCAL_NODE_ID, topology());
        let route = materialized_route(ENGINE_A, 9);
        let replacement = materialized_route(ENGINE_B, 10);
        projector.ingest_materialized_route(&route);

        projector.ingest_round_outcome(&RouterRoundOutcome {
            topology_epoch: RouteEpoch(2),
            engine_change: jacquard_core::RoutingTickChange::PrivateStateUpdated,
            next_round_hint: jacquard_core::RoutingTickHint::Immediate,
            canonical_mutation: RouterCanonicalMutation::RouteReplaced {
                previous_route_id: *route.identity.route_id(),
                route: Box::new(replacement.clone()),
            },
        });
        assert!(!projector
            .snapshot()
            .active_routes
            .contains_key(route.identity.route_id()));
        assert!(projector
            .snapshot()
            .active_routes
            .contains_key(replacement.identity.route_id()));

        projector.ingest_round_outcome(&RouterRoundOutcome {
            topology_epoch: RouteEpoch(2),
            engine_change: jacquard_core::RoutingTickChange::NoChange,
            next_round_hint: jacquard_core::RoutingTickHint::HostDefault,
            canonical_mutation: RouterCanonicalMutation::LeaseTransferred {
                route_id: *replacement.identity.route_id(),
                handoff: RouteSemanticHandoff {
                    route_id: *replacement.identity.route_id(),
                    from_node_id: LOCAL_NODE_ID,
                    to_node_id: REMOTE_NODE_ID,
                    handoff_epoch: RouteEpoch(3),
                    receipt_id: jacquard_core::ReceiptId([8; 16]),
                },
                lease: RouteLease {
                    owner_node_id: REMOTE_NODE_ID,
                    lease_epoch: RouteEpoch(3),
                    valid_for: TimeWindow::new(Tick(8), Tick(20)).expect("lease"),
                },
            },
        });
        assert_eq!(
            projector
                .snapshot()
                .active_routes
                .get(replacement.identity.route_id())
                .expect("replacement route")
                .lease
                .owner_node_id,
            REMOTE_NODE_ID
        );

        projector.ingest_round_outcome(&RouterRoundOutcome {
            topology_epoch: RouteEpoch(3),
            engine_change: jacquard_core::RoutingTickChange::NoChange,
            next_round_hint: jacquard_core::RoutingTickHint::HostDefault,
            canonical_mutation: RouterCanonicalMutation::RouteExpired {
                route_id: *replacement.identity.route_id(),
            },
        });
        assert!(!projector
            .snapshot()
            .active_routes
            .contains_key(replacement.identity.route_id()));
    }

    // long-block-exception: this regression keeps one full published route
    // event fixture inline so the non-invention contract is auditable.
    #[test]
    fn route_event_without_canonical_route_does_not_invent_route_truth() {
        let mut projector = TopologyProjector::new(LOCAL_NODE_ID, topology());

        projector.ingest_route_event(&RouteEventStamped {
            order_stamp: OrderStamp(1),
            emitted_at_tick: Tick(6),
            event: RouteEvent::RouteMaterialized {
                handle: RouteHandle {
                    stamp: jacquard_core::RouteIdentityStamp {
                        route_id: RouteId([1; 16]),
                        topology_epoch: RouteEpoch(2),
                        materialized_at_tick: Tick(6),
                        publication_id: PublicationId([1; 16]),
                    },
                },
                proof: RouteMaterializationProof {
                    stamp: jacquard_core::RouteIdentityStamp {
                        route_id: RouteId([1; 16]),
                        topology_epoch: RouteEpoch(2),
                        materialized_at_tick: Tick(6),
                        publication_id: PublicationId([1; 16]),
                    },
                    witness: Fact {
                        value: RouteWitness {
                            protection: jacquard_core::ObjectiveVsDelivered {
                                objective: RouteProtectionClass::LinkProtected,
                                delivered: RouteProtectionClass::LinkProtected,
                            },
                            connectivity: jacquard_core::ObjectiveVsDelivered {
                                objective: ConnectivityPosture {
                                    repair: RouteRepairClass::Repairable,
                                    partition: RoutePartitionClass::PartitionTolerant,
                                },
                                delivered: ConnectivityPosture {
                                    repair: RouteRepairClass::Repairable,
                                    partition: RoutePartitionClass::PartitionTolerant,
                                },
                            },
                            admission_profile: AdmissionAssumptions {
                                message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
                                failure_model: FailureModelClass::Benign,
                                runtime_envelope: RuntimeEnvelopeClass::Canonical,
                                node_density_class: NodeDensityClass::Moderate,
                                connectivity_regime: ConnectivityRegime::Stable,
                                adversary_regime: AdversaryRegime::Cooperative,
                                claim_strength: ClaimStrength::ConservativeUnderProfile,
                            },
                            topology_epoch: RouteEpoch(2),
                            degradation: RouteDegradation::None,
                        },
                        basis: FactBasis::Published,
                        established_at_tick: Tick(6),
                    },
                },
            },
        });

        assert!(projector.snapshot().active_routes.is_empty());
    }
}
