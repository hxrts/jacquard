//! Scenario definition: topology, hosts, objectives, engine lanes, and overrides.

use jacquard_babel::DecayWindow as BabelDecayWindow;
use jacquard_batman_bellman::DecayWindow;
use jacquard_batman_classic::DecayWindow as ClassicDecayWindow;
use jacquard_core::{
    Configuration, NodeId, Observation, OperatingMode, RoutingObjective, RoutingPolicyInputs,
    SelectedRoutingParameters, SimulationSeed,
};
use jacquard_core::{DestinationId, Tick};
use jacquard_field::{FieldForwardSummaryObservation, FieldSearchConfig};
use jacquard_olsrv2::DecayWindow as OlsrV2DecayWindow;
use jacquard_pathway::PathwaySearchConfig;
use jacquard_scatter::ScatterEngineConfig;
use jacquard_traits::RoutingScenario;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineLane {
    Pathway,
    BatmanBellman,
    BatmanClassic,
    Babel,
    OlsrV2,
    Field,
    Scatter,
    PathwayAndBatmanBellman,
    PathwayAndBabel,
    PathwayAndOlsrV2,
    BabelAndBatmanBellman,
    OlsrV2AndBatmanBellman,
    PathwayAndField,
    FieldAndBatmanBellman,
    AllEngines,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostSpec {
    pub local_node_id: NodeId,
    pub lane: EngineLane,
    pub overrides: HostOverrides,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostOverrides {
    pub routing_profile: Option<SelectedRoutingParameters>,
    pub policy_inputs: Option<RoutingPolicyInputs>,
    pub batman_bellman_decay_window: Option<DecayWindow>,
    pub batman_classic_decay_window: Option<ClassicDecayWindow>,
    pub babel_decay_window: Option<BabelDecayWindow>,
    pub olsrv2_decay_window: Option<OlsrV2DecayWindow>,
    pub pathway_search_config: Option<PathwaySearchConfig>,
    pub field_search_config: Option<FieldSearchConfig>,
    pub scatter_config: Option<ScatterEngineConfig>,
    pub field_bootstrap_summaries: Vec<FieldBootstrapSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldBootstrapSummary {
    pub destination: DestinationId,
    pub from_neighbor: NodeId,
    pub forward_observation: FieldForwardSummaryObservation,
    pub reverse_feedback: Option<(u16, Tick)>,
}

impl FieldBootstrapSummary {
    #[must_use]
    pub fn new(
        destination: DestinationId,
        from_neighbor: NodeId,
        forward_observation: FieldForwardSummaryObservation,
    ) -> Self {
        Self {
            destination,
            from_neighbor,
            forward_observation,
            reverse_feedback: None,
        }
    }

    #[must_use]
    pub fn with_reverse_feedback(mut self, delivery_feedback: u16, observed_at_tick: Tick) -> Self {
        self.reverse_feedback = Some((delivery_feedback, observed_at_tick));
        self
    }
}

impl HostSpec {
    #[must_use]
    pub fn pathway(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::Pathway,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn batman_bellman(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::BatmanBellman,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn batman_classic(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::BatmanClassic,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn babel(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::Babel,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn olsrv2(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::OlsrV2,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn field(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::Field,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn scatter(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::Scatter,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn pathway_and_batman_bellman(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::PathwayAndBatmanBellman,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn pathway_and_babel(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::PathwayAndBabel,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn pathway_and_olsrv2(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::PathwayAndOlsrV2,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn babel_and_batman_bellman(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::BabelAndBatmanBellman,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn olsrv2_and_batman_bellman(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::OlsrV2AndBatmanBellman,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn pathway_and_field(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::PathwayAndField,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn field_and_batman_bellman(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::FieldAndBatmanBellman,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn all_engines(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::AllEngines,
            overrides: HostOverrides::default(),
        }
    }

    #[must_use]
    pub fn with_profile(mut self, routing_profile: SelectedRoutingParameters) -> Self {
        self.overrides.routing_profile = Some(routing_profile);
        self
    }

    #[must_use]
    pub fn with_policy_inputs(mut self, policy_inputs: RoutingPolicyInputs) -> Self {
        self.overrides.policy_inputs = Some(policy_inputs);
        self
    }

    #[must_use]
    pub fn with_batman_bellman_decay_window(
        mut self,
        batman_bellman_decay_window: DecayWindow,
    ) -> Self {
        self.overrides.batman_bellman_decay_window = Some(batman_bellman_decay_window);
        self
    }

    #[must_use]
    pub fn with_batman_classic_decay_window(
        mut self,
        batman_classic_decay_window: ClassicDecayWindow,
    ) -> Self {
        self.overrides.batman_classic_decay_window = Some(batman_classic_decay_window);
        self
    }

    #[must_use]
    pub fn with_babel_decay_window(mut self, babel_decay_window: BabelDecayWindow) -> Self {
        self.overrides.babel_decay_window = Some(babel_decay_window);
        self
    }

    #[must_use]
    pub fn with_olsrv2_decay_window(mut self, olsrv2_decay_window: OlsrV2DecayWindow) -> Self {
        self.overrides.olsrv2_decay_window = Some(olsrv2_decay_window);
        self
    }

    #[must_use]
    pub fn with_pathway_search_config(
        mut self,
        pathway_search_config: PathwaySearchConfig,
    ) -> Self {
        self.overrides.pathway_search_config = Some(pathway_search_config);
        self
    }

    #[must_use]
    pub fn with_field_search_config(mut self, field_search_config: FieldSearchConfig) -> Self {
        self.overrides.field_search_config = Some(field_search_config);
        self
    }

    #[must_use]
    pub fn with_scatter_config(mut self, scatter_config: ScatterEngineConfig) -> Self {
        self.overrides.scatter_config = Some(scatter_config);
        self
    }

    #[must_use]
    pub fn with_field_bootstrap_summary(
        mut self,
        field_bootstrap_summary: FieldBootstrapSummary,
    ) -> Self {
        self.overrides
            .field_bootstrap_summaries
            .push(field_bootstrap_summary);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundObjective {
    pub owner_node_id: NodeId,
    pub objective: RoutingObjective,
    pub activate_at_round: u32,
}

impl BoundObjective {
    #[must_use]
    pub fn new(owner_node_id: NodeId, objective: RoutingObjective) -> Self {
        Self {
            owner_node_id,
            objective,
            activate_at_round: 0,
        }
    }

    #[must_use]
    pub fn with_activation_round(mut self, activate_at_round: u32) -> Self {
        self.activate_at_round = activate_at_round;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostTopologyLag {
    pub local_node_id: NodeId,
    pub start_round: u32,
    pub end_round_inclusive: u32,
    pub lag_rounds: u32,
}

impl HostTopologyLag {
    #[must_use]
    pub fn new(
        local_node_id: NodeId,
        start_round: u32,
        end_round_inclusive: u32,
        lag_rounds: u32,
    ) -> Self {
        Self {
            local_node_id,
            start_round,
            end_round_inclusive,
            lag_rounds,
        }
    }

    #[must_use]
    pub fn applies_at(&self, local_node_id: NodeId, round_index: u32) -> bool {
        self.local_node_id == local_node_id
            && round_index >= self.start_round
            && round_index <= self.end_round_inclusive
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JacquardScenario {
    name: String,
    seed: SimulationSeed,
    deployment_profile: OperatingMode,
    initial_configuration: Observation<Configuration>,
    hosts: Vec<HostSpec>,
    objectives: Vec<RoutingObjective>,
    bound_objectives: Vec<BoundObjective>,
    round_limit: u32,
    checkpoint_period_rounds: Option<u32>,
    topology_lags: Vec<HostTopologyLag>,
    broker_nodes: Vec<NodeId>,
}

impl JacquardScenario {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        seed: SimulationSeed,
        deployment_profile: OperatingMode,
        initial_configuration: Observation<Configuration>,
        hosts: Vec<HostSpec>,
        bound_objectives: Vec<BoundObjective>,
        round_limit: u32,
    ) -> Self {
        let objectives = bound_objectives
            .iter()
            .map(|binding| binding.objective.clone())
            .collect();
        Self {
            name: name.into(),
            seed,
            deployment_profile,
            initial_configuration,
            hosts,
            objectives,
            bound_objectives,
            round_limit,
            checkpoint_period_rounds: None,
            topology_lags: Vec::new(),
            broker_nodes: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_checkpoint_interval(mut self, checkpoint_period_rounds: u32) -> Self {
        self.checkpoint_period_rounds = Some(checkpoint_period_rounds);
        self
    }

    #[must_use]
    pub fn hosts(&self) -> &[HostSpec] {
        &self.hosts
    }

    #[must_use]
    pub fn bound_objectives(&self) -> &[BoundObjective] {
        &self.bound_objectives
    }

    #[must_use]
    pub fn round_limit(&self) -> u32 {
        self.round_limit
    }

    #[must_use]
    pub fn checkpoint_interval(&self) -> Option<u32> {
        self.checkpoint_period_rounds
    }

    #[must_use]
    pub fn topology_lags(&self) -> &[HostTopologyLag] {
        &self.topology_lags
    }

    #[must_use]
    pub fn broker_nodes(&self) -> &[NodeId] {
        &self.broker_nodes
    }

    #[must_use]
    pub fn lag_rounds_for(&self, local_node_id: NodeId, round_index: u32) -> u32 {
        self.topology_lags
            .iter()
            .filter(|lag| lag.applies_at(local_node_id, round_index))
            .map(|lag| lag.lag_rounds)
            .max()
            .unwrap_or(0)
    }

    #[must_use]
    pub fn with_initial_configuration(
        mut self,
        initial_configuration: Observation<Configuration>,
    ) -> Self {
        self.initial_configuration = initial_configuration;
        self
    }

    #[must_use]
    pub fn with_round_limit(mut self, round_limit: u32) -> Self {
        self.round_limit = round_limit;
        self
    }

    #[must_use]
    pub fn with_topology_lags(mut self, topology_lags: Vec<HostTopologyLag>) -> Self {
        self.topology_lags = topology_lags;
        self
    }

    #[must_use]
    pub fn with_broker_nodes(mut self, broker_nodes: Vec<NodeId>) -> Self {
        self.broker_nodes = broker_nodes;
        self
    }

    #[must_use]
    pub fn with_seed(mut self, seed: SimulationSeed) -> Self {
        self.seed = seed;
        self
    }

    #[must_use]
    pub fn all_hosts_pathway(&self) -> bool {
        self.hosts
            .iter()
            .all(|host| matches!(host.lane, EngineLane::Pathway))
    }
}

impl RoutingScenario for JacquardScenario {
    fn name(&self) -> &str {
        &self.name
    }

    fn seed(&self) -> SimulationSeed {
        self.seed
    }

    fn deployment_profile(&self) -> &OperatingMode {
        &self.deployment_profile
    }

    fn initial_configuration(&self) -> &Observation<Configuration> {
        &self.initial_configuration
    }

    fn objectives(&self) -> &[RoutingObjective] {
        &self.objectives
    }
}
