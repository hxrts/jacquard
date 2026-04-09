//! Concrete client builders that pair a `MultiEngineRouter` with one or
//! more routing engines. `build_pathway_client` registers a single mesh
//! engine. `build_pathway_batman_client` registers both mesh and batman.
//! Each builder attaches an `InMemoryTransport` to the shared network,
//! wires up the engine instances, and returns a `PathwayClient`.

use jacquard_batman::BatmanEngine;
use jacquard_core::{
    Configuration, ConnectivityPosture, DiversityFloor, DurationMs, HealthScore,
    IdentityAssuranceClass, LinkEndpoint, NodeId, Observation, OperatingMode,
    RatioPermille, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
    RouteReplacementPolicy, RoutingEngineFallbackPolicy, RoutingPolicyInputs,
    SelectedRoutingParameters, Tick,
};
use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport,
    SharedInMemoryNetwork,
};
use jacquard_pathway::{DeterministicPathwayTopologyModel, PathwayEngine};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::Blake3Hashing;

use crate::Client;

pub type PathwayRouter = MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects>;

pub type PathwayClient = Client<PathwayRouter>;

pub fn build_pathway_client(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
) -> PathwayClient {
    build_pathway_client_with_profile(
        local_node_id,
        topology,
        network,
        now,
        default_profile(),
    )
}

pub fn build_pathway_client_with_profile(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
    profile: SelectedRoutingParameters,
) -> PathwayClient {
    let local_endpoint = local_endpoint(&topology, local_node_id);
    let transport = InMemoryTransport::attach(local_node_id, [local_endpoint], network);

    let engine = PathwayEngine::without_committee_selector(
        local_node_id,
        DeterministicPathwayTopologyModel::new(),
        transport,
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects { now, ..Default::default() },
        Blake3Hashing,
    );
    let mut router = MultiEngineRouter::new(
        local_node_id,
        FixedPolicyEngine::new(profile),
        InMemoryRuntimeEffects { now, ..Default::default() },
        topology.clone(),
        policy_inputs_for(&topology, local_node_id),
    );
    router
        .register_engine(Box::new(engine))
        .expect("register mesh engine");
    Client::new(topology, router)
}

pub fn build_pathway_batman_client(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
) -> PathwayClient {
    build_pathway_batman_client_with_profile(
        local_node_id,
        topology,
        network,
        now,
        default_profile(),
    )
}

pub fn build_pathway_batman_client_with_profile(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
    profile: SelectedRoutingParameters,
) -> PathwayClient {
    let local_endpoint = local_endpoint(&topology, local_node_id);
    let mesh_transport = InMemoryTransport::attach(
        local_node_id,
        [local_endpoint.clone()],
        network.clone(),
    );
    let batman_transport =
        InMemoryTransport::attach(local_node_id, [local_endpoint], network);

    let mesh_engine = PathwayEngine::without_committee_selector(
        local_node_id,
        DeterministicPathwayTopologyModel::new(),
        mesh_transport,
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects { now, ..Default::default() },
        Blake3Hashing,
    );
    let batman_engine = BatmanEngine::new(
        local_node_id,
        batman_transport,
        InMemoryRuntimeEffects { now, ..Default::default() },
    );

    let mut router = MultiEngineRouter::new(
        local_node_id,
        FixedPolicyEngine::new(profile),
        InMemoryRuntimeEffects { now, ..Default::default() },
        topology.clone(),
        policy_inputs_for(&topology, local_node_id),
    );
    router
        .register_engine(Box::new(mesh_engine))
        .expect("register mesh engine");
    router
        .register_engine(Box::new(batman_engine))
        .expect("register batman engine");
    Client::new(topology, router)
}

impl Client<PathwayRouter> {
    pub fn replace_shared_topology(&mut self, topology: Observation<Configuration>) {
        self.router.ingest_topology_observation(topology.clone());
        self.topology = topology;
    }
}

fn local_endpoint(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
) -> LinkEndpoint {
    topology.value.nodes[&local_node_id]
        .profile
        .endpoints
        .first()
        .cloned()
        .expect("reference topology must provide at least one local endpoint")
}

fn policy_inputs_for(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
) -> RoutingPolicyInputs {
    RoutingPolicyInputs {
        local_node: Observation {
            value: topology.value.nodes[&local_node_id].clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        local_environment: Observation {
            value: topology.value.environment.clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        routing_engine_count: 1,
        median_rtt_ms: DurationMs(40),
        loss_permille: RatioPermille(50),
        partition_risk_permille: RatioPermille(150),
        adversary_pressure_permille: RatioPermille(25),
        identity_assurance: IdentityAssuranceClass::ControllerBound,
        direct_reachability_score: HealthScore(900),
    }
}

fn default_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile: OperatingMode::FieldPartitionTolerant,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}
