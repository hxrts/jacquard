//! `OpaqueSummaryTestEngine` — a routing-engine stub used to prove the router
//! can host an external engine with opaque route summaries.
//!
//! The router only sees:
//! - `RouteShapeVisibility::Opaque`
//! - `hop_count_hint = Belief::Absent`
//! - one engine-owned backend ref
//!
//! It does not get a path, corridor, or next-hop disclosure.

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteId, BackendRouteRef,
    Belief, ByteCount, ClaimStrength, Configuration, ConnectivityPosture, ConnectivityRegime, Fact,
    FactBasis, FailureModelClass, HealthScore, Limit, MessageFlowAssumptionClass, NodeDensityClass,
    Observation, RatioPermille, RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCost,
    RouteDegradation, RouteEstimate, RouteHealth, RouteId, RouteInstallation, RouteLifecycleEvent,
    RouteMaintenanceOutcome, RouteMaintenanceResult, RouteMaintenanceTrigger,
    RouteMaterializationInput, RouteMaterializationProof, RoutePartitionClass,
    RouteProgressContract, RouteProgressState, RouteProtectionClass, RouteShapeVisibility,
    RouteSummary, RouteWitness, RoutingEngineCapabilities, RoutingEngineId, RoutingObjective,
    RoutingTickChange, RoutingTickContext, RoutingTickHint, RoutingTickOutcome,
    RuntimeEnvelopeClass, SelectedRoutingParameters, Tick, TimeWindow, TransportKind,
};
use jacquard_traits::{RouterManagedEngine, RoutingEngine, RoutingEnginePlanner};

pub(crate) struct OpaqueSummaryTestEngine {
    local_node_id: jacquard_core::NodeId,
    engine_id: RoutingEngineId,
    now: Tick,
}

impl OpaqueSummaryTestEngine {
    pub(crate) fn new(
        local_node_id: jacquard_core::NodeId,
        engine_id: RoutingEngineId,
        now: Tick,
    ) -> Self {
        Self {
            local_node_id,
            engine_id,
            now,
        }
    }

    fn route_id(&self) -> RouteId {
        let mut bytes = [0_u8; 16];
        bytes[..8].copy_from_slice(&self.engine_id.contract_id.0[..8]);
        bytes[8..].copy_from_slice(&self.local_node_id.0[..8]);
        RouteId(bytes)
    }

    fn route_summary(&self, objective: &RoutingObjective) -> RouteSummary {
        RouteSummary {
            engine: self.engine_id.clone(),
            protection: objective.target_protection,
            connectivity: objective.target_connectivity,
            protocol_mix: vec![TransportKind::Custom("opaque-test".to_owned())],
            hop_count_hint: Belief::Absent,
            valid_for: TimeWindow::new(self.now, Tick(self.now.0.saturating_add(8)))
                .expect("valid route summary window"),
        }
    }
}

impl RoutingEnginePlanner for OpaqueSummaryTestEngine {
    fn engine_id(&self) -> RoutingEngineId {
        self.engine_id.clone()
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        RoutingEngineCapabilities {
            engine: self.engine_id(),
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: ConnectivityPosture {
                repair: jacquard_core::RouteRepairClass::BestEffort,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            repair_support: jacquard_core::RepairSupport::Unsupported,
            hold_support: jacquard_core::HoldSupport::Unsupported,
            decidable_admission: jacquard_core::DecidableSupport::Supported,
            quantitative_bounds: jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility: RouteShapeVisibility::Opaque,
        }
    }

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        vec![RouteCandidate {
            route_id: self.route_id(),
            summary: self.route_summary(objective),
            estimate: jacquard_core::Estimate::certain(
                RouteEstimate {
                    estimated_protection: objective.target_protection,
                    estimated_connectivity: objective.target_connectivity,
                    topology_epoch: topology.value.epoch,
                    degradation: RouteDegradation::None,
                },
                self.now,
            ),
            backend_ref: BackendRouteRef {
                engine: self.engine_id(),
                backend_route_id: BackendRouteId(vec![0xAA, 0xBB, 0xCC]),
            },
        }]
    }

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, jacquard_core::RouteError> {
        self.admit_route(objective, profile, candidate.clone(), topology)
            .map(|admission| admission.admission_check)
    }

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, jacquard_core::RouteError> {
        Ok(RouteAdmission {
            backend_ref: candidate.backend_ref,
            objective: objective.clone(),
            profile: profile.clone(),
            admission_check: RouteAdmissionCheck {
                decision: AdmissionDecision::Admissible,
                profile: AdmissionAssumptions {
                    message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
                    failure_model: FailureModelClass::Benign,
                    runtime_envelope: RuntimeEnvelopeClass::Canonical,
                    node_density_class: NodeDensityClass::Sparse,
                    connectivity_regime: ConnectivityRegime::Stable,
                    adversary_regime: AdversaryRegime::Cooperative,
                    claim_strength: ClaimStrength::ConservativeUnderProfile,
                },
                productive_step_bound: Limit::Bounded(1),
                total_step_bound: Limit::Bounded(1),
                route_cost: RouteCost {
                    message_count_max: Limit::Bounded(1),
                    byte_count_max: Limit::Bounded(ByteCount(96)),
                    hop_count: 0,
                    repair_attempt_count_max: Limit::Bounded(0),
                    hold_bytes_reserved: Limit::Bounded(ByteCount(0)),
                    work_step_count_max: Limit::Bounded(1),
                },
            },
            summary: self.route_summary(objective),
            witness: RouteWitness {
                protection: jacquard_core::ObjectiveVsDelivered {
                    objective: objective.target_protection,
                    delivered: objective.target_protection,
                },
                connectivity: jacquard_core::ObjectiveVsDelivered {
                    objective: objective.target_connectivity,
                    delivered: objective.target_connectivity,
                },
                admission_profile: AdmissionAssumptions {
                    message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
                    failure_model: FailureModelClass::Benign,
                    runtime_envelope: RuntimeEnvelopeClass::Canonical,
                    node_density_class: NodeDensityClass::Sparse,
                    connectivity_regime: ConnectivityRegime::Stable,
                    adversary_regime: AdversaryRegime::Cooperative,
                    claim_strength: ClaimStrength::ConservativeUnderProfile,
                },
                topology_epoch: topology.value.epoch,
                degradation: RouteDegradation::None,
            },
        })
    }
}

impl RoutingEngine for OpaqueSummaryTestEngine {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, jacquard_core::RouteError> {
        Ok(RouteInstallation {
            materialization_proof: RouteMaterializationProof {
                stamp: input.handle.stamp.clone(),
                witness: Fact {
                    basis: FactBasis::Admitted,
                    value: input.admission.witness.clone(),
                    established_at_tick: self.now,
                },
            },
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: RouteHealth {
                reachability_state: jacquard_core::ReachabilityState::Reachable,
                stability_score: HealthScore(900),
                congestion_penalty_points: jacquard_core::PenaltyPoints(0),
                last_validated_at_tick: self.now,
            },
            progress: RouteProgressContract {
                productive_step_count_max: Limit::Bounded(1),
                total_step_count_max: Limit::Bounded(1),
                last_progress_at_tick: self.now,
                state: RouteProgressState::Pending,
            },
        })
    }

    fn route_commitments(
        &self,
        _route: &jacquard_core::MaterializedRoute,
    ) -> Vec<jacquard_core::RouteCommitment> {
        Vec::new()
    }

    fn engine_tick(
        &mut self,
        tick: &RoutingTickContext,
    ) -> Result<RoutingTickOutcome, jacquard_core::RouteError> {
        self.now = tick.topology.observed_at_tick;
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change: RoutingTickChange::PrivateStateUpdated,
            next_tick_hint: RoutingTickHint::Immediate,
        })
    }

    fn maintain_route(
        &mut self,
        _identity: &jacquard_core::PublishedRouteRecord,
        _runtime: &mut jacquard_core::RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, jacquard_core::RouteError> {
        Ok(RouteMaintenanceResult {
            event: RouteLifecycleEvent::Repaired,
            outcome: RouteMaintenanceOutcome::Continued,
        })
    }

    fn teardown(&mut self, _route_id: &RouteId) {}
}

impl RouterManagedEngine for OpaqueSummaryTestEngine {
    fn local_node_id_for_router(&self) -> jacquard_core::NodeId {
        self.local_node_id
    }

    fn forward_payload_for_router(
        &mut self,
        _route_id: &RouteId,
        _payload: &[u8],
    ) -> Result<(), jacquard_core::RouteError> {
        Ok(())
    }

    fn restore_route_runtime_for_router(
        &mut self,
        _route_id: &RouteId,
    ) -> Result<bool, jacquard_core::RouteError> {
        Ok(false)
    }
}
