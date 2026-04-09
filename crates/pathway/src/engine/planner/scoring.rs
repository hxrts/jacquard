//! Pathway-private scoring used after raw path search.
//!
//! Control flow: weighted path search yields feasible node paths first. This
//! module then computes the ranking signals layered on top of that search:
//! per-hop penalties, topology-model preference, deterministic tie-break
//! inputs, letting the planner sort candidates without changing their shared
//! shape. Key methods on `PathwayEngine`: `candidate_preference_score`
//! combines first-hop peer estimates and neighborhood estimates into an
//! overall preference adjustment used as a secondary sort key after raw
//! path-metric score; `edge_metric_score` computes the weighted per-edge
//! cost from delivery, symmetry, and loss penalties minus peer and
//! neighborhood bonuses; `path_metric_score` aggregates edge scores across
//! all segments with protocol-diversity bonuses and deferred-delivery
//! discounts. All scoring is deterministic and uses no floating-point types.

use jacquard_core::{Configuration, NodeId, Observation, RoutingObjective};

use super::{
    super::support::link_quality_penalties, PathwayEngine, PATH_METRIC_BASE_HOP_COST,
    PATH_METRIC_DELIVERY_PENALTY_WEIGHT, PATH_METRIC_LOSS_PENALTY_WEIGHT,
    PATH_METRIC_SYMMETRY_PENALTY_WEIGHT,
};
use crate::{
    topology::{
        estimate_hop_link, optional_health_score_value,
        service_requirements_for_objective,
        service_surface_health_score_for_requirements,
    },
    PathwayNeighborhoodEstimateAccess, PathwayPeerEstimateAccess, PathwayRouteClass,
    PATHWAY_ENGINE_ID,
};

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    PathwayEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::super::PathwayTopologyBounds,
    Topology::PeerEstimate: PathwayPeerEstimateAccess,
    Topology::NeighborhoodEstimate: PathwayNeighborhoodEstimateAccess,
{
    pub(super) fn candidate_preference_score(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        node_path: &[NodeId],
        route_class: &PathwayRouteClass,
    ) -> u32 {
        let peer_score = self.first_hop_preference_score(
            objective,
            topology,
            node_path,
            route_class,
        );
        let (bonus, penalty) = self.neighborhood_preference_adjustments(topology);
        peer_score.saturating_add(bonus).saturating_sub(penalty)
    }

    fn first_hop_preference_score(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        node_path: &[NodeId],
        route_class: &PathwayRouteClass,
    ) -> u32 {
        let requirements = service_requirements_for_objective(
            objective,
            matches!(route_class, PathwayRouteClass::DeferredDelivery),
        );
        let first_hop = node_path.get(1).copied();
        first_hop
            .and_then(|peer_node_id| {
                let node = topology.value.nodes.get(&peer_node_id)?;
                let estimate = self.topology_model.peer_estimate(
                    &self.local_node_id,
                    &peer_node_id,
                    topology.observed_at_tick,
                    &topology.value,
                )?;
                Some(
                    optional_health_score_value(estimate.relay_value_score())
                        .saturating_add(optional_health_score_value(
                            estimate.retention_value_score(),
                        ))
                        .saturating_add(optional_health_score_value(
                            estimate.stability_score(),
                        ))
                        .saturating_add(service_surface_health_score_for_requirements(
                            &node.profile.services,
                            &PATHWAY_ENGINE_ID,
                            topology.observed_at_tick,
                            requirements,
                        )),
                )
            })
            .unwrap_or(0)
    }

    fn neighborhood_preference_adjustments(
        &self,
        topology: &Observation<Configuration>,
    ) -> (u32, u32) {
        let neighborhood = self.topology_model.neighborhood_estimate(
            &self.local_node_id,
            topology.observed_at_tick,
            &topology.value,
        );
        let bonus = neighborhood
            .as_ref()
            .map(|estimate| {
                optional_health_score_value(estimate.density_score()).saturating_add(
                    optional_health_score_value(estimate.service_stability_score()),
                )
            })
            .unwrap_or(0);
        let penalty = neighborhood
            .as_ref()
            .map(|estimate| {
                optional_health_score_value(estimate.repair_pressure_score())
                    .saturating_add(optional_health_score_value(
                        estimate.partition_risk_score(),
                    ))
            })
            .unwrap_or(0);
        (bonus, penalty)
    }

    pub(super) fn edge_metric_score(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        from_node_id: &NodeId,
        to_node_id: &NodeId,
    ) -> Option<u32> {
        let configuration = &topology.value;
        let (_, link_state) =
            estimate_hop_link(from_node_id, to_node_id, configuration)?;
        let penalties = link_quality_penalties(&link_state);
        let (delivery_penalty, symmetry_penalty, loss_penalty) =
            (penalties.delivery, penalties.symmetry, penalties.loss);
        let peer_bonus = self.peer_bonus_for_edge(
            objective,
            topology,
            from_node_id,
            to_node_id,
            configuration,
        );
        let (neighborhood_bonus, neighborhood_penalty) =
            self.neighborhood_adjustments_for_edge(topology, to_node_id, configuration);

        Some(
            PATH_METRIC_BASE_HOP_COST
                .saturating_add(
                    delivery_penalty
                        .saturating_mul(PATH_METRIC_DELIVERY_PENALTY_WEIGHT),
                )
                .saturating_add(
                    loss_penalty.saturating_mul(PATH_METRIC_LOSS_PENALTY_WEIGHT),
                )
                .saturating_add(
                    symmetry_penalty
                        .saturating_mul(PATH_METRIC_SYMMETRY_PENALTY_WEIGHT),
                )
                .saturating_add(neighborhood_penalty)
                .saturating_sub(peer_bonus.saturating_add(neighborhood_bonus)),
        )
    }

    fn peer_bonus_for_edge(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        from_node_id: &NodeId,
        to_node_id: &NodeId,
        configuration: &Configuration,
    ) -> u32 {
        self.topology_model
            .peer_estimate(
                from_node_id,
                to_node_id,
                topology.observed_at_tick,
                configuration,
            )
            .map(|estimate| {
                // Weights are intentionally asymmetric: service surface
                // (÷2) is the primary edge bonus; relay/stability (÷4)
                // contribute equally; retention (÷8) is a secondary signal
                // for deferred-delivery routes only.
                optional_health_score_value(estimate.relay_value_score()) / 4
                    + optional_health_score_value(estimate.retention_value_score()) / 8
                    + optional_health_score_value(estimate.stability_score()) / 4
                    + configuration
                        .nodes
                        .get(to_node_id)
                        .map(|node| {
                            service_surface_health_score_for_requirements(
                                &node.profile.services,
                                &PATHWAY_ENGINE_ID,
                                topology.observed_at_tick,
                                service_requirements_for_objective(objective, false),
                            ) / 2
                        })
                        .unwrap_or_else(|| {
                            optional_health_score_value(estimate.service_score()) / 2
                        })
            })
            .unwrap_or(0)
    }

    fn neighborhood_adjustments_for_edge(
        &self,
        topology: &Observation<Configuration>,
        to_node_id: &NodeId,
        configuration: &Configuration,
    ) -> (u32, u32) {
        let neighborhood = self.topology_model.neighborhood_estimate(
            to_node_id,
            topology.observed_at_tick,
            configuration,
        );
        let penalty = neighborhood
            .as_ref()
            .map(|estimate| {
                optional_health_score_value(estimate.repair_pressure_score()) / 2
                    + optional_health_score_value(estimate.partition_risk_score()) / 2
            })
            .unwrap_or(0);
        let bonus = neighborhood
            .as_ref()
            .map(|estimate| {
                optional_health_score_value(estimate.density_score()) / 4
                    + optional_health_score_value(estimate.service_stability_score())
                        / 2
            })
            .unwrap_or(0);
        (bonus, penalty)
    }
}
