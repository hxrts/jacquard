//! Shared scenario templates for route-visible experiments.

use jacquard_core::{Configuration, Observation, SimulationSeed};

use super::{apply_overrides, BoundObjective, ExperimentParameterSet, HostSpec, JacquardScenario};

pub(super) struct RouteVisibleScenarioTemplate {
    pub scenario_name: String,
    pub seed: SimulationSeed,
    pub operating_mode: jacquard_core::OperatingMode,
    pub topology: Observation<Configuration>,
    pub hosts: Vec<HostSpec>,
    pub objectives: Vec<BoundObjective>,
    pub round_limit: u32,
}

impl RouteVisibleScenarioTemplate {
    #[must_use]
    pub(super) fn into_scenario(self, parameters: &ExperimentParameterSet) -> JacquardScenario {
        apply_overrides(
            &JacquardScenario::new(
                self.scenario_name,
                self.seed,
                self.operating_mode,
                self.topology,
                self.hosts,
                self.objectives,
                self.round_limit,
            ),
            parameters,
        )
    }
}

#[must_use]
pub(super) fn route_visible_template(
    scenario_name: String,
    seed: SimulationSeed,
    operating_mode: jacquard_core::OperatingMode,
    topology: Observation<Configuration>,
    hosts: Vec<HostSpec>,
    objectives: Vec<BoundObjective>,
    round_limit: u32,
) -> RouteVisibleScenarioTemplate {
    RouteVisibleScenarioTemplate {
        scenario_name,
        seed,
        operating_mode,
        topology,
        hosts,
        objectives,
        round_limit,
    }
}
