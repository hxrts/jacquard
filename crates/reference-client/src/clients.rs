//! Concrete reference-client bridge builders.
//!
//! This module provides the primary developer-facing client construction path:
//! [`ClientBuilder`]. It assembles a complete host-side client from its
//! constituent parts: a `MultiEngineRouter`, one or more routing engines
//! (`PathwayEngine`, `BatmanEngine`), an in-memory transport driver, and
//! queue-backed sender capabilities for each engine. The result is a
//! `HostBridge` that owns the transport attachment and drives the router
//! through synchronous rounds.
//!
//! The legacy `build_*client*` functions remain as thin wrappers over the
//! builder for tests that still prefer function-style setup.
//!
//! `PathwayRouter` and `PathwayClient` are type aliases exported for use by
//! integration tests that need to name the bridge type concretely.

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
    bridge::{BridgeQueueConfig, BridgeTransport},
    defaults::DEFAULT_BRIDGE_QUEUE_CONFIG,
    HostBridge,
};

pub type PathwayRouter = MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects>;
pub type PathwayClient = HostBridge<PathwayRouter>;

#[derive(Clone)]
pub struct ClientBuildOptions {
    pub local_node_id: NodeId,
    pub topology: Observation<Configuration>,
    pub network: SharedInMemoryNetwork,
    pub now: Tick,
    pub profile: SelectedRoutingParameters,
    pub queue_config: BridgeQueueConfig,
}

impl ClientBuildOptions {
    #[must_use]
    pub fn new(
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
        }
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
}

#[derive(Clone)]
pub struct ClientBuilder {
    options: ClientBuildOptions,
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
        Self::from_options(ClientBuildOptions::new(
            local_node_id,
            topology,
            network,
            now,
        ))
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
    pub fn from_options(options: ClientBuildOptions) -> Self {
        Self { options, include_batman: false }
    }

    #[must_use]
    pub fn with_profile(mut self, profile: SelectedRoutingParameters) -> Self {
        self.options = self.options.with_profile(profile);
        self
    }

    #[must_use]
    pub fn with_queue_config(mut self, queue_config: BridgeQueueConfig) -> Self {
        self.options = self.options.with_queue_config(queue_config);
        self
    }

    #[must_use]
    pub fn with_batman(mut self) -> Self {
        self.include_batman = true;
        self
    }

    #[must_use]
    pub fn build(self) -> PathwayClient {
        let local_endpoint =
            local_endpoint(&self.options.topology, self.options.local_node_id);
        let driver = InMemoryTransport::attach(
            self.options.local_node_id,
            [local_endpoint],
            self.options.network,
        );
        let transport =
            BridgeTransport::with_queue_config(driver, self.options.queue_config);
        let pathway_sender = transport.sender();

        let pathway_engine = PathwayEngine::without_committee_selector(
            self.options.local_node_id,
            DeterministicPathwayTopologyModel::new(),
            pathway_sender,
            InMemoryRetentionStore::default(),
            InMemoryRuntimeEffects {
                now: self.options.now,
                ..Default::default()
            },
            Blake3Hashing,
        );

        let mut router = MultiEngineRouter::new(
            self.options.local_node_id,
            FixedPolicyEngine::new(self.options.profile),
            InMemoryRuntimeEffects {
                now: self.options.now,
                ..Default::default()
            },
            self.options.topology.clone(),
            policy_inputs_for(&self.options.topology, self.options.local_node_id),
        );
        router
            .register_engine(Box::new(pathway_engine))
            .expect("register pathway engine");

        if self.include_batman {
            let batman_engine = BatmanEngine::new(
                self.options.local_node_id,
                transport.sender(),
                InMemoryRuntimeEffects {
                    now: self.options.now,
                    ..Default::default()
                },
            );
            router
                .register_engine(Box::new(batman_engine))
                .expect("register batman engine");
        }

        HostBridge::from_transport(
            self.options.topology,
            router,
            transport,
            self.options.queue_config,
        )
    }
}

#[doc(hidden)]
pub fn build_pathway_client(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
) -> PathwayClient {
    ClientBuilder::pathway(local_node_id, topology, network, now).build()
}

#[doc(hidden)]
pub fn build_pathway_client_with_profile(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
    profile: SelectedRoutingParameters,
) -> PathwayClient {
    ClientBuilder::pathway(local_node_id, topology, network, now)
        .with_profile(profile)
        .build()
}

#[doc(hidden)]
pub fn build_pathway_batman_client(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
) -> PathwayClient {
    ClientBuilder::pathway_and_batman(local_node_id, topology, network, now).build()
}

#[doc(hidden)]
pub fn build_pathway_batman_client_with_profile(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
    profile: SelectedRoutingParameters,
) -> PathwayClient {
    ClientBuilder::pathway_and_batman(local_node_id, topology, network, now)
        .with_profile(profile)
        .build()
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        Environment, FactSourceClass, OriginAuthenticationClass, RouteEpoch,
        RoutingEvidenceClass,
    };

    use super::*;
    use crate::{topology, BridgeRoundProgress};

    fn sample_topology(local_node_id: NodeId) -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(1),
                nodes: BTreeMap::from([(
                    local_node_id,
                    topology::node(1).pathway().build(),
                )]),
                links: BTreeMap::new(),
                environment: Environment {
                    reachable_neighbor_count: 0,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        }
    }

    #[test]
    fn client_builder_constructs_waiting_pathway_bridge() {
        let local_node_id = NodeId([1; 32]);
        let topology = sample_topology(local_node_id);
        let network = SharedInMemoryNetwork::default();
        let mut client =
            ClientBuilder::pathway(local_node_id, topology, network, Tick(1)).build();
        let mut bound = client.bind();

        let progress = bound.advance_round().expect("advance initial round");

        match progress {
            | BridgeRoundProgress::Advanced(report) => {
                assert_eq!(report.router_outcome.topology_epoch, RouteEpoch(1));
            },
            | BridgeRoundProgress::Waiting(_) => {},
        }
    }

    #[test]
    fn client_builder_accepts_explicit_queue_config_and_profile() {
        let local_node_id = NodeId([1; 32]);
        let options = ClientBuildOptions::new(
            local_node_id,
            sample_topology(local_node_id),
            SharedInMemoryNetwork::default(),
            Tick(1),
        )
        .with_profile(default_profile())
        .with_queue_config(BridgeQueueConfig::new(1, 1));

        let mut client = ClientBuilder::from_options(options).with_batman().build();
        let mut bound = client.bind();

        let progress = bound.advance_round().expect("advance initial round");

        match progress {
            | BridgeRoundProgress::Advanced(report) => {
                assert_eq!(report.router_outcome.topology_epoch, RouteEpoch(1));
            },
            | BridgeRoundProgress::Waiting(_) => {},
        }
    }
}
