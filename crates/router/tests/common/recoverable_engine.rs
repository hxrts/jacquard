//! `RecoverableTestEngine` — a routing engine stub that owns mutable route
//! state behind a shared `BTreeSet`, used to exercise router recovery and
//! handoff logic.

use std::sync::{Arc, Mutex};

use jacquard_core::{
    Belief, ByteCount, Configuration, ConnectivityPosture, FactBasis, HealthScore,
    Observation, RatioPermille, RouteMaintenanceOutcome, RouteProtectionClass,
    RouteRepairClass, RoutingObjective, SelectedRoutingParameters, Tick, TimeWindow,
    TransportKind,
};

use super::fixtures::profile;

pub(crate) struct RecoverableTestEngine {
    local_node_id: jacquard_core::NodeId,
    shared_routes: Arc<Mutex<std::collections::BTreeSet<jacquard_core::RouteId>>>,
    now: Tick,
}

impl RecoverableTestEngine {
    pub(crate) fn new(
        local_node_id: jacquard_core::NodeId,
        shared_routes: Arc<Mutex<std::collections::BTreeSet<jacquard_core::RouteId>>>,
        now: Tick,
    ) -> Self {
        Self { local_node_id, shared_routes, now }
    }

    fn engine_id_value() -> jacquard_core::RoutingEngineId {
        jacquard_core::RoutingEngineId::from_contract_bytes([8; 16])
    }

    fn route_summary(
        &self,
        objective: &RoutingObjective,
    ) -> jacquard_core::RouteSummary {
        jacquard_core::RouteSummary {
            engine: Self::engine_id_value(),
            protection: objective.target_protection,
            connectivity: objective.target_connectivity,
            protocol_mix: vec![TransportKind::WifiAware],
            hop_count_hint: Belief::Estimated(jacquard_core::Estimate {
                value: 1,
                confidence_permille: RatioPermille(1000),
                updated_at_tick: self.now,
            }),
            valid_for: TimeWindow::new(self.now, Tick(self.now.0.saturating_add(8)))
                .expect("valid candidate window"),
        }
    }

    fn route_id() -> jacquard_core::RouteId {
        jacquard_core::RouteId([7; 16])
    }
}

impl jacquard_traits::RoutingEnginePlanner for RecoverableTestEngine {
    fn engine_id(&self) -> jacquard_core::RoutingEngineId {
        Self::engine_id_value()
    }

    fn capabilities(&self) -> jacquard_core::RoutingEngineCapabilities {
        jacquard_core::RoutingEngineCapabilities {
            engine: Self::engine_id_value(),
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::BestEffort,
                partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            repair_support: jacquard_core::RepairSupport::Unsupported,
            hold_support: jacquard_core::HoldSupport::Unsupported,
            decidable_admission: jacquard_core::DecidableSupport::Supported,
            quantitative_bounds:
                jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility: jacquard_core::RouteShapeVisibility::AggregatePath,
        }
    }

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<jacquard_core::RouteCandidate> {
        vec![jacquard_core::RouteCandidate {
            route_id: Self::route_id(),
            summary: self.route_summary(objective),
            estimate: jacquard_core::Estimate {
                value: jacquard_core::RouteEstimate {
                    estimated_protection: objective.target_protection,
                    estimated_connectivity: objective.target_connectivity,
                    topology_epoch: topology.value.epoch,
                    degradation: jacquard_core::RouteDegradation::None,
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: self.now,
            },
            backend_ref: jacquard_core::BackendRouteRef {
                engine: Self::engine_id_value(),
                backend_route_id: jacquard_core::BackendRouteId(vec![7]),
            },
        }]
    }

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: &jacquard_core::RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<jacquard_core::RouteAdmissionCheck, jacquard_core::RouteError> {
        self.admit_route(
            objective,
            &profile(),
            self.candidate_routes(objective, &profile(), topology)[0].clone(),
            topology,
        )
        .map(|admission| admission.admission_check)
    }

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: jacquard_core::RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<jacquard_core::RouteAdmission, jacquard_core::RouteError> {
        Ok(jacquard_core::RouteAdmission {
            backend_ref: candidate.backend_ref,
            objective: objective.clone(),
            profile: profile.clone(),
            admission_check: admissible_check(),
            summary: self.route_summary(objective),
            witness: admission_witness(objective, topology),
        })
    }
}

impl jacquard_traits::RoutingEngine for RecoverableTestEngine {
    fn materialize_route(
        &mut self,
        input: jacquard_core::RouteMaterializationInput,
    ) -> Result<jacquard_core::RouteInstallation, jacquard_core::RouteError> {
        self.shared_routes
            .lock()
            .expect("recoverable engine state")
            .insert(*input.handle.route_id());
        Ok(jacquard_core::RouteInstallation {
            materialization_proof: jacquard_core::RouteMaterializationProof {
                stamp: input.handle.stamp.clone(),
                witness: jacquard_core::Fact {
                    basis: FactBasis::Admitted,
                    value: input.admission.witness.clone(),
                    established_at_tick: self.now,
                },
            },
            last_lifecycle_event: jacquard_core::RouteLifecycleEvent::Activated,
            health: jacquard_core::RouteHealth {
                reachability_state: jacquard_core::ReachabilityState::Reachable,
                stability_score: HealthScore(1000),
                congestion_penalty_points: jacquard_core::PenaltyPoints(0),
                last_validated_at_tick: self.now,
            },
            progress: jacquard_core::RouteProgressContract {
                productive_step_count_max: jacquard_core::Limit::Bounded(1),
                total_step_count_max: jacquard_core::Limit::Bounded(1),
                last_progress_at_tick: self.now,
                state: jacquard_core::RouteProgressState::Pending,
            },
        })
    }

    fn route_commitments(
        &self,
        _route: &jacquard_core::MaterializedRoute,
    ) -> Vec<jacquard_core::RouteCommitment> {
        Vec::new()
    }

    fn maintain_route(
        &mut self,
        _identity: &jacquard_core::PublishedRouteRecord,
        _runtime: &mut jacquard_core::RouteRuntimeState,
        _trigger: jacquard_core::RouteMaintenanceTrigger,
    ) -> Result<jacquard_core::RouteMaintenanceResult, jacquard_core::RouteError> {
        Ok(jacquard_core::RouteMaintenanceResult {
            event: jacquard_core::RouteLifecycleEvent::Activated,
            outcome: RouteMaintenanceOutcome::Continued,
        })
    }

    fn teardown(&mut self, route_id: &jacquard_core::RouteId) {
        self.shared_routes
            .lock()
            .expect("recoverable engine state")
            .remove(route_id);
    }
}

impl jacquard_traits::RouterManagedEngine for RecoverableTestEngine {
    fn local_node_id_for_router(&self) -> jacquard_core::NodeId {
        self.local_node_id
    }

    fn forward_payload_for_router(
        &mut self,
        route_id: &jacquard_core::RouteId,
        _payload: &[u8],
    ) -> Result<(), jacquard_core::RouteError> {
        if self
            .shared_routes
            .lock()
            .expect("recoverable engine state")
            .contains(route_id)
        {
            Ok(())
        } else {
            Err(jacquard_core::RouteSelectionError::NoCandidate.into())
        }
    }

    fn restore_route_runtime_for_router(
        &mut self,
        route_id: &jacquard_core::RouteId,
    ) -> Result<bool, jacquard_core::RouteError> {
        Ok(self
            .shared_routes
            .lock()
            .expect("recoverable engine state")
            .contains(route_id))
    }
}

fn neutral_assumptions() -> jacquard_core::AdmissionAssumptions {
    jacquard_core::AdmissionAssumptions {
        message_flow_assumption: jacquard_core::MessageFlowAssumptionClass::BestEffort,
        failure_model: jacquard_core::FailureModelClass::Benign,
        runtime_envelope: jacquard_core::RuntimeEnvelopeClass::Canonical,
        node_density_class: jacquard_core::NodeDensityClass::Sparse,
        connectivity_regime: jacquard_core::ConnectivityRegime::Stable,
        adversary_regime: jacquard_core::AdversaryRegime::Cooperative,
        claim_strength: jacquard_core::ClaimStrength::ExactUnderAssumptions,
    }
}

fn admissible_check() -> jacquard_core::RouteAdmissionCheck {
    jacquard_core::RouteAdmissionCheck {
        decision: jacquard_core::AdmissionDecision::Admissible,
        profile: neutral_assumptions(),
        productive_step_bound: jacquard_core::Limit::Bounded(1),
        total_step_bound: jacquard_core::Limit::Bounded(1),
        route_cost: jacquard_core::RouteCost {
            message_count_max: jacquard_core::Limit::Bounded(1),
            byte_count_max: jacquard_core::Limit::Bounded(ByteCount(256)),
            hop_count: 1,
            repair_attempt_count_max: jacquard_core::Limit::Bounded(0),
            hold_bytes_reserved: jacquard_core::Limit::Bounded(ByteCount(0)),
            work_step_count_max: jacquard_core::Limit::Bounded(1),
        },
    }
}

fn admission_witness(
    objective: &RoutingObjective,
    topology: &Observation<Configuration>,
) -> jacquard_core::RouteWitness {
    jacquard_core::RouteWitness {
        protection: jacquard_core::ObjectiveVsDelivered {
            objective: objective.target_protection,
            delivered: objective.target_protection,
        },
        connectivity: jacquard_core::ObjectiveVsDelivered {
            objective: objective.target_connectivity,
            delivered: objective.target_connectivity,
        },
        admission_profile: neutral_assumptions(),
        topology_epoch: topology.value.epoch,
        degradation: jacquard_core::RouteDegradation::None,
    }
}
