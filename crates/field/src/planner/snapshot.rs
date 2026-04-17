use std::collections::BTreeMap;

use jacquard_core::{
    Belief, Configuration, DestinationId, NodeId, Observation, RouteDegradation, RouteEpoch,
    RouteSummary,
};

use crate::{
    planner::{
        admission::{
            bootstrap_class_for_state_with_config, continuity_band_for_state_with_config,
            delivered_connectivity, delivered_protection, evidence_class_from_state,
            uncertainty_class_for,
        },
        publication::publication_confidence_for,
    },
    policy::FieldPolicy,
    route::FieldWitnessDetail,
    search::FieldSearchConfig,
    state::{
        ControlState, DestinationFieldState, DestinationKey, MeanFieldState, OperatingRegime,
        RoutingPosture,
    },
    summary::{derive_degradation_class, FieldSummary, SummaryDestinationKey},
    FieldEngine, FIELD_ENGINE_ID,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldPlannerSnapshot {
    pub(crate) local_node_id: NodeId,
    pub(crate) destinations: BTreeMap<DestinationKey, DestinationFieldState>,
    pub(crate) mean_field: MeanFieldState,
    pub(crate) controller: ControlState,
    pub(crate) regime: OperatingRegime,
    pub(crate) posture: RoutingPosture,
    pub(crate) search_config: FieldSearchConfig,
    pub(crate) effective_search_config: FieldSearchConfig,
    pub(crate) policy: FieldPolicy,
}

impl FieldPlannerSnapshot {
    pub(crate) fn witness_detail_from_state(
        &self,
        destination_state: &DestinationFieldState,
    ) -> FieldWitnessDetail {
        FieldWitnessDetail {
            evidence_class: evidence_class_from_state(destination_state),
            uncertainty_class: uncertainty_class_for(
                destination_state.posterior.usability_entropy.value(),
            ),
            bootstrap_class: bootstrap_class_for_state_with_config(
                destination_state,
                &self.search_config,
            ),
            continuity_band: continuity_band_for_state_with_config(
                destination_state,
                &self.search_config,
            ),
            corridor_support: destination_state.corridor_belief.delivery_support,
            retention_support: destination_state.corridor_belief.retention_affinity,
            usability_entropy: destination_state.posterior.usability_entropy,
            top_corridor_mass: destination_state.posterior.top_corridor_mass,
            frontier_width: u8::try_from(destination_state.frontier.len()).unwrap_or(u8::MAX),
            regime: self.regime,
            posture: self.posture,
            degradation: self.route_degradation_for(destination_state, RouteEpoch(0)),
        }
    }

    pub(crate) fn route_summary_for(
        &self,
        destination_state: &DestinationFieldState,
        summary_neighbor: NodeId,
        topology: &Observation<Configuration>,
    ) -> RouteSummary {
        let hop_midpoint = destination_state
            .corridor_belief
            .expected_hop_band
            .min_hops
            .saturating_add(
                destination_state
                    .corridor_belief
                    .expected_hop_band
                    .max_hops
                    .saturating_sub(destination_state.corridor_belief.expected_hop_band.min_hops)
                    / 2,
            );
        let protocol_mix = topology
            .value
            .links
            .get(&(self.local_node_id, summary_neighbor))
            .map(|link| vec![link.endpoint.transport_kind.clone()])
            .unwrap_or_default();
        RouteSummary {
            engine: FIELD_ENGINE_ID,
            protection: delivered_protection(destination_state, &self.search_config),
            connectivity: delivered_connectivity(
                self.posture,
                destination_state,
                &self.search_config,
            ),
            protocol_mix,
            hop_count_hint: Belief::estimated(
                hop_midpoint,
                jacquard_core::RatioPermille(publication_confidence_for(
                    destination_state,
                    &self.search_config,
                )),
                topology.observed_at_tick,
            ),
            valid_for: destination_state.corridor_belief.validity_window,
        }
    }

    pub(crate) fn route_degradation_for(
        &self,
        destination_state: &DestinationFieldState,
        topology_epoch: RouteEpoch,
    ) -> RouteDegradation {
        let summary = FieldSummary {
            destination: SummaryDestinationKey::from(&DestinationId::from(
                &destination_state.destination,
            )),
            topology_epoch,
            freshness_tick: destination_state
                .corridor_belief
                .validity_window
                .start_tick(),
            hop_band: destination_state.corridor_belief.expected_hop_band,
            delivery_support: destination_state.corridor_belief.delivery_support,
            congestion_penalty: destination_state.corridor_belief.congestion_penalty,
            retention_support: destination_state.corridor_belief.retention_affinity,
            uncertainty_penalty: destination_state.posterior.usability_entropy,
            evidence_class: evidence_class_from_state(destination_state),
            uncertainty_class: uncertainty_class_for(
                destination_state.posterior.usability_entropy.value(),
            ),
        };
        derive_degradation_class(&summary, self.regime, &self.controller)
    }

    pub(crate) fn destination_supports_objective(
        &self,
        topology: &Observation<Configuration>,
        objective: &jacquard_core::RoutingObjective,
    ) -> bool {
        match objective.destination {
            DestinationId::Node(destination) => topology
                .value
                .nodes
                .get(&destination)
                .map(|node| {
                    node.profile.services.iter().any(|service| {
                        service.service_kind == objective.service_kind
                            && service.routing_engines.contains(&FIELD_ENGINE_ID)
                    })
                })
                .unwrap_or(false),
            DestinationId::Gateway(_) | DestinationId::Service(_) => self
                .destinations
                .contains_key(&DestinationKey::from(&objective.destination)),
        }
    }
}

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    pub(crate) fn planner_snapshot(&self) -> FieldPlannerSnapshot {
        FieldPlannerSnapshot {
            local_node_id: self.local_node_id,
            destinations: self.state.destinations.clone(),
            mean_field: self.state.mean_field.clone(),
            controller: self.state.controller.clone(),
            regime: self.state.regime.current,
            posture: self.state.posture.current,
            search_config: self.search_config.clone(),
            effective_search_config: self.effective_search_config(),
            policy: *self.policy(),
        }
    }
}
