//! Telltale definition for deferred hold and replay.
//!
//! Control flow intuition: a partitioned owner stores a held payload with a
//! holder, the holder announces storage to the observer, and the recipient
//! either replays immediately or defers, yielding a visible release outcome
//! back to the original owner.

use telltale::tell;

pub(crate) const SOURCE_PATH: &str = "crates/mesh/src/choreography/hold_replay.rs";
pub(crate) const PROTOCOL_NAME: &str = "HoldReplayExchange";
pub(crate) const ROLE_NAMES: &[&str] =
    &["PartitionedOwner", "Holder", "Recipient", "Observer"];

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    type MeshProtocolError =
      | Unavailable
      | Rejected
      | TimedOut

    type alias HeldPayload =
    {
      routeId : String
      payloadDigest : String
    }

    type alias HoldReceipt =
    {
      routeId : String
      storedBy : Role
    }

    effect MeshRuntime
      command storeHeldPayload : HeldPayload -> Result MeshProtocolError HoldReceipt
      {
        class : best_effort
        progress : immediate
        region : fragment
        agreement_use : none
        reentrancy : allow
      }
      command replayHeldPayload : HeldPayload -> Result MeshProtocolError HoldReceipt
      {
        class : best_effort
        progress : immediate
        region : fragment
        agreement_use : none
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

    protocol HoldReplayExchange uses MeshRuntime, MeshAudit under Replay =
      roles PartitionedOwner, Holder, Recipient, Observer
      PartitionedOwner -> Holder : StoreHeldPayload { routeId : String, payloadDigest : String }
      Holder -> Observer : Stored { routeId : String }
      Holder -> Recipient : ReplayHeldPayload { routeId : String, payloadDigest : String }
      choice Recipient at
        | Replayed =>
          Recipient -> Holder : ReplayAccepted { routeId : String }
          Holder -> PartitionedOwner : Released { routeId : String }
        | Deferred =>
          Recipient -> Holder : ReplayDeferred { routeId : String }
          Holder -> PartitionedOwner : StillHeld { routeId : String }
}
