use jacquard_core::{
    Configuration, NodeId, Observation, OperatingMode, RoutingObjective, SimulationSeed,
};
use jacquard_traits::RoutingScenario;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineLane {
    Pathway,
    Batman,
    PathwayAndBatman,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostSpec {
    pub local_node_id: NodeId,
    pub lane: EngineLane,
}

impl HostSpec {
    #[must_use]
    pub fn pathway(local_node_id: NodeId) -> Self {
        Self { local_node_id, lane: EngineLane::Pathway }
    }

    #[must_use]
    pub fn batman(local_node_id: NodeId) -> Self {
        Self { local_node_id, lane: EngineLane::Batman }
    }

    #[must_use]
    pub fn pathway_and_batman(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            lane: EngineLane::PathwayAndBatman,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundObjective {
    pub owner_node_id: NodeId,
    pub objective: RoutingObjective,
}

impl BoundObjective {
    #[must_use]
    pub fn new(owner_node_id: NodeId, objective: RoutingObjective) -> Self {
        Self { owner_node_id, objective }
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
    checkpoint_interval: Option<u32>,
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
            checkpoint_interval: None,
        }
    }

    #[must_use]
    pub fn with_checkpoint_interval(mut self, checkpoint_interval: u32) -> Self {
        self.checkpoint_interval = Some(checkpoint_interval);
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
        self.checkpoint_interval
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
