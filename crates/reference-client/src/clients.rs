//! Concrete reference-client bridge builders.
//!
//! [`ClientBuilder`] is the intended path: choose a topology, optionally add
//! BATMAN or override queue/policy settings, then build one bridge-owned
//! client.

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

use crate::{
    bridge::{BridgeQueueConfig, BridgeTransport, DEFAULT_BRIDGE_QUEUE_CONFIG},
    HostBridge,
};

pub type PathwayRouter = MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects>;
pub type PathwayClient = HostBridge<PathwayRouter>;

#[derive(Clone)]
pub struct ClientBuilder {
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
    profile: SelectedRoutingParameters,
    queue_config: BridgeQueueConfig,
    include_batman: bool,
}

impl ClientBuilder {
    #[must_use]
    pub fn pathway(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self {
            local_node_id,
            topology,
            network,
            now,
            profile: default_profile(),
            queue_config: DEFAULT_BRIDGE_QUEUE_CONFIG,
            include_batman: false,
        }
    }

    #[must_use]
    pub fn pathway_and_batman(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::pathway(local_node_id, topology, network, now).with_batman()
    }

    #[must_use]
    pub fn with_profile(mut self, profile: SelectedRoutingParameters) -> Self {
        self.profile = profile;
        self
    }

    #[must_use]
    pub fn with_queue_config(mut self, queue_config: BridgeQueueConfig) -> Self {
        self.queue_config = queue_config;
        self
    }

    #[must_use]
    pub fn with_batman(mut self) -> Self {
        self.include_batman = true;
        self
    }

    #[must_use]
    pub fn build(self) -> PathwayClient {
        let local_endpoint = local_endpoint(&self.topology, self.local_node_id);
        let driver = InMemoryTransport::attach(
            self.local_node_id,
            [local_endpoint],
            self.network,
        );
        let transport = BridgeTransport::with_queue_config(driver, self.queue_config);
        let pathway_sender = transport.sender();

        let pathway_engine = PathwayEngine::without_committee_selector(
            self.local_node_id,
            DeterministicPathwayTopologyModel::new(),
            pathway_sender,
            InMemoryRetentionStore::default(),
            InMemoryRuntimeEffects { now: self.now, ..Default::default() },
            Blake3Hashing,
        );

        let mut router = MultiEngineRouter::new(
            self.local_node_id,
            FixedPolicyEngine::new(self.profile),
            InMemoryRuntimeEffects { now: self.now, ..Default::default() },
            self.topology.clone(),
            policy_inputs_for(&self.topology, self.local_node_id),
        );
        router
            .register_engine(Box::new(pathway_engine))
            .expect("register pathway engine");

        if self.include_batman {
            let batman_engine = BatmanEngine::new(
                self.local_node_id,
                transport.sender(),
                InMemoryRuntimeEffects { now: self.now, ..Default::default() },
            );
            router
                .register_engine(Box::new(batman_engine))
                .expect("register batman engine");
        }

        HostBridge::from_transport(self.topology, router, transport, self.queue_config)
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

// Extracts per-node and environment facts from the shared topology observation,
// using reference defaults for RTT, loss, and adversary pressure.
pub(crate) fn policy_inputs_for(
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

#[cfg(test)]
pub(crate) fn policy_inputs_for_empty(local_node_id: NodeId) -> RoutingPolicyInputs {
    let empty_node = jacquard_mem_node_profile::NodePreset::route_capable(
        jacquard_mem_node_profile::NodePresetOptions::new(
            jacquard_mem_node_profile::NodeIdentity::new(
                local_node_id,
                jacquard_core::ControllerId(local_node_id.0),
            ),
            jacquard_adapter::opaque_endpoint(
                jacquard_core::TransportKind::Custom("reference".to_owned()),
                vec![0],
                jacquard_core::ByteCount(64),
            ),
            Tick(1),
        ),
        &jacquard_pathway::PATHWAY_ENGINE_ID,
    )
    .build();
    let topology = Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(1),
            nodes: std::collections::BTreeMap::from([(local_node_id, empty_node)]),
            links: std::collections::BTreeMap::new(),
            environment: jacquard_core::Environment {
                reachable_neighbor_count: 0,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: jacquard_core::FactSourceClass::Local,
        evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    };
    policy_inputs_for(&topology, local_node_id)
}

pub(crate) fn default_profile() -> SelectedRoutingParameters {
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
