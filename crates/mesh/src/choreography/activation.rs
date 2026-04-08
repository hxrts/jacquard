//! Inline Telltale definition for mesh route activation.
//!
//! Control flow intuition: the router asks the current owner to activate a
//! route, the owner prepares the next hop, and the destination either accepts
//! or rejects. The generated session code owns that visible handshake shape;
//! mesh runtime code only decides when to enter it.

use std::{error::Error, marker, result};

use jacquard_core::{RouteEpoch, RouteError, RouteId, RouteRuntimeError};
use telltale::{
    futures::{executor, try_join},
    tell, try_session,
};

pub(crate) const SOURCE_PATH: &str = "crates/mesh/src/choreography/activation.rs";
pub(crate) const PROTOCOL_NAME: &str = "ActivationHandshake";
pub(crate) const ROLE_NAMES: &[&str] =
    &["Router", "CurrentOwner", "NextHop", "Destination"];

type ProtocolResult<T> = result::Result<T, Box<dyn Error + marker::Send + Sync>>;

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    type MeshProtocolError =
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

use super::{effects::MeshProtocolRuntime, runtime::MeshGuestRuntime};

pub(crate) fn execute<E>(
    _runtime: &mut MeshGuestRuntime<E>,
    route_id: &RouteId,
    epoch: RouteEpoch,
) -> Result<(), RouteError>
where
    E: MeshProtocolRuntime,
{
    let epoch = i64::try_from(epoch.0)
        .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))?;
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
    .map_err(|_| RouteError::Runtime(RouteRuntimeError::MaintenanceFailed))
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
