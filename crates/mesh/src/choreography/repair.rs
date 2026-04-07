//! Inline Telltale definition for bounded suffix repair.
//!
//! Control flow intuition: the current owner proposes a repair through a
//! candidate relay, the destination accepts or rejects the offered suffix, and
//! the observer receives the same externally visible outcome.

use telltale::tell;

pub(crate) const SOURCE_PATH: &str = "crates/mesh/src/choreography/repair.rs";
pub(crate) const PROTOCOL_NAME: &str = "BoundedSuffixRepair";
pub(crate) const ROLE_NAMES: &[&str] =
    &["CurrentOwner", "CandidateRelay", "Destination", "Observer"];

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    type MeshProtocolError =
      | Unavailable
      | Rejected
      | TimedOut

    type alias RepairProposal =
    {
      routeId : String
      suffixEpoch : Int
    }

    type alias RepairReceipt =
    {
      routeId : String
      committedBy : Role
    }

    effect MeshRuntime
      authoritative proposeRepair : Session -> Result MeshProtocolError RepairProposal
      {
        class : authoritative
        progress : may_block
        region : fragment
        agreement_use : required
        reentrancy : reject_same_fragment
      }
      command commitRepair : RepairProposal -> Result MeshProtocolError RepairReceipt
      {
        class : best_effort
        progress : immediate
        region : fragment
        agreement_use : required
        reentrancy : allow
      }

    effect MeshAudit
      observe record : AuditEvent -> Unit
      {
        class : observational
        progress : immediate
        region : global
        agreement_use : forbidden
        reentrancy : allow
      }

    protocol BoundedSuffixRepair uses MeshRuntime, MeshAudit under Replay =
      roles CurrentOwner, CandidateRelay, Destination, Observer
      CurrentOwner -> CandidateRelay : RepairRequest { routeId : String }
      CandidateRelay -> Destination : RepairOffer { routeId : String }
      choice Destination at
        | Accepted =>
          Destination -> CandidateRelay : RepairAccepted { routeId : String }
          CandidateRelay -> CurrentOwner : RepairAccepted { routeId : String }
          CandidateRelay -> Observer : RepairAccepted { routeId : String }
        | Rejected =>
          Destination -> CandidateRelay : RepairRejected { routeId : String }
          CandidateRelay -> CurrentOwner : RepairRejected { routeId : String }
          CandidateRelay -> Observer : RepairRejected { routeId : String }
}
