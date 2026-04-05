//! Committee-selection results shared across routing engines.

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    Belief, ClaimStrength, CommitteeId, ControllerId, FactBasis, HealthScore,
    IdentityAssuranceClass, NodeId, RouteEpoch, Tick, TimeWindow,
};

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Committee role exposed to the shared control plane.
///
/// No distinguished role is required. Leaderless protocols may assign every
/// member `Participant` or use only witness/relay roles.
pub enum CommitteeRole {
    Participant,
    Relay,
    Witness,
    Facilitator,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// One selected committee member plus its declared role.
pub struct CommitteeMember {
    pub node_id: NodeId,
    pub controller_id: ControllerId,
    pub role: CommitteeRole,
    pub trust_score: Belief<HealthScore>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Routing-engine-selected coordination group.
///
/// The selection policy remains engine-local. This shared object only exposes
/// the resulting membership, lease, and evidentiary posture to the rest of the
/// control plane.
pub struct CommitteeSelection {
    pub committee_id: CommitteeId,
    pub topology_epoch: RouteEpoch,
    pub selected_at_tick: Tick,
    pub valid_for: TimeWindow,
    pub evidence_basis: FactBasis,
    pub claim_strength: ClaimStrength,
    pub identity_assurance: IdentityAssuranceClass,
    pub quorum_threshold: u8,
    /// Bounded by [`PROVIDER_CANDIDATE_COUNT_MAX`](crate::PROVIDER_CANDIDATE_COUNT_MAX).
    pub members: Vec<CommitteeMember>,
}
