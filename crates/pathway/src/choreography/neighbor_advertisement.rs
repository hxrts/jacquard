//! Inline Telltale definition for neighbor advertisement exchange.
//!
//! Control flow: the local node advertises coarse neighbor-facing
//! capability state, the neighbor either sees or ignores it, and the observer
//! receives that same visible outcome through the generated branch. The
//! three-role `NeighborAdvertisementExchange` protocol carries a small
//! capability snapshot: service count and adjacent neighbor count. The
//! neighbor branch resolves to `Seen` when service count is positive and
//! `Ignored` otherwise. The `execute` entry point is called from the pathway
//! tick runtime and returns the observer-branch detail string so the caller
//! can emit a structured protocol observation without reading session types
//! directly. Protocol constants are exported for the artifacts catalog.

use std::{error::Error, marker, result};

use jacquard_core::{RouteEpoch, RouteError};
use telltale::{
    futures::{executor, try_join},
    tell, try_session,
};

use super::{
    effects::ChoreographyResultExt,
    runtime::{PathwayGuestRuntime, PathwayNeighborAdvertisementSnapshot},
};

pub(crate) const SOURCE_PATH: &str = "crates/pathway/src/choreography/neighbor_advertisement.rs";
pub(crate) const PROTOCOL_NAME: &str = "NeighborAdvertisementExchange";
pub(crate) const ROLE_NAMES: &[&str] = &["LocalNode", "Neighbor", "Observer"];

type ProtocolResult<T> = result::Result<T, Box<dyn Error + marker::Send + Sync>>;

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    protocol NeighborAdvertisementExchange under Replay =
      roles LocalNode, Neighbor, Observer
      LocalNode -> Neighbor : AdvertiseNeighbor {
        nodeId : String,
        serviceCount : Int,
        adjacentNeighborCount : Int
      }
      choice Neighbor at
        | Seen =>
          Neighbor -> Observer : Seen { nodeId : String }
        | Ignored =>
          Neighbor -> Observer : Ignored { nodeId : String }
}

use NeighborAdvertisementExchange::sessions::{
    AdvertiseNeighbor, Ignored, LocalNode, LocalNodeSession, Neighbor, NeighborSession, Observer,
    ObserverChoice1, ObserverSession, Roles, Seen,
};

pub(crate) fn execute<E>(
    _runtime: &mut PathwayGuestRuntime<E>,
    _epoch: RouteEpoch,
    snapshot: &PathwayNeighborAdvertisementSnapshot,
) -> Result<&'static str, RouteError> {
    let node_id = hex_bytes(&snapshot.local_node_id.0);
    let service_count = i64::from(snapshot.service_count);
    let adjacent_neighbor_count = i64::from(snapshot.adjacent_neighbor_count);
    let Roles(mut local_node, mut neighbor, mut observer) = Roles::default();

    executor::block_on(async {
        try_join!(
            local_node_role(
                &mut local_node,
                node_id.clone(),
                service_count,
                adjacent_neighbor_count,
            ),
            neighbor_role(&mut neighbor, service_count),
            observer_role(&mut observer),
        )
    })
    .map(|(_, _, detail)| detail)
    .choreography_failed()
}

async fn local_node_role(
    role: &mut LocalNode,
    node_id: String,
    service_count: i64,
    adjacent_neighbor_count: i64,
) -> ProtocolResult<()> {
    try_session(role, |s: LocalNodeSession<'_, _>| async move {
        let end = s
            .send(AdvertiseNeighbor {
                node_id,
                service_count,
                adjacent_neighbor_count,
            })
            .await?;
        Ok(((), end))
    })
    .await
}

async fn neighbor_role(role: &mut Neighbor, service_count: i64) -> ProtocolResult<()> {
    try_session(role, |s: NeighborSession<'_, _>| async move {
        let (AdvertiseNeighbor { node_id, .. }, s) = s.receive().await?;
        let end = if service_count > 0 {
            s.select(Seen { node_id }).await?
        } else {
            s.select(Ignored { node_id }).await?
        };
        Ok(((), end))
    })
    .await
}

async fn observer_role(role: &mut Observer) -> ProtocolResult<&'static str> {
    try_session(role, |s: ObserverSession<'_, _>| async {
        match s.branch().await? {
            ObserverChoice1::Seen(Seen { .. }, end) => Ok(("advertised", end)),
            ObserverChoice1::Ignored(Ignored { .. }, end) => Ok(("advertisement-ignored", end)),
        }
    })
    .await
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
