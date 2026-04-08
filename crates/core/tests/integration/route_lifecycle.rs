//! Build a MaterializedRoute from router-owned identity and engine installation
//! state.

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteRef, Belief,
    ByteCount, ClaimStrength, ConnectivityPosture, ConnectivityRegime, DiversityFloor,
    Estimate, Fact, FactBasis, FailureModelClass, HoldFallbackPolicy, Limit,
    MaterializedRoute, MessageFlowAssumptionClass, NodeDensityClass,
    ObjectiveVsDelivered, OperatingMode, PublicationId, ReachabilityState,
    RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCost, RouteDegradation,
    RouteEpoch, RouteEstimate, RouteHandle, RouteHealth, RouteId, RouteInstallation,
    RouteLease, RouteLifecycleEvent, RouteMaterializationInput,
    RouteMaterializationProof, RoutePartitionClass, RouteProgressContract,
    RouteProgressState, RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy,
    RouteRuntimeError, RouteServiceKind, RouteSummary, RouteWitness,
    RoutingEngineFallbackPolicy, RoutingEngineId, RoutingObjective,
    RuntimeEnvelopeClass, SelectedRoutingParameters, Tick, TimeWindow,
    TransportProtocol,
};

fn repairable_connected() -> ConnectivityPosture {
    ConnectivityPosture {
        repair: RouteRepairClass::Repairable,
        partition: RoutePartitionClass::ConnectedOnly,
    }
}

fn sample_engine_id() -> RoutingEngineId {
    RoutingEngineId::from_contract_bytes([1; 16])
}

fn sample_objective() -> RoutingObjective {
    RoutingObjective {
        destination: jacquard_core::DestinationId::Node(jacquard_core::NodeId([7; 32])),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::None,
        target_connectivity: repairable_connected(),
        hold_fallback_policy: HoldFallbackPolicy::Allowed,
        latency_budget_ms: Limit::Bounded(jacquard_core::DurationMs(250)),
        protection_priority: jacquard_core::PriorityPoints(10),
        connectivity_priority: jacquard_core::PriorityPoints(20),
    }
}

fn sample_admission_assumptions() -> AdmissionAssumptions {
    AdmissionAssumptions {
        message_flow_assumption: MessageFlowAssumptionClass::PerRouteSequenced,
        failure_model: FailureModelClass::CrashStop,
        runtime_envelope: RuntimeEnvelopeClass::Canonical,
        node_density_class: NodeDensityClass::Moderate,
        connectivity_regime: ConnectivityRegime::Stable,
        adversary_regime: AdversaryRegime::Cooperative,
        claim_strength: ClaimStrength::ExactUnderAssumptions,
    }
}

fn sample_summary() -> RouteSummary {
    RouteSummary {
        engine: sample_engine_id(),
        protection: RouteProtectionClass::LinkProtected,
        connectivity: repairable_connected(),
        protocol_mix: vec![TransportProtocol::BleGatt, TransportProtocol::WifiLan],
        hop_count_hint: Belief::Estimated(Estimate {
            value: 3,
            confidence_permille: jacquard_core::RatioPermille(1000),
            updated_at_tick: Tick(100),
        }),
        valid_for: TimeWindow::new(Tick(100), Tick(500))
            .expect("valid route summary window"),
    }
}

fn sample_witness(admission_profile: AdmissionAssumptions) -> RouteWitness {
    RouteWitness {
        protection: ObjectiveVsDelivered {
            objective: RouteProtectionClass::LinkProtected,
            delivered: RouteProtectionClass::LinkProtected,
        },
        connectivity: ObjectiveVsDelivered {
            objective: repairable_connected(),
            delivered: repairable_connected(),
        },
        admission_profile,
        topology_epoch: RouteEpoch(4),
        degradation: RouteDegradation::None,
    }
}

fn sample_route_cost() -> RouteCost {
    RouteCost {
        message_count_max: Limit::Bounded(12),
        byte_count_max: Limit::Bounded(ByteCount(2048)),
        hop_count: 3,
        repair_attempt_count_max: Limit::Bounded(2),
        hold_bytes_reserved: Limit::Bounded(ByteCount(512)),
        work_step_count_max: Limit::Bounded(40),
    }
}

// long-block-exception: candidate/input/installation assembled together.
fn sample_route_parts() -> (RouteCandidate, RouteMaterializationInput, RouteInstallation)
{
    let objective = sample_objective();
    let admission_profile = sample_admission_assumptions();
    let summary = sample_summary();
    let witness = sample_witness(admission_profile.clone());
    let candidate = RouteCandidate {
        summary: summary.clone(),
        estimate: Estimate {
            value: RouteEstimate {
                estimated_protection: summary.protection,
                estimated_connectivity: summary.connectivity,
                topology_epoch: RouteEpoch(4),
                degradation: RouteDegradation::None,
            },
            confidence_permille: jacquard_core::RatioPermille(1000),
            updated_at_tick: Tick(100),
        },
        backend_ref: BackendRouteRef {
            engine: sample_engine_id(),
            backend_route_id: jacquard_core::BackendRouteId(vec![1, 2, 3]),
        },
    };
    let input = RouteMaterializationInput {
        handle: RouteHandle {
            stamp: jacquard_core::RouteIdentityStamp {
                route_id: RouteId([5; 16]),
                topology_epoch: RouteEpoch(4),
                materialized_at_tick: Tick(101),
                publication_id: PublicationId([4; 16]),
            },
        },
        admission: RouteAdmission {
            route_id: RouteId([5; 16]),
            backend_ref: candidate.backend_ref.clone(),
            objective,
            profile: SelectedRoutingParameters {
                selected_protection: RouteProtectionClass::LinkProtected,
                selected_connectivity: repairable_connected(),
                deployment_profile: OperatingMode::DenseInteractive,
                diversity_floor: DiversityFloor(1),
                routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
                route_replacement_policy: RouteReplacementPolicy::Allowed,
            },
            admission_check: RouteAdmissionCheck {
                decision: AdmissionDecision::Admissible,
                profile: admission_profile,
                productive_step_bound: Limit::Bounded(4),
                total_step_bound: Limit::Bounded(8),
                route_cost: sample_route_cost(),
            },
            summary,
            witness: witness.clone(),
        },
        lease: RouteLease {
            owner_node_id: jacquard_core::NodeId([9; 32]),
            lease_epoch: RouteEpoch(4),
            valid_for: TimeWindow::new(Tick(100), Tick(500))
                .expect("valid route lease window"),
        },
    };
    let installation = RouteInstallation {
        materialization_proof: RouteMaterializationProof {
            stamp: jacquard_core::RouteIdentityStamp {
                route_id: RouteId([5; 16]),
                topology_epoch: RouteEpoch(4),
                materialized_at_tick: Tick(101),
                publication_id: PublicationId([4; 16]),
            },
            witness: Fact {
                value: witness,
                basis: FactBasis::Published,
                established_at_tick: Tick(101),
            },
        },
        last_lifecycle_event: RouteLifecycleEvent::Activated,
        health: RouteHealth {
            reachability_state: ReachabilityState::Reachable,
            stability_score: jacquard_core::HealthScore(900),
            congestion_penalty_points: jacquard_core::PenaltyPoints(12),
            last_validated_at_tick: Tick(110),
        },
        progress: RouteProgressContract {
            productive_step_count_max: Limit::Bounded(6),
            total_step_count_max: Limit::Bounded(12),
            last_progress_at_tick: Tick(110),
            state: RouteProgressState::Satisfied,
        },
    };
    (candidate, input, installation)
}

fn sample_route() -> (RouteCandidate, MaterializedRoute) {
    let (candidate, input, installation) = sample_route_parts();
    let route = MaterializedRoute::from_installation(input, installation);

    (candidate, route)
}

#[test]
fn materialized_route_can_be_built_from_shared_lifecycle_types() {
    let (candidate, route) = sample_route();

    assert_eq!(candidate.summary.engine, sample_engine_id());
    assert_eq!(
        candidate.estimate.value.estimated_connectivity,
        repairable_connected(),
    );
    assert_eq!(route.identity.admission.summary.protocol_mix.len(), 2);
    assert_eq!(route.identity.stamp.route_id, RouteId([5; 16]));
    assert_eq!(
        route.identity.proof.witness.value.topology_epoch,
        RouteEpoch(4),
    );
    assert_eq!(
        route.identity.lease.owner_node_id,
        jacquard_core::NodeId([9; 32])
    );
    assert_eq!(
        route.runtime.last_lifecycle_event,
        RouteLifecycleEvent::Activated
    );
}

#[test]
#[should_panic(
    expected = "route installation proof stamp must match the canonical handle stamp"
)]
fn materialized_route_rejects_mismatched_installation_proof_identity() {
    let (_, input, mut installation) = sample_route_parts();
    installation.materialization_proof.stamp.route_id = RouteId([6; 16]);

    let _ = MaterializedRoute::from_installation(input, installation);
}

#[test]
#[should_panic(
    expected = "route installation requires an admissible control-plane decision"
)]
fn materialized_route_rejects_inadmissible_activation() {
    let (_, mut input, installation) = sample_route_parts();
    input.admission.admission_check.decision = AdmissionDecision::Rejected(
        jacquard_core::RouteAdmissionRejection::CapacityExceeded,
    );

    let _ = MaterializedRoute::from_installation(input, installation);
}

#[test]
#[should_panic(
    expected = "route installation must satisfy the objective protection floor"
)]
fn materialized_route_rejects_activation_below_protection_floor() {
    let (_, mut input, installation) = sample_route_parts();
    input.admission.objective.protection_floor =
        RouteProtectionClass::TopologyProtected;
    input.admission.summary.protection = RouteProtectionClass::LinkProtected;

    let _ = MaterializedRoute::from_installation(input, installation);
}

#[test]
fn expired_lease_is_rejected_before_publication_or_maintenance() {
    let (_, route) = sample_route();

    assert_eq!(
        route.identity.ensure_lease_valid_at(Tick(500)),
        Err(RouteRuntimeError::LeaseExpired)
    );
}
