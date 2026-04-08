//! Shared contract checks for the router-facing middleware traits.

use std::collections::BTreeMap;

use jacquard_traits::{
    jacquard_core::{
        Configuration, ConnectivityPosture, Environment, NodeId, Observation,
        RouteCommitment, RouteError, RouteId, RouteMaintenanceResult,
        RouteMaintenanceTrigger, RouteProtectionClass, RouteRuntimeState,
        RoutingEngineCapabilities, RoutingEngineId,
        RoutingObjective, RoutingPolicyInputs, RoutingTickContext,
        RoutingTickOutcome, SelectedRoutingParameters, Tick,
    },
    RouterEngineRegistry, RouterManagedEngine, RoutingEngine,
    RoutingEnginePlanner, RoutingMiddleware,
};

struct StubManagedEngine {
    local_node_id: NodeId,
    engine_id: RoutingEngineId,
    restored: bool,
}

impl StubManagedEngine {
    fn new(local_node_id: NodeId, engine_id: RoutingEngineId) -> Self {
        Self {
            local_node_id,
            engine_id,
            restored: false,
        }
    }
}

impl RoutingEnginePlanner for StubManagedEngine {
    fn engine_id(&self) -> RoutingEngineId {
        self.engine_id.clone()
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        RoutingEngineCapabilities {
            engine: self.engine_id.clone(),
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: ConnectivityPosture {
                repair: jacquard_traits::jacquard_core::RouteRepairClass::BestEffort,
                partition: jacquard_traits::jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            repair_support: jacquard_traits::jacquard_core::RepairSupport::Unsupported,
            hold_support: jacquard_traits::jacquard_core::HoldSupport::Unsupported,
            decidable_admission: jacquard_traits::jacquard_core::DecidableSupport::Supported,
            quantitative_bounds:
                jacquard_traits::jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support:
                jacquard_traits::jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility:
                jacquard_traits::jacquard_core::RouteShapeVisibility::Opaque,
        }
    }

    fn candidate_routes(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Configuration>,
    ) -> Vec<jacquard_traits::jacquard_core::RouteCandidate> {
        Vec::new()
    }

    fn check_candidate(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: &jacquard_traits::jacquard_core::RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<jacquard_traits::jacquard_core::RouteAdmissionCheck, RouteError> {
        Err(jacquard_traits::jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn admit_route(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: jacquard_traits::jacquard_core::RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<jacquard_traits::jacquard_core::RouteAdmission, RouteError> {
        Err(jacquard_traits::jacquard_core::RouteSelectionError::NoCandidate.into())
    }
}

impl RoutingEngine for StubManagedEngine {
    fn materialize_route(
        &mut self,
        _input: jacquard_traits::jacquard_core::RouteMaterializationInput,
    ) -> Result<jacquard_traits::jacquard_core::RouteInstallation, RouteError> {
        Err(jacquard_traits::jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn route_commitments(
        &self,
        _route: &jacquard_traits::jacquard_core::MaterializedRoute,
    ) -> Vec<RouteCommitment> {
        Vec::new()
    }

    fn engine_tick(
        &mut self,
        tick: &RoutingTickContext,
    ) -> Result<RoutingTickOutcome, RouteError> {
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change: jacquard_traits::jacquard_core::RoutingTickChange::NoChange,
        })
    }

    fn maintain_route(
        &mut self,
        _identity: &jacquard_traits::jacquard_core::MaterializedRouteIdentity,
        _runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        Err(jacquard_traits::jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn teardown(&mut self, _route_id: &RouteId) {}
}

impl RouterManagedEngine for StubManagedEngine {
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id
    }

    fn forward_payload_for_router(
        &mut self,
        _route_id: &RouteId,
        _payload: &[u8],
    ) -> Result<(), RouteError> {
        Ok(())
    }

    fn restore_route_runtime_for_router(
        &mut self,
        _route_id: &RouteId,
    ) -> Result<bool, RouteError> {
        self.restored = true;
        Ok(true)
    }
}

struct StubMiddleware {
    topology: Observation<Configuration>,
    inputs: RoutingPolicyInputs,
    engines: BTreeMap<RoutingEngineId, RoutingEngineCapabilities>,
    recovered_count: usize,
}

impl StubMiddleware {
    fn new(topology: Observation<Configuration>, inputs: RoutingPolicyInputs) -> Self {
        Self {
            topology,
            inputs,
            engines: BTreeMap::new(),
            recovered_count: 0,
        }
    }
}

impl RouterEngineRegistry for StubMiddleware {
    fn register_engine(
        &mut self,
        extension: Box<dyn RouterManagedEngine>,
    ) -> Result<(), RouteError> {
        self.engines
            .insert(extension.engine_id(), extension.capabilities());
        Ok(())
    }

    fn registered_engine_ids(&self) -> Vec<RoutingEngineId> {
        self.engines.keys().cloned().collect()
    }

    fn registered_engine_capabilities(
        &self,
        engine_id: &RoutingEngineId,
    ) -> Option<RoutingEngineCapabilities> {
        self.engines.get(engine_id).cloned()
    }
}

impl RoutingMiddleware for StubMiddleware {
    fn replace_topology(&mut self, topology: Observation<Configuration>) {
        self.topology = topology;
    }

    fn replace_policy_inputs(&mut self, inputs: RoutingPolicyInputs) {
        self.inputs = inputs;
    }

    fn recover_checkpointed_routes(&mut self) -> Result<usize, RouteError> {
        self.recovered_count = self.recovered_count.saturating_add(1);
        Ok(self.recovered_count)
    }
}

#[test]
fn router_engine_registry_tracks_shared_engine_metadata() {
    let topology = sample_topology();
    let inputs = sample_policy_inputs();
    let mut middleware = StubMiddleware::new(topology, inputs);
    let engine_id = RoutingEngineId::External {
        name: "stub".to_string(),
        contract_id: jacquard_traits::jacquard_core::RoutingEngineContractId([9; 16]),
    };

    middleware
        .register_engine(Box::new(StubManagedEngine::new(
            NodeId([1; 32]),
            engine_id.clone(),
        )))
        .expect("register engine");

    assert_eq!(middleware.registered_engine_ids(), vec![engine_id.clone()]);
    assert_eq!(
        middleware
            .registered_engine_capabilities(&engine_id)
            .expect("registered capabilities")
            .engine,
        engine_id,
    );
}

#[test]
fn routing_middleware_updates_topology_policy_inputs_and_recovery_state() {
    let topology = sample_topology();
    let inputs = sample_policy_inputs();
    let mut middleware = StubMiddleware::new(topology.clone(), inputs.clone());
    let mut next_topology = topology;
    next_topology.value.environment.reachable_neighbor_count = 7;
    let mut next_inputs = inputs;
    next_inputs.routing_engine_count = 3;

    middleware.replace_topology(next_topology.clone());
    middleware.replace_policy_inputs(next_inputs.clone());

    assert_eq!(
        middleware.topology.value.environment.reachable_neighbor_count,
        7,
    );
    assert_eq!(middleware.inputs.routing_engine_count, 3);
    assert_eq!(
        middleware
            .recover_checkpointed_routes()
            .expect("recover checkpointed routes"),
        1,
    );
}

fn sample_topology() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_traits::jacquard_core::RouteEpoch(1),
            nodes: BTreeMap::new(),
            links: BTreeMap::new(),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: jacquard_traits::jacquard_core::RatioPermille(100),
                contention_permille: jacquard_traits::jacquard_core::RatioPermille(50),
            },
        },
        source_class: jacquard_traits::jacquard_core::FactSourceClass::Local,
        evidence_class:
            jacquard_traits::jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication:
            jacquard_traits::jacquard_core::OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    }
}

fn sample_policy_inputs() -> RoutingPolicyInputs {
    RoutingPolicyInputs {
        local_node: sample_node_observation(),
        local_environment: Observation {
            value: Environment {
                reachable_neighbor_count: 2,
                churn_permille: jacquard_traits::jacquard_core::RatioPermille(100),
                contention_permille: jacquard_traits::jacquard_core::RatioPermille(50),
            },
            source_class: jacquard_traits::jacquard_core::FactSourceClass::Local,
            evidence_class:
                jacquard_traits::jacquard_core::RoutingEvidenceClass::DirectObservation,
            origin_authentication:
                jacquard_traits::jacquard_core::OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        },
        routing_engine_count: 1,
        median_rtt_ms: jacquard_traits::jacquard_core::DurationMs(40),
        loss_permille: jacquard_traits::jacquard_core::RatioPermille(50),
        partition_risk_permille: jacquard_traits::jacquard_core::RatioPermille(25),
        adversary_pressure_permille: jacquard_traits::jacquard_core::RatioPermille(25),
        identity_assurance:
            jacquard_traits::jacquard_core::IdentityAssuranceClass::ControllerBound,
        direct_reachability_score: jacquard_traits::jacquard_core::HealthScore(900),
    }
}

fn sample_node_observation(
) -> Observation<jacquard_traits::jacquard_core::Node> {
    Observation {
        value: jacquard_traits::jacquard_core::Node {
            controller_id: jacquard_traits::jacquard_core::ControllerId([1; 32]),
            profile: jacquard_traits::jacquard_core::NodeProfile {
                services: Vec::new(),
                endpoints: Vec::new(),
                connection_count_max: 0,
                neighbor_state_count_max: 0,
                simultaneous_transfer_count_max: 0,
                active_route_count_max: 0,
                relay_work_budget_max: RelayWorkBudget(0),
                maintenance_work_budget_max: MaintenanceWorkBudget(0),
                hold_item_count_max: HoldItemCount(0),
                hold_capacity_bytes_max: jacquard_traits::jacquard_core::ByteCount(0),
            },
            state: jacquard_traits::jacquard_core::NodeState {
                relay_budget: jacquard_traits::jacquard_core::Belief::Absent,
                available_connection_count:
                    jacquard_traits::jacquard_core::Belief::Absent,
                hold_capacity_available_bytes:
                    jacquard_traits::jacquard_core::Belief::Absent,
                information_summary: jacquard_traits::jacquard_core::Belief::Absent,
            },
        },
        source_class: jacquard_traits::jacquard_core::FactSourceClass::Local,
        evidence_class:
            jacquard_traits::jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication:
            jacquard_traits::jacquard_core::OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    }
}
