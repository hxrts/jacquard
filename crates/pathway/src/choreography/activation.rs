//! Inline Telltale definition for pathway route activation.
//!
//! Control flow: the router asks the current owner to activate a
//! route, the owner prepares the next hop, and the destination either accepts
//! or rejects. The generated session code owns that visible handshake shape;
//! pathway runtime code only decides when to enter it. The four-role
//! `ActivationHandshake` protocol is declared via `tell!` and executed
//! synchronously through `executor::block_on`. The `execute` entry point is
//! called from the pathway guest runtime when the router triggers route
//! activation. Protocol constants (`SOURCE_PATH`, `PROTOCOL_NAME`,
//! `ROLE_NAMES`) are exported for the artifacts catalog so checkpoints and
//! observations can resolve protocol metadata without depending on generated
//! session types directly.

use std::{error::Error, marker, result};

use jacquard_core::{RouteEpoch, RouteError, RouteId};
use telltale::{
    futures::{executor, try_join},
    tell, try_session,
};

pub(crate) const SOURCE_PATH: &str = "crates/pathway/src/choreography/activation.rs";
pub(crate) const PROTOCOL_NAME: &str = "ActivationHandshake";
pub(crate) const ROLE_NAMES: &[&str] =
    &["Router", "CurrentOwner", "NextHop", "Destination"];

type ProtocolResult<T> = result::Result<T, Box<dyn Error + marker::Send + Sync>>;

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    type PathwayProtocolError =
      | Unavailable
      | Rejected
      | TimedOut

    protocol ActivationHandshake under Replay =
      roles Router, CurrentOwner, NextHop, Destination
      Router -> CurrentOwner : Activate { routeId : String, epoch : Int }
      CurrentOwner -> NextHop : Prepare { routeId : String }
      NextHop -> Destination : Offer { routeId : String }
      choice Destination at
        | Activated =>
          Destination -> NextHop : Activated { routeId : String }
        | Rejected =>
          Destination -> NextHop : Rejected { routeId : String }
}

use ActivationHandshake::sessions::{
    Activate, Activated, CurrentOwner, CurrentOwnerSession, Destination,
    DestinationSession, NextHop, NextHopChoice1, NextHopSession, Offer, Prepare,
    Rejected, Roles, Router, RouterSession,
};

use super::{
    effects::{ChoreographyResultExt, InvalidatedResultExt, PathwayProtocolRuntime},
    runtime::PathwayGuestRuntime,
};

pub(crate) fn execute<E>(
    _runtime: &mut PathwayGuestRuntime<E>,
    route_id: &RouteId,
    epoch: RouteEpoch,
) -> Result<(), RouteError>
where
    E: PathwayProtocolRuntime,
{
    let epoch = i64::try_from(epoch.0).invalidated()?;
    let route_id = hex_bytes(&route_id.0);
    let Roles(mut router, mut current_owner, mut next_hop, mut destination) =
        Roles::default();

    executor::block_on(async {
        try_join!(
            router_role(&mut router, route_id.clone(), epoch),
            current_owner_role(&mut current_owner),
            next_hop_role(&mut next_hop),
            destination_role(&mut destination),
        )
    })
    .map(|_| ())
    .choreography_failed()
}

async fn router_role(
    role: &mut Router,
    route_id: String,
    epoch: i64,
) -> ProtocolResult<()> {
    try_session(role, |s: RouterSession<'_, _>| async move {
        let end = s.send(Activate { route_id, epoch }).await?;
        Ok(((), end))
    })
    .await
}

async fn current_owner_role(role: &mut CurrentOwner) -> ProtocolResult<()> {
    try_session(role, |s: CurrentOwnerSession<'_, _>| async {
        let (Activate { route_id, .. }, s) = s.receive().await?;
        let end = s.send(Prepare { route_id: route_id.clone() }).await?;
        Ok(((), end))
    })
    .await
}

async fn next_hop_role(role: &mut NextHop) -> ProtocolResult<()> {
    try_session(role, |s: NextHopSession<'_, _>| async {
        let (Prepare { route_id }, s) = s.receive().await?;
        let s = s.send(Offer { route_id }).await?;
        match s.branch().await? {
            | NextHopChoice1::Activated(Activated { route_id: _ }, end) => {
                Ok(((), end))
            },
            | NextHopChoice1::Rejected(Rejected { route_id: _ }, s) => Ok(((), s)),
        }
    })
    .await
}

async fn destination_role(role: &mut Destination) -> ProtocolResult<()> {
    try_session(role, |s: DestinationSession<'_, _>| async {
        let (Offer { route_id }, s) = s.receive().await?;
        let end = s.select(Activated { route_id }).await?;
        Ok(((), end))
    })
    .await
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
