//! Committee-selection results and membership types shared across routing
//! engines.
//!
//! This module defines the shared types that represent the output of a routing
//! engine's committee-selection process. The selection policy itself is
//! engine-local; only the resulting membership, lease, and evidentiary posture
//! are exposed through these shared types.
//!
//! [`CommitteeRole`] declares the roles available to members — leaderless
//! protocols may assign `Participant` to all members. [`CommitteeMember`]
//! pairs a node and controller identity with its declared role. The top-level
//! [`CommitteeSelection`] bundles the committee identity, topology epoch,
//! selection tick, validity window, evidence basis, claim strength, identity
//! assurance posture, quorum threshold, and the bounded member list. Member
//! count is bounded by
//! [`PROVIDER_CANDIDATE_COUNT_MAX`](crate::PROVIDER_CANDIDATE_COUNT_MAX).

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::{
    ClaimStrength, CommitteeId, ControllerId, FactBasis, IdentityAssuranceClass,
    NodeId, RouteEpoch, Tick, TimeWindow,
};

#[id_type]
pub struct QuorumThreshold(pub u8);

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
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
    pub quorum_threshold: QuorumThreshold,
    /// Bounded by
    /// [`PROVIDER_CANDIDATE_COUNT_MAX`](crate::PROVIDER_CANDIDATE_COUNT_MAX).
    pub members: Vec<CommitteeMember>,
}
