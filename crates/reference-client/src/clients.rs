//! Concrete reference-client bridge builders.
//!
//! [`ClientBuilder`] is the intended path: choose a topology, optionally add
//! BATMAN, Babel, or override queue/policy settings, then build one bridge-owned
//! client.

use std::collections::BTreeSet;

use jacquard_babel::{BabelEngine, DecayWindow as BabelDecayWindow};
use jacquard_batman_bellman::{BatmanBellmanEngine, DecayWindow};
use jacquard_batman_classic::{BatmanClassicEngine, DecayWindow as ClassicDecayWindow};
use jacquard_core::{
    Configuration, ConnectivityPosture, DestinationId, DiversityFloor, DurationMs, HealthScore,
    IdentityAssuranceClass, LinkEndpoint, NodeId, Observation, OperatingMode, RatioPermille,
    RouteError, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
    RouteReplacementPolicy, RoutingEngineFallbackPolicy, RoutingPolicyInputs,
    SelectedRoutingParameters, Tick,
};
use jacquard_field::{FieldEngine, FieldForwardSummaryObservation, FieldSearchConfig};
use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport, SharedInMemoryNetwork,
};
use jacquard_mercator::MercatorEngine;
use jacquard_olsrv2::{DecayWindow as OlsrV2DecayWindow, OlsrV2Engine};
use jacquard_pathway::{DeterministicPathwayTopologyModel, PathwayEngine, PathwaySearchConfig};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_scatter::{ScatterEngine, ScatterEngineConfig};
use jacquard_traits::Blake3Hashing;
use thiserror::Error;

use crate::{
    bridge::{BridgeQueueConfig, BridgeTransport, DEFAULT_BRIDGE_QUEUE_CONFIG},
    HostBridge,
};

pub type ReferenceRouter = MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects>;
pub type ReferenceClient = HostBridge<ReferenceRouter>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EngineKind {
    Pathway,
    BatmanBellman,
    BatmanClassic,
    OlsrV2,
    Babel,
    Field,
    Scatter,
    Mercator,
}

impl EngineKind {
    const CANONICAL_REGISTRATION_ORDER: [Self; 8] = [
        Self::Pathway,
        Self::BatmanBellman,
        Self::BatmanClassic,
        Self::OlsrV2,
        Self::Babel,
        Self::Field,
        Self::Scatter,
        Self::Mercator,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Pathway => "pathway",
            Self::BatmanBellman => "batman-bellman",
            Self::BatmanClassic => "batman-classic",
            Self::OlsrV2 => "olsrv2",
            Self::Babel => "babel",
            Self::Field => "field",
            Self::Scatter => "scatter",
            Self::Mercator => "mercator",
        }
    }
}

#[derive(Debug, Error)]
pub enum ReferenceClientBuildError {
    #[error("failed to register {engine} engine in reference client builder: {source}")]
    EngineRegistration {
        engine: &'static str,
        #[source]
        source: RouteError,
    },
}

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
    batman_bellman_decay_window: Option<DecayWindow>,
    batman_classic_decay_window: Option<ClassicDecayWindow>,
    olsrv2_decay_window: Option<OlsrV2DecayWindow>,
    pathway_search_config: Option<PathwaySearchConfig>,
    field_search_config: Option<FieldSearchConfig>,
    scatter_config: Option<ScatterEngineConfig>,
    field_bootstrap_summaries: Vec<FieldBootstrapSummary>,
    babel_decay_window: Option<BabelDecayWindow>,
    queue_config: BridgeQueueConfig,
    engines: BTreeSet<EngineKind>,
}

impl ClientBuilder {
    fn with_engine_set(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
        engines: BTreeSet<EngineKind>,
    ) -> Self {
        Self {
            local_node_id,
            topology,
            network,
            now,
            profile: default_profile_for_engine_set(&engines),
            policy_inputs: None,
            batman_bellman_decay_window: None,
            batman_classic_decay_window: None,
            olsrv2_decay_window: None,
            babel_decay_window: None,
            pathway_search_config: None,
            field_search_config: None,
            scatter_config: None,
            field_bootstrap_summaries: Vec::new(),
            queue_config: DEFAULT_BRIDGE_QUEUE_CONFIG,
            engines,
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
            singleton_engine(EngineKind::Pathway),
        )
    }

    #[must_use]
    pub fn batman_bellman(
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
            singleton_engine(EngineKind::BatmanBellman),
        )
    }

    #[must_use]
    pub fn batman_classic(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        let mut builder = Self::with_engine_set(
            local_node_id,
            topology,
            network,
            now,
            singleton_engine(EngineKind::BatmanClassic),
        );
        builder.profile = batman_default_profile();
        builder
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
            singleton_engine(EngineKind::Babel),
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
            singleton_engine(EngineKind::Field),
        )
    }

    #[must_use]
    pub fn scatter(
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
            singleton_engine(EngineKind::Scatter),
        )
    }

    #[must_use]
    pub fn mercator(
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
            singleton_engine(EngineKind::Mercator),
        )
    }

    #[must_use]
    pub fn olsrv2(
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
            singleton_engine(EngineKind::OlsrV2),
        )
    }

    #[must_use]
    pub fn pathway_and_batman_bellman(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::pathway(local_node_id, topology, network, now).with_batman_bellman()
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
    pub fn field_and_batman_bellman(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::field(local_node_id, topology, network, now).with_batman_bellman()
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
            .with_batman_bellman()
            .with_batman_classic()
            .with_babel()
            .with_olsrv2()
            .with_scatter()
            .with_mercator()
            .with_queue_config(BridgeQueueConfig::new(320, 320))
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
    pub fn with_scatter_config(mut self, scatter_config: ScatterEngineConfig) -> Self {
        self.scatter_config = Some(scatter_config);
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
    pub fn babel_and_batman_bellman(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::babel(local_node_id, topology, network, now).with_batman_bellman()
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
    pub fn pathway_and_olsrv2(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::pathway(local_node_id, topology, network, now).with_olsrv2()
    }

    #[must_use]
    pub fn olsrv2_and_batman_bellman(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self::olsrv2(local_node_id, topology, network, now).with_batman_bellman()
    }

    #[must_use]
    pub fn with_batman_bellman_decay_window(mut self, decay_window: DecayWindow) -> Self {
        self.batman_bellman_decay_window = Some(decay_window);
        self
    }

    #[must_use]
    pub fn with_olsrv2_decay_window(mut self, decay_window: OlsrV2DecayWindow) -> Self {
        self.olsrv2_decay_window = Some(decay_window);
        self
    }

    #[must_use]
    pub fn with_babel_decay_window(mut self, decay_window: BabelDecayWindow) -> Self {
        self.babel_decay_window = Some(decay_window);
        self
    }

    #[must_use]
    pub fn with_batman_bellman(mut self) -> Self {
        self.engines.insert(EngineKind::BatmanBellman);
        self
    }

    #[must_use]
    pub fn with_batman_classic(mut self) -> Self {
        self.engines.insert(EngineKind::BatmanClassic);
        self
    }

    #[must_use]
    pub fn with_batman_classic_decay_window(mut self, decay_window: ClassicDecayWindow) -> Self {
        self.batman_classic_decay_window = Some(decay_window);
        self
    }

    #[must_use]
    pub fn with_babel(mut self) -> Self {
        self.engines.insert(EngineKind::Babel);
        self
    }

    #[must_use]
    pub fn with_olsrv2(mut self) -> Self {
        self.engines.insert(EngineKind::OlsrV2);
        self
    }

    #[must_use]
    pub fn with_field(mut self) -> Self {
        self.engines.insert(EngineKind::Field);
        self
    }

    #[must_use]
    pub fn with_scatter(mut self) -> Self {
        self.engines.insert(EngineKind::Scatter);
        self
    }

    #[must_use]
    pub fn with_mercator(mut self) -> Self {
        self.engines.insert(EngineKind::Mercator);
        self
    }

    // long-block-exception: the reference client builder wires a single
    // bridge-owned host from the chosen engine set in one place so mixed-engine
    // simulator and test setups stay deterministic.
    pub fn build(self) -> Result<ReferenceClient, ReferenceClientBuildError> {
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
        for engine in EngineKind::CANONICAL_REGISTRATION_ORDER {
            if !self.engines.contains(&engine) {
                continue;
            }
            match engine {
                EngineKind::Pathway => {
                    let pathway_engine = PathwayEngine::without_committee_selector(
                        self.local_node_id,
                        DeterministicPathwayTopologyModel::new(),
                        pathway_sender.clone(),
                        InMemoryRetentionStore::default(),
                        InMemoryRuntimeEffects {
                            now: self.now,
                            ..Default::default()
                        },
                        Blake3Hashing,
                    );
                    let pathway_engine =
                        if let Some(search_config) = self.pathway_search_config.clone() {
                            pathway_engine.with_search_config(search_config)
                        } else {
                            pathway_engine
                        };
                    router
                        .register_engine(Box::new(pathway_engine))
                        .map_err(|source| ReferenceClientBuildError::EngineRegistration {
                            engine: engine.label(),
                            source,
                        })?;
                }
                EngineKind::BatmanBellman => {
                    let batman_engine = if let Some(decay_window) = self.batman_bellman_decay_window
                    {
                        BatmanBellmanEngine::with_decay_window(
                            self.local_node_id,
                            transport.sender(),
                            InMemoryRuntimeEffects {
                                now: self.now,
                                ..Default::default()
                            },
                            decay_window,
                        )
                    } else {
                        BatmanBellmanEngine::new(
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
                        .map_err(|source| ReferenceClientBuildError::EngineRegistration {
                            engine: engine.label(),
                            source,
                        })?;
                }
                EngineKind::BatmanClassic => {
                    let classic_engine =
                        if let Some(decay_window) = self.batman_classic_decay_window {
                            BatmanClassicEngine::with_decay_window(
                                self.local_node_id,
                                transport.sender(),
                                InMemoryRuntimeEffects {
                                    now: self.now,
                                    ..Default::default()
                                },
                                decay_window,
                            )
                        } else {
                            BatmanClassicEngine::new(
                                self.local_node_id,
                                transport.sender(),
                                InMemoryRuntimeEffects {
                                    now: self.now,
                                    ..Default::default()
                                },
                            )
                        };
                    router
                        .register_engine(Box::new(classic_engine))
                        .map_err(|source| ReferenceClientBuildError::EngineRegistration {
                            engine: engine.label(),
                            source,
                        })?;
                }
                EngineKind::OlsrV2 => {
                    let olsrv2_engine = if let Some(decay_window) = self.olsrv2_decay_window {
                        OlsrV2Engine::with_decay_window(
                            self.local_node_id,
                            transport.sender(),
                            InMemoryRuntimeEffects {
                                now: self.now,
                                ..Default::default()
                            },
                            decay_window,
                        )
                    } else {
                        OlsrV2Engine::new(
                            self.local_node_id,
                            transport.sender(),
                            InMemoryRuntimeEffects {
                                now: self.now,
                                ..Default::default()
                            },
                        )
                    };
                    router
                        .register_engine(Box::new(olsrv2_engine))
                        .map_err(|source| ReferenceClientBuildError::EngineRegistration {
                            engine: engine.label(),
                            source,
                        })?;
                }
                EngineKind::Scatter => {
                    let scatter_engine = if let Some(scatter_config) = self.scatter_config {
                        ScatterEngine::with_config(
                            self.local_node_id,
                            transport.sender(),
                            InMemoryRuntimeEffects {
                                now: self.now,
                                ..Default::default()
                            },
                            scatter_config,
                        )
                    } else {
                        ScatterEngine::new(
                            self.local_node_id,
                            transport.sender(),
                            InMemoryRuntimeEffects {
                                now: self.now,
                                ..Default::default()
                            },
                        )
                    };
                    router
                        .register_engine(Box::new(scatter_engine))
                        .map_err(|source| ReferenceClientBuildError::EngineRegistration {
                            engine: engine.label(),
                            source,
                        })?;
                }
                EngineKind::Babel => {
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
                        .map_err(|source| ReferenceClientBuildError::EngineRegistration {
                            engine: engine.label(),
                            source,
                        })?;
                }
                EngineKind::Field => {
                    let field_engine = FieldEngine::new(
                        self.local_node_id,
                        transport.sender(),
                        InMemoryRuntimeEffects {
                            now: self.now,
                            ..Default::default()
                        },
                    );
                    let mut field_engine =
                        if let Some(search_config) = self.field_search_config.clone() {
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
                        if let Some((delivery_feedback, observed_at_tick)) =
                            bootstrap.reverse_feedback
                        {
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
                        .map_err(|source| ReferenceClientBuildError::EngineRegistration {
                            engine: engine.label(),
                            source,
                        })?;
                }
                EngineKind::Mercator => {
                    router
                        .register_engine(Box::new(MercatorEngine::new(self.local_node_id)))
                        .map_err(|source| ReferenceClientBuildError::EngineRegistration {
                            engine: engine.label(),
                            source,
                        })?;
                }
            }
        }

        Ok(HostBridge::from_transport(
            self.topology,
            router,
            transport,
            self.queue_config,
        ))
    }
}

fn singleton_engine(engine: EngineKind) -> BTreeSet<EngineKind> {
    BTreeSet::from([engine])
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

fn default_profile_for_engine_set(engines: &BTreeSet<EngineKind>) -> SelectedRoutingParameters {
    let next_hop_only = engines.contains(&EngineKind::BatmanBellman)
        || engines.contains(&EngineKind::BatmanClassic)
        || engines.contains(&EngineKind::Babel)
        || engines.contains(&EngineKind::OlsrV2);
    let mixed_or_corridor =
        engines.contains(&EngineKind::Pathway) || engines.contains(&EngineKind::Field);
    if next_hop_only && !mixed_or_corridor {
        batman_default_profile()
    } else {
        default_profile()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use jacquard_babel::BABEL_ENGINE_ID;
    use jacquard_batman_bellman::BATMAN_BELLMAN_ENGINE_ID;
    use jacquard_batman_classic::BATMAN_CLASSIC_ENGINE_ID;
    use jacquard_core::{
        Configuration, Environment, FactSourceClass, Observation, OriginAuthenticationClass,
        RatioPermille, RouteEpoch, RoutingEvidenceClass, Tick,
    };
    use jacquard_field::FIELD_ENGINE_ID;
    use jacquard_mercator::MERCATOR_ENGINE_ID;
    use jacquard_olsrv2::OLSRV2_ENGINE_ID;
    use jacquard_pathway::PATHWAY_ENGINE_ID;
    use jacquard_scatter::SCATTER_ENGINE_ID;

    use super::{
        batman_default_profile, default_profile, default_profile_for_engine_set, ClientBuilder,
        EngineKind,
    };
    use crate::SharedInMemoryNetwork;
    use jacquard_testkit::topology;

    fn sample_topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(1),
                nodes: BTreeMap::from([(
                    jacquard_core::NodeId([1; 32]),
                    topology::node(1).all_engines().build(),
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
    fn default_profile_prefers_next_hop_defaults_only_for_pure_next_hop_sets() {
        assert_eq!(
            default_profile_for_engine_set(&BTreeSet::from([EngineKind::BatmanBellman])),
            batman_default_profile()
        );
        assert_eq!(
            default_profile_for_engine_set(&BTreeSet::from([
                EngineKind::BatmanClassic,
                EngineKind::Babel,
            ])),
            batman_default_profile()
        );
        assert_eq!(
            default_profile_for_engine_set(&BTreeSet::from([
                EngineKind::Pathway,
                EngineKind::BatmanBellman,
            ])),
            default_profile()
        );
        assert_eq!(
            default_profile_for_engine_set(&BTreeSet::from([EngineKind::Field])),
            default_profile()
        );
    }

    #[test]
    fn all_engines_builder_registers_full_membership_without_workarounds() {
        let mut client = ClientBuilder::all_engines(
            jacquard_core::NodeId([1; 32]),
            sample_topology(),
            SharedInMemoryNetwork::default(),
            Tick(1),
        )
        .build()
        .expect("build all-engines client");
        let registered = client.bind().router().registered_engine_ids();

        assert!(registered.contains(&PATHWAY_ENGINE_ID));
        assert!(registered.contains(&BATMAN_BELLMAN_ENGINE_ID));
        assert!(registered.contains(&BATMAN_CLASSIC_ENGINE_ID));
        assert!(registered.contains(&BABEL_ENGINE_ID));
        assert!(registered.contains(&OLSRV2_ENGINE_ID));
        assert!(registered.contains(&FIELD_ENGINE_ID));
        assert!(registered.contains(&SCATTER_ENGINE_ID));
        assert!(registered.contains(&MERCATOR_ENGINE_ID));
    }

    #[test]
    fn mercator_builder_registers_explicit_engine() {
        let mut client = ClientBuilder::mercator(
            jacquard_core::NodeId([1; 32]),
            sample_topology(),
            SharedInMemoryNetwork::default(),
            Tick(1),
        )
        .build()
        .expect("build mercator client");
        let registered = client.bind().router().registered_engine_ids();

        assert_eq!(registered, vec![MERCATOR_ENGINE_ID]);
    }
}
