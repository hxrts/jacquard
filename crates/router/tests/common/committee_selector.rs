//! `AdvisoryCommitteeSelector` — a configurable committee selector stub
//! that either returns a single-member committee or deliberately fails,
//! used to exercise router fail-closed and committee paths.
//!
//! The selector wraps a single boolean `fail` field. When `fail` is `false`
//! it produces a minimal `CommitteeSelection` with `LOCAL_NODE_ID` as the
//! sole `Leader` member, satisfying the router's proof-bearing activation
//! requirement. When `fail` is `true` it returns
//! `RouteAdmissionRejection::BackendUnavailable` so tests can assert that
//! the router refuses to publish canonical route truth when the committee
//! layer is unavailable.
//!
//! Used by `router_builder::build_router_with_selector` and the fail-closed
//! integration tests in `router_fail_closed`.

use jacquard_core::{
    ClaimStrength, CommitteeId, CommitteeMember, CommitteeRole, CommitteeSelection, Configuration,
    ControllerId, FactBasis, IdentityAssuranceClass, Observation, QuorumThreshold,
    RoutingObjective, SelectedRoutingParameters, Tick, TimeWindow,
};
use jacquard_traits::CommitteeSelector;

use super::fixtures::LOCAL_NODE_ID;

#[derive(Clone, Copy)]
pub(crate) struct AdvisoryCommitteeSelector {
    pub(crate) fail: bool,
}

impl CommitteeSelector for AdvisoryCommitteeSelector {
    type TopologyView = Configuration;

    fn select_committee(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, jacquard_core::RouteError> {
        if self.fail {
            return Err(jacquard_core::RouteSelectionError::Inadmissible(
                jacquard_core::RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        Ok(Some(CommitteeSelection {
            committee_id: CommitteeId([4; 16]),
            topology_epoch: topology.value.epoch,
            selected_at_tick: topology.observed_at_tick,
            valid_for: TimeWindow::new(
                topology.observed_at_tick,
                Tick(topology.observed_at_tick.0.saturating_add(8)),
            )
            .expect("committee window"),
            evidence_basis: FactBasis::Observed,
            claim_strength: ClaimStrength::ConservativeUnderProfile,
            identity_assurance: IdentityAssuranceClass::ControllerBound,
            quorum_threshold: QuorumThreshold(1),
            members: vec![CommitteeMember {
                node_id: LOCAL_NODE_ID,
                controller_id: ControllerId([1; 32]),
                role: CommitteeRole::Participant,
            }],
        }))
    }
}
