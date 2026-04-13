use jacquard_batman::DecayWindow;
use jacquard_core::{
    Configuration, NodeId, Observation, OperatingMode, RoutingObjective, RoutingPolicyInputs,
    SelectedRoutingParameters, SimulationSeed,
};
use jacquard_field::FieldSearchConfig;
use jacquard_pathway::PathwaySearchConfig;
use jacquard_traits::RoutingScenario;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineLane {
    Pathway,
    Batman,
    Field,
    PathwayAndBatman,
    PathwayAndField,
    FieldAndBatman,
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
    pub batman_decay_window: Option<DecayWindow>,
    pub pathway_search_config: Option<PathwaySearchConfig>,
    pub field_search_config: Option<FieldSearchConfig>,
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
    pub fn batman(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::Batman,
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
    pub fn pathway_and_batman(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::PathwayAndBatman,
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
    pub fn field_and_batman(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::FieldAndBatman,
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
    pub fn with_batman_decay_window(mut self, batman_decay_window: DecayWindow) -> Self {
        self.overrides.batman_decay_window = Some(batman_decay_window);
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
