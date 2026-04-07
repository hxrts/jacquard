//! Shared-candidate publication and planner cache population.
//!
//! Control flow: once a candidate has been fully derived, this module builds
//! the shared `RouteSummary`/`RouteEstimate`, applies final ordering and
//! truncation, stores the memoized candidate entry, and publishes the shared
//! `RouteCandidate` values the router sees.

use std::cmp::Reverse;

use jacquard_core::{
    AdaptiveRoutingProfile, Belief, Configuration, Estimate, Observation,
    RouteCandidate, RouteConnectivityProfile, RouteEstimate, RouteSummary,
    RoutingObjective, TimeWindow,
};

use super::{
    super::support::{
        confidence_for_segments, decode_backend_token, degradation_for_candidate,
        node_path_from_plan_token, unique_protocol_mix,
    },
    MeshEngine,
};
use crate::{
    engine::{CachedCandidate, MeshSelectorBounds, MESH_CANDIDATE_COUNT_MAX},
    MeshRouteClass, MeshRouteSegment, MESH_ENGINE_ID,
};

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
{
    pub(super) fn build_candidate_summary(
        &self,
        topology: &Observation<Configuration>,
        connectivity: RouteConnectivityProfile,
        segments: &[MeshRouteSegment],
        valid_for: TimeWindow,
    ) -> RouteSummary {
        RouteSummary {
            engine: MESH_ENGINE_ID,
            protection: jacquard_core::RouteProtectionClass::LinkProtected,
            connectivity,
            protocol_mix: unique_protocol_mix(segments),
            hop_count_hint: Belief::Estimated(Estimate {
                value: u8::try_from(segments.len())
                    .expect("segment count is bounded by ROUTE_HOP_COUNT_MAX"),
                confidence_permille: jacquard_core::RatioPermille(1000),
                updated_at_tick: topology.observed_at_tick,
            }),
            valid_for,
        }
    }

    pub(super) fn build_candidate_estimate(
        &self,
        topology: &Observation<Configuration>,
        connectivity: RouteConnectivityProfile,
        route_class: &MeshRouteClass,
        segments: &[MeshRouteSegment],
    ) -> Estimate<RouteEstimate> {
        let configuration = &topology.value;
        Estimate {
            value: RouteEstimate {
                estimated_protection:
                    jacquard_core::RouteProtectionClass::LinkProtected,
                estimated_connectivity: connectivity,
                topology_epoch: configuration.epoch,
                degradation: degradation_for_candidate(configuration, route_class),
            },
            confidence_permille: confidence_for_segments(segments, configuration),
            updated_at_tick: topology.observed_at_tick,
        }
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Selector: MeshSelectorBounds,
{
    pub(super) fn maybe_select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Result<Option<jacquard_core::CommitteeSelection>, jacquard_core::RouteError>
    {
        self.selector.select_committee(objective, profile, topology)
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::super::MeshTopologyBounds,
    Topology::PeerEstimate: jacquard_traits::MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: jacquard_traits::MeshNeighborhoodEstimateAccess,
    Hasher: super::MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    pub(super) fn collect_candidates(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Vec<(jacquard_core::BackendRouteId, CachedCandidate)> {
        let configuration = &topology.value;
        self.weighted_paths(objective, topology)
            .into_iter()
            .filter(|(_, node_path)| {
                node_path.last().copied() != Some(self.local_node_id)
            })
            .filter_map(|(_, node_path)| {
                let destination_node_id = *node_path.last()?;
                let destination_node = configuration.nodes.get(&destination_node_id)?;
                if !super::objective_matches_node(
                    &destination_node_id,
                    destination_node,
                    objective,
                    &MESH_ENGINE_ID,
                    topology.observed_at_tick,
                ) {
                    return None;
                }
                self.candidate_for_path(
                    objective,
                    profile,
                    topology,
                    &node_path,
                    destination_node,
                )
            })
            .collect()
    }

    pub(super) fn sort_candidates(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        cached: &mut Vec<(jacquard_core::BackendRouteId, CachedCandidate)>,
    ) {
        cached.sort_by_key(|(backend_route_id, candidate)| {
            let preference = decode_backend_token(backend_route_id)
                .map(|plan| {
                    let node_path = node_path_from_plan_token(&plan);
                    self.candidate_preference_score(
                        objective,
                        topology,
                        &node_path,
                        &plan.route_class,
                    )
                })
                .unwrap_or(0);
            // Sort: lower path_metric_score first, then higher preference
            // (Reverse), then stable_key + tie_break for a total
            // deterministic order when cost and preference are equal.
            (
                candidate.path_metric_score,
                Reverse(preference),
                candidate.ordering_key.stable_key,
                candidate.ordering_key.tie_break,
            )
        });
        cached.truncate(MESH_CANDIDATE_COUNT_MAX);
    }

    pub(super) fn cache_and_publish_candidates(
        &self,
        cached: Vec<(jacquard_core::BackendRouteId, CachedCandidate)>,
    ) -> Vec<RouteCandidate> {
        let mut cache = self.candidate_cache.borrow_mut();
        // Clear the full cache before inserting new candidates. Stale
        // entries from prior planning cycles must not survive across calls.
        cache.clear();

        cached
            .into_iter()
            .map(|(backend_route_id, candidate)| {
                cache.insert(backend_route_id.clone(), candidate.clone());
                RouteCandidate {
                    summary: candidate.summary,
                    estimate: candidate.estimate,
                    backend_ref: jacquard_core::BackendRouteRef {
                        engine: MESH_ENGINE_ID,
                        backend_route_id,
                    },
                }
            })
            .collect()
    }
}
