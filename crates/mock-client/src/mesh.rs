//! Concrete mesh/router wiring for the mock-client crate.
//!
//! Control flow intuition: the host/client assembles shared topology
//! observations, attaches one in-memory transport to the shared carrier, builds
//! a mesh engine behind the public traits, and hands that engine to the
//! router. The client remains observational with respect to canonical route
//! truth; only the router publishes the canonical route table.
//!
//! Ownership:
//! - local composition only
//! - no canonical route publication outside the router-owned path

use jacquard_core::{
    AdaptiveRoutingProfile, Configuration, DeploymentProfile, DurationMs, HealthScore,
    IdentityAssuranceClass, LinkEndpoint, NodeId, Observation, RatioPermille,
    RouteConnectivityProfile, RoutePartitionClass, RouteProtectionClass,
    RouteRepairClass, RouteReplacementPolicy, RoutingEngineFallbackPolicy,
    RoutingPolicyInputs, Tick,
};
use jacquard_mem_link_profile::{
    InMemoryMeshTransport, InMemoryRetentionStore, InMemoryRuntimeEffects,
    SharedInMemoryNetwork,
};
use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::Blake3Hashing;

use crate::Client;

pub type MeshRouter = MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects>;

pub type MeshClient = Client<MeshRouter>;

pub fn build_mesh_client(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
) -> MeshClient {
    build_mesh_client_with_profile(
        local_node_id,
        topology,
        network,
        now,
        default_profile(),
    )
}

pub fn build_mesh_client_with_profile(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
    profile: AdaptiveRoutingProfile,
) -> MeshClient {
    let local_endpoint = local_endpoint(&topology, local_node_id);
    let mut transport = InMemoryMeshTransport::attached(
        local_endpoint.protocol.clone(),
        local_node_id,
        [local_endpoint],
        network,
    );
    transport.set_ingress_tick(now);

    let engine = MeshEngine::without_committee_selector(
        local_node_id,
        DeterministicMeshTopologyModel::new(),
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

impl Client<MeshRouter> {
    pub fn replace_shared_topology(&mut self, topology: Observation<Configuration>) {
        self.router.replace_topology(topology.clone());
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
        .unwrap_or_else(|| jacquard_mem_link_profile::ble_endpoint(local_node_id.0[0]))
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

fn default_profile() -> AdaptiveRoutingProfile {
    AdaptiveRoutingProfile {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: RouteConnectivityProfile {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile: DeploymentProfile::FieldPartitionTolerant,
        diversity_floor: 1,
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}
