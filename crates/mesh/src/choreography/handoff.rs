//! Inline Telltale definition for semantic ownership handoff.
//!
//! Control flow intuition: the old owner offers transfer to the new owner, the
//! new owner accepts or rejects, and both the router and observer learn the
//! visible ownership outcome from the same generated branch structure.

use telltale::tell;

pub(crate) const SOURCE_PATH: &str = "crates/mesh/src/choreography/handoff.rs";
pub(crate) const PROTOCOL_NAME: &str = "SemanticHandoff";
pub(crate) const ROLE_NAMES: &[&str] = &["OldOwner", "NewOwner", "Router", "Observer"];

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    type MeshProtocolError =
      | Unavailable
      | Rejected
      | TimedOut

    type alias TransferReceipt =
    {
      routeId : String
      acceptedBy : Role
    }

    effect MeshRuntime
      authoritative prepareTransfer : Session -> Result MeshProtocolError TransferReceipt
      {
        class : authoritative
        progress : may_block
        region : fragment
        agreement_use : required
        reentrancy : reject_same_fragment
      }
      command commitTransfer : TransferReceipt -> Result MeshProtocolError TransferReceipt
      {
        class : best_effort
        progress : immediate
        region : session
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

    protocol SemanticHandoff uses MeshRuntime, MeshAudit under Replay =
      roles OldOwner, NewOwner, Router, Observer
      OldOwner -> NewOwner : Transfer { routeId : String }
      choice NewOwner at
        | Accepted =>
          NewOwner -> OldOwner : TransferAccepted { routeId : String }
          NewOwner -> Router : OwnershipMoved { routeId : String }
          NewOwner -> Observer : OwnershipMoved { routeId : String }
        | Rejected =>
          NewOwner -> OldOwner : TransferRejected { routeId : String }
          NewOwner -> Router : TransferRejected { routeId : String }
          NewOwner -> Observer : TransferRejected { routeId : String }
}
