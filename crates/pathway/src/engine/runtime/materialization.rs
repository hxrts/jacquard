//! Materialization validation and active-route installation.
//!
//! Control flow: runtime entry points first decode and validate the backend
//! plan token against the explicit admission and latest topology. Once that
//! passes, this module assembles the materialization proof, shared
//! installation object, and mesh-private active-route state that gets
//! checkpointed and inserted into the live route table.

use jacquard_core::{
    Configuration, Fact, FactBasis, Observation, RouteError, RouteInstallation,
    RouteLifecycleEvent, RouteMaterializationInput, RouteMaterializationProof,
    RouteProgressContract, RouteProgressState, RouteRuntimeError,
};

use super::{
    super::{
        support::{
            decode_backend_token, deterministic_order_key, encode_path_bytes,
            limit_u32, node_path_from_plan_token,
        },
        types::PathwayCommitteeStatus,
        ActivePathwayRoute, PATHWAY_ACTIVE_ROUTE_COUNT_MAX,
    },
    PathwayEffectsBounds, PathwayEngine, PathwayHasherBounds, PathwaySelectorBounds,
    TransportEffectsBounds,
};
use crate::{PathwayNeighborhoodEstimateAccess, PathwayPeerEstimateAccess};

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    PathwayEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::super::PathwayTopologyBounds,
    Topology::PeerEstimate: PathwayPeerEstimateAccess,
    Topology::NeighborhoodEstimate: PathwayNeighborhoodEstimateAccess,
    Transport: TransportEffectsBounds,
    Retention: super::super::PathwayRetentionBounds,
    Effects: PathwayEffectsBounds,
    Hasher: PathwayHasherBounds,
    Selector: PathwaySelectorBounds,
{
    pub(super) fn materialization_proof_for(
        &self,
        input: &RouteMaterializationInput,
        now: jacquard_core::Tick,
    ) -> RouteMaterializationProof {
        RouteMaterializationProof {
            stamp: jacquard_core::RouteIdentityStamp {
                route_id: *input.handle.route_id(),
                topology_epoch: input.handle.topology_epoch(),
                materialized_at_tick: now,
                publication_id: *input.handle.publication_id(),
            },
            witness: Fact {
                value: input.admission.witness.clone(),
                basis: FactBasis::Admitted,
                established_at_tick: now,
            },
        }
    }

    pub(super) fn installation_for(
        &self,
        input: &RouteMaterializationInput,
        now: jacquard_core::Tick,
        proof: RouteMaterializationProof,
    ) -> RouteInstallation {
        RouteInstallation {
            materialization_proof: proof,
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: self.current_route_health(None, now),
            progress: RouteProgressContract {
                productive_step_count_max: input
                    .admission
                    .admission_check
                    .productive_step_bound,
                total_step_count_max: input.admission.admission_check.total_step_bound,
                last_progress_at_tick: now,
                state: RouteProgressState::Satisfied,
            },
        }
    }

    pub(super) fn active_route_for_materialization(
        &self,
        input: &RouteMaterializationInput,
        path: super::super::PathwayPath,
        committee: Option<jacquard_core::CommitteeSelection>,
        ordering_key: jacquard_core::DeterministicOrderKey<jacquard_core::RouteId>,
    ) -> ActivePathwayRoute {
        ActivePathwayRoute {
            current_epoch: path.epoch,
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            path,
            committee,
            route_cost: input.admission.admission_check.route_cost.clone(),
            ordering_key,
            forwarding: super::super::PathwayForwardingState {
                current_owner_node_id: input.lease.owner_node_id,
                next_hop_index: 0,
                in_flight_frames: 0,
                last_ack_at_tick: None,
            },
            repair: super::super::PathwayRepairState {
                steps_remaining: limit_u32(
                    input.admission.admission_check.productive_step_bound,
                ),
                last_repaired_at_tick: None,
            },
            handoff: super::super::PathwayHandoffState::default(),
            anti_entropy: super::super::PathwayRouteAntiEntropyState::default(),
        }
    }

    pub(super) fn materialization_plan(
        &self,
        input: &RouteMaterializationInput,
    ) -> Result<
        (
            super::super::PathwayPath,
            Option<jacquard_core::CommitteeSelection>,
            jacquard_core::DeterministicOrderKey<jacquard_core::RouteId>,
        ),
        RouteError,
    > {
        if input.admission.backend_ref.engine != super::super::PATHWAY_ENGINE_ID {
            return Err(RouteRuntimeError::Invalidated.into());
        }

        let plan = decode_backend_token(&input.admission.backend_ref.backend_route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        if plan.source != self.local_node_id
            || plan.destination != input.admission.objective.destination
        {
            return Err(RouteRuntimeError::Invalidated.into());
        }

        let derived_route_id =
            self.route_id_for_backend(&input.admission.backend_ref.backend_route_id)?;
        // Validate the plan token is consistent with the canonical handle
        // stamp chosen by the router. Admission evidence no longer restates
        // canonical identity.
        if derived_route_id != *input.handle.route_id() {
            return Err(RouteRuntimeError::Invalidated.into());
        }

        let node_path = node_path_from_plan_token(&plan);
        let path_bytes = encode_path_bytes(&node_path, &plan.segments);
        let ordering_key =
            deterministic_order_key(derived_route_id, &self.hashing, &path_bytes);
        // Use the router-assigned canonical route_id for the path record.
        // The canonical identity lives in the handle stamp; derived_route_id
        // is the engine-internal content-addressed plan identifier.
        let path = super::super::PathwayPath {
            route_id: *input.handle.route_id(),
            epoch: plan.epoch,
            source: plan.source,
            destination: plan.destination,
            segments: plan.segments,
            valid_for: plan.valid_for,
            route_class: plan.route_class,
        };
        let committee = match plan.committee_status {
            | PathwayCommitteeStatus::Selected(selection) => Some(selection),
            | PathwayCommitteeStatus::NotApplicable => None,
            // SelectorFailed in the plan token means admission should have
            // rejected this candidate. Reaching materialization here is an
            // upstream invariant violation; fail closed.
            | PathwayCommitteeStatus::SelectorFailed => {
                return Err(RouteRuntimeError::Invalidated.into());
            },
        };
        Ok((path, committee, ordering_key))
    }

    pub(super) fn validated_materialization_candidate(
        &self,
        input: &RouteMaterializationInput,
        topology: &Observation<Configuration>,
        now: jacquard_core::Tick,
    ) -> Result<(), RouteError> {
        let plan = decode_backend_token(&input.admission.backend_ref.backend_route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        let claimed_epoch = input.handle.topology_epoch();
        if plan.epoch != claimed_epoch
            || input.admission.witness.topology_epoch != claimed_epoch
            || topology.value.epoch != claimed_epoch
        {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        if !plan.valid_for.contains(now) {
            return Err(RouteRuntimeError::Invalidated.into());
        }

        let derived = self
            .derive_candidate_from_backend_ref(
                &input.admission.objective,
                &input.admission.profile,
                topology,
                &input.admission.backend_ref.backend_route_id,
            )
            .map_err(|_| RouteRuntimeError::Invalidated)?;
        if derived.route_id != *input.handle.route_id()
            || derived.summary != input.admission.summary
            || derived.witness != input.admission.witness
            || derived.admission_check != input.admission.admission_check
        {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        Ok(())
    }

    pub(super) fn materialize_route_inner(
        &mut self,
        input: &RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        let route_id = *input.handle.route_id();
        // Replacements re-use an existing route slot so the budget cap is
        // skipped — an already-active route does not consume an extra slot
        // when re-materialized.
        let previous_active_route = self.active_routes.get(&route_id).cloned();
        let is_replacement = previous_active_route.is_some();
        if !is_replacement && self.active_routes.len() >= PATHWAY_ACTIVE_ROUTE_COUNT_MAX
        {
            return Err(RouteError::Policy(
                jacquard_core::RoutePolicyError::BudgetExceeded,
            ));
        }
        let now = self.effects.now_tick();
        let latest_topology = self
            .latest_topology
            .as_ref()
            .ok_or(RouteError::Runtime(RouteRuntimeError::Invalidated))?;
        input.lease.ensure_valid_at(now)?;
        self.validated_materialization_candidate(input, latest_topology, now)?;
        let (path, committee, ordering_key) = self.materialization_plan(input)?;

        let proof = self.materialization_proof_for(input, now);
        let installation = self.installation_for(input, now, proof.clone());
        let active_route =
            self.active_route_for_materialization(input, path, committee, ordering_key);
        let installation = RouteInstallation {
            health: self.current_route_health(Some(&active_route), now),
            ..installation
        };
        self.store_checkpoint(&active_route)?;
        if let Err(error) = self
            .choreography_runtime()
            .activation_handshake(&route_id, input.handle.topology_epoch())
        {
            if let Some(previous_active_route) = previous_active_route.as_ref() {
                self.checkpoint_best_effort(previous_active_route);
            } else {
                let _ = self.remove_checkpoint(&route_id);
            }
            return Err(error);
        }
        self.active_routes.insert(route_id, active_route);
        Ok(installation)
    }
}
