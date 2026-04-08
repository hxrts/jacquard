//! Concrete mesh/router wiring for the mock-device crate.
//!
//! Control flow intuition: the host/device assembles shared topology
//! observations, attaches one in-memory transport to the shared carrier, builds
//! a mesh engine behind the public traits, and hands that engine to the
//! router. The device remains observational with respect to canonical route
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
use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine};
use jacquard_mock_transport::{
    InMemoryMeshTransport, InMemoryRetentionStore, InMemoryRuntimeEffects,
    SharedInMemoryMeshNetwork,
};
use jacquard_router::{FixedPolicyEngine, SingleEngineRouter};
use jacquard_traits::Blake3Hashing;

use crate::MockDevice;

pub type MockMeshRouter = SingleEngineRouter<
    MeshEngine<
        DeterministicMeshTopologyModel,
        InMemoryMeshTransport,
        InMemoryRetentionStore,
        InMemoryRuntimeEffects,
        Blake3Hashing,
    >,
    FixedPolicyEngine,
    InMemoryRuntimeEffects,
>;

pub type MockMeshDevice = MockDevice<MockMeshRouter>;

pub fn build_mock_mesh_device(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryMeshNetwork,
    now: Tick,
) -> MockMeshDevice {
    build_mock_mesh_device_with_profile(
        local_node_id,
        topology,
        network,
        now,
        default_profile(),
    )
}

pub fn build_mock_mesh_device_with_profile(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryMeshNetwork,
    now: Tick,
    profile: AdaptiveRoutingProfile,
) -> MockMeshDevice {
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
    let router = SingleEngineRouter::new(
        engine,
        FixedPolicyEngine::new(profile),
        InMemoryRuntimeEffects { now, ..Default::default() },
        topology.clone(),
        policy_inputs_for(&topology, local_node_id),
    );
    MockDevice::new(topology, router)
}

impl MockDevice<MockMeshRouter> {
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
        .expect("mock mesh device requires at least one endpoint")
}

fn policy_inputs_for(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
) -> RoutingPolicyInputs {
    RoutingPolicyInputs {
        local_node:                  Observation {
            value:                 topology.value.nodes[&local_node_id].clone(),
            source_class:          topology.source_class,
            evidence_class:        topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick:      topology.observed_at_tick,
        },
        local_environment:           Observation {
            value:                 topology.value.environment.clone(),
            source_class:          topology.source_class,
            evidence_class:        topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick:      topology.observed_at_tick,
        },
        routing_engine_count:        1,
        median_rtt_ms:               DurationMs(40),
        loss_permille:               RatioPermille(50),
        partition_risk_permille:     RatioPermille(150),
        adversary_pressure_permille: RatioPermille(25),
        identity_assurance:          IdentityAssuranceClass::ControllerBound,
        direct_reachability_score:   HealthScore(900),
    }
}

fn default_profile() -> AdaptiveRoutingProfile {
    AdaptiveRoutingProfile {
        selected_protection:            RouteProtectionClass::LinkProtected,
        selected_connectivity:          RouteConnectivityProfile {
            repair:    RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile:             DeploymentProfile::FieldPartitionTolerant,
        diversity_floor:                1,
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy:       RouteReplacementPolicy::Allowed,
    }
}
