//! Candidate assembly and plan-token re-derivation.
//!
//! Control flow: path search hands this module a concrete node path. We turn
//! that into mesh segments, classify the route, derive route cost/summary/
//! witness/admission state, and encode the result into a self-contained
//! backend token. The same logic is also used on cache miss so planner state
//! stays memoization-only rather than semantic.

use jacquard_core::{
    SelectedRoutingParameters, BackendRouteId, Configuration, NodeId, Observation,
    RouteError, RouteSelectionError, RouteWitness, RoutingObjective, Tick, TimeWindow,
};
use jacquard_traits::{MeshNeighborhoodEstimateAccess, MeshPeerEstimateAccess};

use super::{
    super::support::{
        decode_backend_token, deterministic_order_key, encode_backend_token,
        encode_path_bytes, node_path_from_plan_token, route_cost_for_segments,
        MeshPlanToken,
    },
    admission::mesh_admission_check,
    MeshEngine, MeshHasherBounds, MeshSelectorBounds,
};
use crate::{
    committee::mesh_admission_assumptions,
    engine::{CachedCandidate, MESH_CANDIDATE_VALIDITY_TICKS},
    MeshRouteSegment,
};

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::super::MeshTopologyBounds,
    Topology::PeerEstimate: MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    pub(super) fn candidate_for_path(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
        node_path: &[NodeId],
        destination_node: &jacquard_core::Node,
    ) -> Option<(BackendRouteId, CachedCandidate)> {
        let (plan, _segments) = self.plan_token_for_path(
            objective,
            profile,
            topology,
            node_path,
            destination_node,
        )?;
        let backend_route_id = encode_backend_token(&plan);
        let cached = self.cached_candidate_from_plan(
            objective, profile, topology, node_path, &plan,
        )?;
        Some((backend_route_id, cached))
    }

    fn plan_token_for_path(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
        node_path: &[NodeId],
        destination_node: &jacquard_core::Node,
    ) -> Option<(MeshPlanToken, Vec<MeshRouteSegment>)> {
        let segments = self.derive_segments(&topology.value, node_path)?;
        let hold_capable = self
            .hold_capable_for_destination(destination_node, topology.observed_at_tick);
        let route_class =
            self.determine_route_class(objective, segments.len(), hold_capable);
        let valid_for = TimeWindow::new(
            topology.observed_at_tick,
            Tick(
                topology
                    .observed_at_tick
                    .0
                    .saturating_add(MESH_CANDIDATE_VALIDITY_TICKS),
            ),
        )
        .expect("mesh candidates always use a positive validity window");
        let committee_status = super::super::support::committee_status(
            self.maybe_select_committee(objective, profile, topology),
        );
        let plan = MeshPlanToken {
            epoch: topology.value.epoch,
            source: self.local_node_id,
            destination: objective.destination.clone(),
            segments: segments.clone(),
            valid_for,
            route_class: route_class.clone(),
            committee_status,
        };
        Some((plan, segments))
    }

    // long-block-exception: linear composer over helper calls.
    fn cached_candidate_from_plan(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
        node_path: &[NodeId],
        plan: &MeshPlanToken,
    ) -> Option<CachedCandidate> {
        let route_id = self
            .route_id_for_backend(&encode_backend_token(plan))
            .ok()?;
        let segments = &plan.segments;
        let route_class = &plan.route_class;
        let connectivity = self.route_connectivity_for_path(
            objective,
            topology,
            node_path,
            route_class,
        );
        let path_metric_score = self.path_metric_score(
            objective,
            topology,
            node_path,
            segments,
            route_class,
        );
        let route_cost =
            route_cost_for_segments(node_path, segments, route_class, &topology.value);
        let summary = self.build_candidate_summary(
            topology,
            connectivity,
            segments,
            plan.valid_for,
        );
        let estimate = self.build_candidate_estimate(
            topology,
            connectivity,
            route_class,
            segments,
        );
        let admission_assumptions =
            mesh_admission_assumptions(profile, &topology.value);
        let admission_check = mesh_admission_check(
            objective,
            profile,
            &summary,
            &route_cost,
            &admission_assumptions,
            &plan.committee_status,
        );
        let witness = RouteWitness {
            objective_protection: objective.target_protection,
            delivered_protection: summary.protection,
            objective_connectivity: objective.target_connectivity,
            delivered_connectivity: summary.connectivity,
            admission_profile: admission_assumptions,
            topology_epoch: topology.value.epoch,
            degradation: estimate.value.degradation,
        };
        let path_bytes = encode_path_bytes(node_path, segments);
        let ordering_key =
            deterministic_order_key(route_id, &self.hashing, &path_bytes);
        Some(CachedCandidate {
            route_id,
            path_metric_score,
            summary,
            estimate,
            admission_check,
            witness,
            ordering_key,
        })
    }

    pub(in crate::engine) fn derive_candidate_from_backend_ref(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
        backend_route_id: &BackendRouteId,
    ) -> Result<CachedCandidate, RouteError> {
        let plan =
            self.validated_plan_for_backend_ref(objective, topology, backend_route_id)?;
        let node_path = node_path_from_plan_token(&plan);
        let candidate = self
            .cached_candidate_from_plan(objective, profile, topology, &node_path, &plan)
            .ok_or(RouteSelectionError::NoCandidate)?;
        Ok(candidate)
    }

    fn validated_plan_for_backend_ref(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        backend_route_id: &BackendRouteId,
    ) -> Result<MeshPlanToken, RouteError> {
        let plan = decode_backend_token(backend_route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        self.ensure_plan_matches_objective(objective, topology, &plan)?;
        self.ensure_plan_matches_current_topology(objective, topology, &plan)?;
        if encode_backend_token(&plan) != *backend_route_id {
            return Err(RouteSelectionError::NoCandidate.into());
        }
        Ok(plan)
    }

    fn ensure_plan_matches_objective(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        plan: &MeshPlanToken,
    ) -> Result<(), RouteError> {
        if plan.source != self.local_node_id
            || plan.destination != objective.destination
        {
            return Err(RouteSelectionError::NoCandidate.into());
        }
        if plan.epoch != topology.value.epoch
            || !plan.valid_for.contains(topology.observed_at_tick)
        {
            return Err(RouteSelectionError::NoCandidate.into());
        }
        Ok(())
    }

    fn ensure_plan_matches_current_topology(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        plan: &MeshPlanToken,
    ) -> Result<(), RouteError> {
        let node_path = node_path_from_plan_token(plan);
        let destination_node_id =
            *node_path.last().ok_or(RouteSelectionError::NoCandidate)?;
        let destination_node = topology
            .value
            .nodes
            .get(&destination_node_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        let hold_capable = self
            .hold_capable_for_destination(destination_node, topology.observed_at_tick);
        let route_class =
            self.determine_route_class(objective, plan.segments.len(), hold_capable);
        if route_class != plan.route_class {
            return Err(RouteSelectionError::NoCandidate.into());
        }
        let derived_segments = self
            .derive_segments(&topology.value, &node_path)
            .ok_or(RouteSelectionError::NoCandidate)?;
        if derived_segments != plan.segments {
            return Err(RouteSelectionError::NoCandidate.into());
        }
        Ok(())
    }
}
