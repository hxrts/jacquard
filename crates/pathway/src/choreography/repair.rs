//! Inline Telltale definition for bounded suffix repair.
//!
//! Control flow: the current owner proposes a repair through a
//! candidate relay, and the destination accepts or rejects the offered suffix.
//! The generated session code owns that visible branch structure.

use std::{error::Error, marker, result};

use jacquard_core::{RouteError, RouteId};
use telltale::{
    futures::{executor, try_join},
    tell, try_session,
};

pub(crate) const SOURCE_PATH: &str = "crates/pathway/src/choreography/repair.rs";
pub(crate) const PROTOCOL_NAME: &str = "BoundedSuffixRepair";
pub(crate) const ROLE_NAMES: &[&str] =
    &["CurrentOwner", "CandidateRelay", "Destination"];

type ProtocolResult<T> = result::Result<T, Box<dyn Error + marker::Send + Sync>>;

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    type PathwayProtocolError =
      | Unavailable
      | Rejected
      | TimedOut

    protocol BoundedSuffixRepair under Replay =
      roles CurrentOwner, CandidateRelay, Destination
      CurrentOwner -> CandidateRelay : RepairRequest { routeId : String }
      CandidateRelay -> Destination : RepairOffer { routeId : String }
      choice Destination at
        | RepairAccepted =>
          Destination -> CandidateRelay : RepairAccepted { routeId : String }
        | RepairRejected =>
          Destination -> CandidateRelay : RepairRejected { routeId : String }
}

use BoundedSuffixRepair::sessions::{
    CandidateRelay, CandidateRelayChoice1, CandidateRelaySession, CurrentOwner,
    CurrentOwnerSession, Destination, DestinationSession, RepairAccepted, RepairOffer,
    RepairRejected, RepairRequest, Roles,
};

use super::{
    effects::{ChoreographyResultExt, PathwayProtocolRuntime},
    runtime::PathwayGuestRuntime,
};

pub(crate) fn execute<E>(
    _runtime: &mut PathwayGuestRuntime<E>,
    route_id: &RouteId,
) -> Result<(), RouteError>
where
    E: PathwayProtocolRuntime,
{
    let route_id = hex_bytes(&route_id.0);
    let Roles(mut current_owner, mut candidate_relay, mut destination) =
        Roles::default();

    executor::block_on(async {
        try_join!(
            current_owner_role(&mut current_owner, route_id.clone()),
            candidate_relay_role(&mut candidate_relay),
            destination_role(&mut destination),
        )
    })
    .map(|_| ())
    .choreography_failed()
}

async fn current_owner_role(
    role: &mut CurrentOwner,
    route_id: String,
) -> ProtocolResult<()> {
    try_session(role, |s: CurrentOwnerSession<'_, _>| async move {
        let end = s.send(RepairRequest { route_id }).await?;
        Ok(((), end))
    })
    .await
}

async fn candidate_relay_role(role: &mut CandidateRelay) -> ProtocolResult<()> {
    try_session(role, |s: CandidateRelaySession<'_, _>| async {
        let (RepairRequest { route_id }, s) = s.receive().await?;
        let s = s.send(RepairOffer { route_id: route_id.clone() }).await?;
        match s.branch().await? {
            | CandidateRelayChoice1::RepairAccepted(
                RepairAccepted { route_id: _ },
                end,
            ) => Ok(((), end)),
            | CandidateRelayChoice1::RepairRejected(
                RepairRejected { route_id: _ },
                end,
            ) => Ok(((), end)),
        }
    })
    .await
}

async fn destination_role(role: &mut Destination) -> ProtocolResult<()> {
    try_session(role, |s: DestinationSession<'_, _>| async {
        let (RepairOffer { route_id }, s) = s.receive().await?;
        let end = s.select(RepairAccepted { route_id }).await?;
        Ok(((), end))
    })
    .await
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
