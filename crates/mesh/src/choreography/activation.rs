//! Inline Telltale definition for mesh route activation.
//!
//! Control flow intuition: the router asks the current owner to activate a
//! route, the owner prepares the next hop, and the destination either accepts
//! or rejects the route. The generated protocol surface owns that visible
//! handshake shape; mesh runtime code only decides when to enter it.

use telltale::tell;

pub(crate) const SOURCE_PATH: &str = "crates/mesh/src/choreography/activation.rs";
pub(crate) const PROTOCOL_NAME: &str = "ActivationHandshake";
pub(crate) const ROLE_NAMES: &[&str] =
    &["Router", "CurrentOwner", "NextHop", "Destination"];

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    type MeshProtocolError =
      | Unavailable
      | Rejected
      | TimedOut

    type alias RouteTicket =
    {
      routeId : String
      epoch : Int
    }

    type alias ActivationReceipt =
    {
      routeId : String
      acceptedBy : Role
    }

    effect MeshRuntime
      authoritative prepareActivation : Session -> Result MeshProtocolError RouteTicket
      {
        class : authoritative
        progress : may_block
        region : fragment
        agreement_use : required
        reentrancy : reject_same_fragment
      }
      command publishActivation : RouteTicket -> Result MeshProtocolError ActivationReceipt
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

    protocol ActivationHandshake uses MeshRuntime, MeshAudit under Replay =
      roles Router, CurrentOwner, NextHop, Destination
      Router -> CurrentOwner : Activate { routeId : String, epoch : Int }
      CurrentOwner -> NextHop : Prepare { routeId : String }
      NextHop -> Destination : Offer { routeId : String }
      choice Destination at
        | Accepted =>
          Destination -> NextHop : Accepted { routeId : String }
          NextHop -> CurrentOwner : Activated { routeId : String }
          CurrentOwner -> Router : Activated { routeId : String }
        | Rejected =>
          Destination -> NextHop : Rejected { routeId : String }
          NextHop -> CurrentOwner : Rejected { routeId : String }
          CurrentOwner -> Router : Rejected { routeId : String }
}
