use jacquard_core::{Configuration, Observation, Tick};
use jacquard_mem_link_profile::SharedInMemoryNetwork;
use jacquard_reference_client::{
    BridgeQueueConfig, ClientBuilder, FieldBootstrapSummary as ClientFieldBootstrapSummary,
};

use crate::scenario::{EngineLane, HostOverrides, HostSpec, JacquardScenario};

const SIMULATOR_BRIDGE_QUEUE_CONFIG: BridgeQueueConfig = BridgeQueueConfig::new(320, 320);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HostBuildPlan {
    local_node_id: jacquard_core::NodeId,
    lane: EngineLane,
    overrides: HostBuildOverrides,
    queue_config: BridgeQueueConfig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HostBuildOverrides {
    routing_profile: Option<jacquard_core::SelectedRoutingParameters>,
    policy_inputs: Option<jacquard_core::RoutingPolicyInputs>,
    batman_bellman_decay_window: Option<jacquard_batman_bellman::DecayWindow>,
    batman_classic_decay_window: Option<jacquard_batman_classic::DecayWindow>,
    babel_decay_window: Option<jacquard_babel::DecayWindow>,
    olsrv2_decay_window: Option<jacquard_olsrv2::DecayWindow>,
    pathway_search_config: Option<jacquard_pathway::PathwaySearchConfig>,
    field_search_config: Option<jacquard_field::FieldSearchConfig>,
    scatter_config: Option<jacquard_scatter::ScatterEngineConfig>,
    field_bootstrap_summaries: Vec<ClientFieldBootstrapSummary>,
}

impl From<&HostOverrides> for HostBuildOverrides {
    fn from(overrides: &HostOverrides) -> Self {
        Self {
            routing_profile: overrides.routing_profile.clone(),
            policy_inputs: overrides.policy_inputs.clone(),
            batman_bellman_decay_window: overrides.batman_bellman_decay_window,
            batman_classic_decay_window: overrides.batman_classic_decay_window,
            babel_decay_window: overrides.babel_decay_window,
            olsrv2_decay_window: overrides.olsrv2_decay_window,
            pathway_search_config: overrides.pathway_search_config.clone(),
            field_search_config: overrides.field_search_config.clone(),
            scatter_config: overrides.scatter_config,
            field_bootstrap_summaries: overrides
                .field_bootstrap_summaries
                .iter()
                .map(|bootstrap| ClientFieldBootstrapSummary {
                    destination: bootstrap.destination.clone(),
                    from_neighbor: bootstrap.from_neighbor,
                    forward_observation: bootstrap.forward_observation,
                    reverse_feedback: bootstrap.reverse_feedback,
                })
                .collect(),
        }
    }
}

impl From<&HostSpec> for HostBuildPlan {
    fn from(host: &HostSpec) -> Self {
        Self {
            local_node_id: host.local_node_id,
            lane: host.lane.clone(),
            overrides: HostBuildOverrides::from(&host.overrides),
            queue_config: SIMULATOR_BRIDGE_QUEUE_CONFIG,
        }
    }
}

impl HostBuildPlan {
    #[must_use]
    pub(super) fn local_node_id(&self) -> jacquard_core::NodeId {
        self.local_node_id
    }

    #[must_use]
    pub(super) fn into_builder(
        self,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        observed_at_tick: Tick,
    ) -> ClientBuilder {
        let mut builder = base_builder(
            &self.lane,
            self.local_node_id,
            topology,
            network,
            observed_at_tick,
        )
        .with_queue_config(self.queue_config);
        if let Some(routing_profile) = self.overrides.routing_profile {
            builder = builder.with_profile(routing_profile);
        }
        if let Some(policy_inputs) = self.overrides.policy_inputs {
            builder = builder.with_policy_inputs(policy_inputs);
        }
        if let Some(decay_window) = self.overrides.batman_bellman_decay_window {
            builder = builder.with_batman_bellman_decay_window(decay_window);
        }
        if let Some(decay_window) = self.overrides.batman_classic_decay_window {
            builder = builder.with_batman_classic_decay_window(decay_window);
        }
        if let Some(decay_window) = self.overrides.babel_decay_window {
            builder = builder.with_babel_decay_window(decay_window);
        }
        if let Some(decay_window) = self.overrides.olsrv2_decay_window {
            builder = builder.with_olsrv2_decay_window(decay_window);
        }
        if let Some(search_config) = self.overrides.pathway_search_config {
            builder = builder.with_pathway_search_config(search_config);
        }
        if let Some(search_config) = self.overrides.field_search_config {
            builder = builder.with_field_search_config(search_config);
        }
        if let Some(scatter_config) = self.overrides.scatter_config {
            builder = builder.with_scatter_config(scatter_config);
        }
        for bootstrap in self.overrides.field_bootstrap_summaries {
            builder = builder.with_field_bootstrap_summary(bootstrap);
        }
        builder
    }
}

#[must_use]
pub(super) fn host_build_plans(scenario: &JacquardScenario) -> Vec<HostBuildPlan> {
    scenario.hosts().iter().map(HostBuildPlan::from).collect()
}

// long-block-exception: the lane-to-builder mapping is intentionally direct so
// engine wiring stays explicit at the host construction boundary.
fn base_builder(
    lane: &EngineLane,
    local_node_id: jacquard_core::NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    observed_at_tick: Tick,
) -> ClientBuilder {
    match lane {
        EngineLane::Pathway => {
            ClientBuilder::pathway(local_node_id, topology, network, observed_at_tick)
        }
        EngineLane::Field => {
            ClientBuilder::field(local_node_id, topology, network, observed_at_tick)
        }
        EngineLane::Scatter => {
            ClientBuilder::scatter(local_node_id, topology, network, observed_at_tick)
        }
        EngineLane::PathwayAndBatmanBellman => ClientBuilder::pathway_and_batman_bellman(
            local_node_id,
            topology,
            network,
            observed_at_tick,
        ),
        EngineLane::PathwayAndField => {
            ClientBuilder::pathway_and_field(local_node_id, topology, network, observed_at_tick)
        }
        EngineLane::FieldAndBatmanBellman => ClientBuilder::field_and_batman_bellman(
            local_node_id,
            topology,
            network,
            observed_at_tick,
        ),
        EngineLane::AllEngines => {
            ClientBuilder::all_engines(local_node_id, topology, network, observed_at_tick)
        }
        EngineLane::BatmanBellman => {
            ClientBuilder::batman_bellman(local_node_id, topology, network, observed_at_tick)
        }
        EngineLane::BatmanClassic => {
            ClientBuilder::batman_classic(local_node_id, topology, network, observed_at_tick)
        }
        EngineLane::Babel => {
            ClientBuilder::babel(local_node_id, topology, network, observed_at_tick)
        }
        EngineLane::OlsrV2 => {
            ClientBuilder::olsrv2(local_node_id, topology, network, observed_at_tick)
        }
        EngineLane::PathwayAndBabel => {
            ClientBuilder::pathway_and_babel(local_node_id, topology, network, observed_at_tick)
        }
        EngineLane::PathwayAndOlsrV2 => {
            ClientBuilder::pathway_and_olsrv2(local_node_id, topology, network, observed_at_tick)
        }
        EngineLane::BabelAndBatmanBellman => ClientBuilder::babel_and_batman_bellman(
            local_node_id,
            topology,
            network,
            observed_at_tick,
        ),
        EngineLane::OlsrV2AndBatmanBellman => ClientBuilder::olsrv2_and_batman_bellman(
            local_node_id,
            topology,
            network,
            observed_at_tick,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::host_build_plans;
    use crate::presets;

    #[test]
    fn host_build_plans_preserve_scenario_host_order() {
        let (scenario, _) = presets::pathway_line();
        let plan_node_ids = host_build_plans(&scenario)
            .into_iter()
            .map(|plan| plan.local_node_id())
            .collect::<Vec<_>>();
        let scenario_node_ids = scenario
            .hosts()
            .iter()
            .map(|host| host.local_node_id)
            .collect::<Vec<_>>();
        assert_eq!(plan_node_ids, scenario_node_ids);
    }
}
