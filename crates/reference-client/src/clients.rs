//! Concrete reference-client bridge builders.
//!
//! [`ClientBuilder`] is the intended path: choose a topology, optionally add
//! BATMAN, Babel, or override queue/policy settings, then build one bridge-owned
//! client.

use jacquard_babel::{BabelEngine, DecayWindow as BabelDecayWindow};
use jacquard_batman::{BatmanEngine, DecayWindow};
use jacquard_core::{
    Configuration, ConnectivityPosture, DestinationId, DiversityFloor, DurationMs, HealthScore,
    IdentityAssuranceClass, LinkEndpoint, NodeId, Observation, OperatingMode, RatioPermille,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy,
    RoutingEngineFallbackPolicy, RoutingPolicyInputs, SelectedRoutingParameters, Tick,
};
use jacquard_field::{FieldEngine, FieldForwardSummaryObservation, FieldSearchConfig};
use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport, SharedInMemoryNetwork,
};
use jacquard_pathway::{DeterministicPathwayTopologyModel, PathwayEngine, PathwaySearchConfig};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::Blake3Hashing;

use crate::{
    bridge::{BridgeQueueConfig, BridgeTransport, DEFAULT_BRIDGE_QUEUE_CONFIG},
    HostBridge,
};

pub type ReferenceRouter = MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects>;
pub type ReferenceClient = HostBridge<ReferenceRouter>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldBootstrapSummary {
    pub destination: DestinationId,
    pub from_neighbor: NodeId,
    pub forward_observation: FieldForwardSummaryObservation,
    pub reverse_feedback: Option<(u16, Tick)>,
}

#[derive(Clone)]
pub struct ClientBuilder {
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
    profile: SelectedRoutingParameters,
    policy_inputs: Option<RoutingPolicyInputs>,
    batman_decay_window: Option<DecayWindow>,
    pathway_search_config: Option<PathwaySearchConfig>,
    field_search_config: Option<FieldSearchConfig>,
    field_bootstrap_summaries: Vec<FieldBootstrapSummary>,
    babel_decay_window: Option<BabelDecayWindow>,
    queue_config: BridgeQueueConfig,
    include_pathway: bool,
    include_batman: bool,
    include_babel: bool,
    include_field: bool,
}

impl ClientBuilder {
    fn with_engine_set(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
        include_pathway: bool,
        include_batman: bool,
        include_babel: bool,
        include_field: bool,
    ) -> Self {
        Self {
            local_node_id,
            topology,
            network,
            now,
            profile: default_profile_for_engine_set(
                include_pathway,
                include_batman,
                include_babel,
                include_field,
            ),
            policy_inputs: None,
            batman_decay_window: None,
            babel_decay_window: None,
            pathway_search_config: None,
            field_search_config: None,
            field_bootstrap_summaries: Vec::new(),
            queue_config: DEFAULT_BRIDGE_QUEUE_CONFIG,
            include_pathway,
            include_batman,
            include_babel,
            include_field,
        }
    }

    #[must_use]
    pub fn pathway(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::with_engine_set(
            local_node_id,
            topology,
            network,
            now,
            true,
            false,
            false,
            false,
        )
    }

    #[must_use]
    pub fn batman(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::with_engine_set(
            local_node_id,
            topology,
            network,
            now,
            false,
            true,
            false,
            false,
        )
    }

    #[must_use]
    pub fn babel(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::with_engine_set(
            local_node_id,
            topology,
            network,
            now,
            false,
            false,
            true,
            false,
        )
    }

    #[must_use]
    pub fn field(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::with_engine_set(
            local_node_id,
            topology,
            network,
            now,
            false,
            false,
            false,
            true,
        )
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
    pub fn pathway_and_field(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::pathway(local_node_id, topology, network, now).with_field()
    }

    #[must_use]
    pub fn field_and_batman(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::field(local_node_id, topology, network, now).with_batman()
    }

    #[must_use]
    pub fn all_engines(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::pathway(local_node_id, topology, network, now)
            .with_field()
            .with_batman()
    }

    #[must_use]
    pub fn with_profile(mut self, profile: SelectedRoutingParameters) -> Self {
        self.profile = profile;
        self
    }

    #[must_use]
    pub fn with_policy_inputs(mut self, policy_inputs: RoutingPolicyInputs) -> Self {
        self.policy_inputs = Some(policy_inputs);
        self
    }

    #[must_use]
    pub fn with_pathway_search_config(mut self, search_config: PathwaySearchConfig) -> Self {
        self.pathway_search_config = Some(search_config);
        self
    }

    #[must_use]
    pub fn with_field_search_config(mut self, search_config: FieldSearchConfig) -> Self {
        self.field_search_config = Some(search_config);
        self
    }

    #[must_use]
    pub fn with_field_bootstrap_summary(mut self, bootstrap: FieldBootstrapSummary) -> Self {
        self.field_bootstrap_summaries.push(bootstrap);
        self
    }

    #[must_use]
    pub fn with_queue_config(mut self, queue_config: BridgeQueueConfig) -> Self {
        self.queue_config = queue_config;
        self
    }

    #[must_use]
    pub fn babel_and_batman(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::babel(local_node_id, topology, network, now).with_batman()
    }

    #[must_use]
    pub fn pathway_and_babel(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::pathway(local_node_id, topology, network, now).with_babel()
    }

    #[must_use]
    pub fn with_batman_decay_window(mut self, decay_window: DecayWindow) -> Self {
        self.batman_decay_window = Some(decay_window);
        self
    }

    #[must_use]
    pub fn with_babel_decay_window(mut self, decay_window: BabelDecayWindow) -> Self {
        self.babel_decay_window = Some(decay_window);
        self
    }

    #[must_use]
    pub fn with_batman(mut self) -> Self {
        self.include_batman = true;
        self
    }

    #[must_use]
    pub fn with_babel(mut self) -> Self {
        self.include_babel = true;
        self
    }

    #[must_use]
    pub fn with_field(mut self) -> Self {
        self.include_field = true;
        self
    }

    #[must_use]
    // long-block-exception: the reference client builder wires a single
    // bridge-owned host from the chosen engine set in one place so mixed-engine
    // simulator and test setups stay deterministic.
    pub fn build(self) -> ReferenceClient {
        let local_endpoint = local_endpoint(&self.topology, self.local_node_id);
        let driver = InMemoryTransport::attach(self.local_node_id, [local_endpoint], self.network);
        let transport = BridgeTransport::with_queue_config(driver, self.queue_config);
        let pathway_sender = transport.sender();

        let mut router = MultiEngineRouter::new(
            self.local_node_id,
            FixedPolicyEngine::new(self.profile),
            InMemoryRuntimeEffects {
                now: self.now,
                ..Default::default()
            },
            self.topology.clone(),
            self.policy_inputs
                .unwrap_or_else(|| policy_inputs_for(&self.topology, self.local_node_id)),
        );
        if self.include_pathway {
            let pathway_engine = PathwayEngine::without_committee_selector(
                self.local_node_id,
                DeterministicPathwayTopologyModel::new(),
                pathway_sender,
                InMemoryRetentionStore::default(),
                InMemoryRuntimeEffects {
                    now: self.now,
                    ..Default::default()
                },
                Blake3Hashing,
            );
            let pathway_engine = if let Some(search_config) = self.pathway_search_config.clone() {
                pathway_engine.with_search_config(search_config)
            } else {
                pathway_engine
            };
            router
                .register_engine(Box::new(pathway_engine))
                .expect("register pathway engine");
        }

        if self.include_batman {
            let batman_engine = if let Some(decay_window) = self.batman_decay_window {
                BatmanEngine::with_decay_window(
                    self.local_node_id,
                    transport.sender(),
                    InMemoryRuntimeEffects {
                        now: self.now,
                        ..Default::default()
                    },
                    decay_window,
                )
            } else {
                BatmanEngine::new(
                    self.local_node_id,
                    transport.sender(),
                    InMemoryRuntimeEffects {
                        now: self.now,
                        ..Default::default()
                    },
                )
            };
            router
                .register_engine(Box::new(batman_engine))
                .expect("register batman engine");
        }

        if self.include_babel {
            let babel_engine = if let Some(decay_window) = self.babel_decay_window {
                BabelEngine::with_decay_window(
                    self.local_node_id,
                    transport.sender(),
                    InMemoryRuntimeEffects {
                        now: self.now,
                        ..Default::default()
                    },
                    decay_window,
                )
            } else {
                BabelEngine::new(
                    self.local_node_id,
                    transport.sender(),
                    InMemoryRuntimeEffects {
                        now: self.now,
                        ..Default::default()
                    },
                )
            };
            router
                .register_engine(Box::new(babel_engine))
                .expect("register babel engine");
        }

        if self.include_field {
            let field_engine = FieldEngine::new(
                self.local_node_id,
                transport.sender(),
                InMemoryRuntimeEffects {
                    now: self.now,
                    ..Default::default()
                },
            );
            let mut field_engine = if let Some(search_config) = self.field_search_config.clone() {
                field_engine.with_search_config(search_config)
            } else {
                field_engine
            };
            for bootstrap in &self.field_bootstrap_summaries {
                field_engine.record_forward_summary(
                    &bootstrap.destination,
                    bootstrap.from_neighbor,
                    bootstrap.forward_observation,
                );
                if let Some((delivery_feedback, observed_at_tick)) = bootstrap.reverse_feedback {
                    field_engine.record_reverse_feedback(
                        &bootstrap.destination,
                        bootstrap.from_neighbor,
                        delivery_feedback,
                        observed_at_tick,
                    );
                }
            }
            router
                .register_engine(Box::new(field_engine))
                .expect("register field engine");
        }

        HostBridge::from_transport(self.topology, router, transport, self.queue_config)
    }
}

fn local_endpoint(topology: &Observation<Configuration>, local_node_id: NodeId) -> LinkEndpoint {
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

fn batman_default_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: OperatingMode::SparseLowPower,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

fn default_profile_for_engine_set(
    include_pathway: bool,
    include_batman: bool,
    include_babel: bool,
    include_field: bool,
) -> SelectedRoutingParameters {
    if (include_batman || include_babel) && !include_pathway && !include_field {
        batman_default_profile()
    } else {
        default_profile()
    }
}
