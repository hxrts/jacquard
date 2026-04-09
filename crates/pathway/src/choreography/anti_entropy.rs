//! Inline Telltale definition for anti-entropy reconciliation exchange.
//!
//! Control flow: the current owner proposes reconciliation state to
//! a peer, the peer either syncs or defers, and the generated branch becomes
//! the only live sequencing path for mesh anti-entropy exchange.

use std::{error::Error, marker, result};

use jacquard_core::{RouteError, RouteId};
use telltale::{
    futures::{executor, try_join},
    tell, try_session,
};

use super::{
    effects::ChoreographyResultExt,
    runtime::{PathwayAntiEntropySnapshot, PathwayGuestRuntime},
};

pub(crate) const SOURCE_PATH: &str = "crates/pathway/src/choreography/anti_entropy.rs";
pub(crate) const PROTOCOL_NAME: &str = "AntiEntropyExchange";
pub(crate) const ROLE_NAMES: &[&str] = &["CurrentOwner", "Peer", "Observer"];

type ProtocolResult<T> = result::Result<T, Box<dyn Error + marker::Send + Sync>>;

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    protocol AntiEntropyExchange under Replay =
      roles CurrentOwner, Peer, Observer
      CurrentOwner -> Peer : Reconcile {
        routeId : String,
        retainedCount : Int,
        pressureScore : Int,
        partitioned : Bool
      }
      choice Peer at
        | Synced =>
          Peer -> Observer : Synced { routeId : String }
        | Deferred =>
          Peer -> Observer : Deferred { routeId : String }
}

use AntiEntropyExchange::sessions::{
    CurrentOwner, CurrentOwnerSession, Deferred, Observer, ObserverChoice1,
    ObserverSession, Peer, PeerSession, Reconcile, Roles, Synced,
};

pub(crate) fn execute<E>(
    _runtime: &mut PathwayGuestRuntime<E>,
    route_id: &RouteId,
    snapshot: &PathwayAntiEntropySnapshot,
) -> Result<&'static str, RouteError> {
    let route_id_hex = hex_bytes(&route_id.0);
    let retained_count = i64::from(snapshot.retained_count);
    let pressure_score = i64::from(snapshot.pressure_score.0);
    let partitioned = snapshot.partition_mode;
    let Roles(mut current_owner, mut peer, mut observer) = Roles::default();

    executor::block_on(async {
        try_join!(
            current_owner_role(
                &mut current_owner,
                route_id_hex.clone(),
                retained_count,
                pressure_score,
                partitioned,
            ),
            peer_role(&mut peer, retained_count, pressure_score, partitioned),
            observer_role(&mut observer),
        )
    })
    .map(|(_, _, detail)| detail)
    .choreography_failed()
}

async fn current_owner_role(
    role: &mut CurrentOwner,
    route_id: String,
    retained_count: i64,
    pressure_score: i64,
    partitioned: bool,
) -> ProtocolResult<()> {
    try_session(role, |s: CurrentOwnerSession<'_, _>| async move {
        let end = s
            .send(Reconcile {
                route_id,
                retained_count,
                pressure_score,
                partitioned,
            })
            .await?;
        Ok(((), end))
    })
    .await
}

async fn peer_role(
    role: &mut Peer,
    retained_count: i64,
    pressure_score: i64,
    partitioned: bool,
) -> ProtocolResult<()> {
    try_session(role, |s: PeerSession<'_, _>| async move {
        let (Reconcile { route_id, .. }, s) = s.receive().await?;
        let end = if partitioned || retained_count > 0 || pressure_score > 0 {
            s.select(Deferred { route_id }).await?
        } else {
            s.select(Synced { route_id }).await?
        };
        Ok(((), end))
    })
    .await
}

async fn observer_role(role: &mut Observer) -> ProtocolResult<&'static str> {
    try_session(role, |s: ObserverSession<'_, _>| async {
        match s.branch().await? {
            | ObserverChoice1::Synced(Synced { .. }, end) => {
                Ok(("anti-entropy-synced", end))
            },
            | ObserverChoice1::Deferred(Deferred { .. }, end) => {
                Ok(("anti-entropy-deferred", end))
            },
        }
    })
    .await
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
