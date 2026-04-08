//! Inline Telltale definition for semantic ownership handoff.
//!
//! Control flow: the old owner offers transfer to the new owner, and
//! the new owner accepts or rejects. The generated session code owns that
//! visible ownership branch structure.

use std::{error::Error, marker, result};

use jacquard_core::{RouteError, RouteId};
use telltale::{
    futures::{executor, try_join},
    tell, try_session,
};

pub(crate) const SOURCE_PATH: &str = "crates/mesh/src/choreography/handoff.rs";
pub(crate) const PROTOCOL_NAME: &str = "SemanticHandoff";
pub(crate) const ROLE_NAMES: &[&str] = &["OldOwner", "NewOwner"];

type ProtocolResult<T> = result::Result<T, Box<dyn Error + marker::Send + Sync>>;

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    type MeshProtocolError =
      | Unavailable
      | Rejected
      | TimedOut

    protocol SemanticHandoff under Replay =
      roles OldOwner, NewOwner
      OldOwner -> NewOwner : Transfer { routeId : String }
      choice NewOwner at
        | TransferAccepted =>
          NewOwner -> OldOwner : TransferAccepted { routeId : String }
        | TransferRejected =>
          NewOwner -> OldOwner : TransferRejected { routeId : String }
}

use SemanticHandoff::sessions::{
    NewOwner, NewOwnerSession, OldOwner, OldOwnerChoice1, OldOwnerSession, Roles,
    Transfer, TransferAccepted, TransferRejected,
};

use super::{
    effects::{ChoreographyResultExt, MeshProtocolRuntime},
    runtime::MeshGuestRuntime,
};

pub(crate) fn execute<E>(
    _runtime: &mut MeshGuestRuntime<E>,
    route_id: &RouteId,
) -> Result<(), RouteError>
where
    E: MeshProtocolRuntime,
{
    let route_id = hex_bytes(&route_id.0);
    let Roles(mut old_owner, mut new_owner) = Roles::default();

    executor::block_on(async {
        try_join!(
            old_owner_role(&mut old_owner, route_id.clone()),
            new_owner_role(&mut new_owner),
        )
    })
    .map(|_| ())
    .choreography_failed()
}

async fn old_owner_role(role: &mut OldOwner, route_id: String) -> ProtocolResult<()> {
    try_session(role, |s: OldOwnerSession<'_, _>| async move {
        let s = s.send(Transfer { route_id }).await?;
        match s.branch().await? {
            | OldOwnerChoice1::TransferAccepted(
                TransferAccepted { route_id: _ },
                end,
            ) => Ok(((), end)),
            | OldOwnerChoice1::TransferRejected(
                TransferRejected { route_id: _ },
                end,
            ) => Ok(((), end)),
        }
    })
    .await
}

async fn new_owner_role(role: &mut NewOwner) -> ProtocolResult<()> {
    try_session(role, |s: NewOwnerSession<'_, _>| async {
        let (Transfer { route_id }, s) = s.receive().await?;
        let end = s.select(TransferAccepted { route_id }).await?;
        Ok(((), end))
    })
    .await
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
