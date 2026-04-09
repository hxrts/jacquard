//! Inline Telltale definition for route export exchange.
//!
//! Control flow: the exporting node offers a route-shaped summary to
//! a neighbor, the neighbor either publishes or ignores it, and the generated
//! branch structure becomes the only live sequencing path for that exchange.

use std::{error::Error, marker, result};

use jacquard_core::{RouteError, RouteId};
use telltale::{
    futures::{executor, try_join},
    tell, try_session,
};

use super::{
    effects::ChoreographyResultExt,
    runtime::{PathwayGuestRuntime, PathwayRouteExportSnapshot},
};

pub(crate) const SOURCE_PATH: &str = "crates/pathway/src/choreography/route_export.rs";
pub(crate) const PROTOCOL_NAME: &str = "RouteExportExchange";
pub(crate) const ROLE_NAMES: &[&str] = &["Exporter", "Neighbor", "Observer"];

type ProtocolResult<T> = result::Result<T, Box<dyn Error + marker::Send + Sync>>;

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    protocol RouteExportExchange under Replay =
      roles Exporter, Neighbor, Observer
      Exporter -> Neighbor : ExportRoute {
        routeId : String,
        hopCount : Int,
        routeClass : String,
        partitioned : Bool
      }
      choice Neighbor at
        | Published =>
          Neighbor -> Observer : Published { routeId : String }
        | Ignored =>
          Neighbor -> Observer : Ignored { routeId : String }
}

use RouteExportExchange::sessions::{
    ExportRoute, Exporter, ExporterSession, Ignored, Neighbor, NeighborSession,
    Observer, ObserverChoice1, ObserverSession, Published, Roles,
};

pub(crate) fn execute<E>(
    _runtime: &mut PathwayGuestRuntime<E>,
    route_id: &RouteId,
    snapshot: &PathwayRouteExportSnapshot,
) -> Result<&'static str, RouteError> {
    let route_id_hex = hex_bytes(&route_id.0);
    let hop_count = i64::from(snapshot.hop_count);
    let route_class = snapshot.route_class.clone();
    let partitioned = snapshot.partition_mode;
    let Roles(mut exporter, mut neighbor, mut observer) = Roles::default();

    executor::block_on(async {
        try_join!(
            exporter_role(
                &mut exporter,
                route_id_hex.clone(),
                hop_count,
                route_class.clone(),
                partitioned,
            ),
            neighbor_role(&mut neighbor, partitioned),
            observer_role(&mut observer),
        )
    })
    .map(|(_, _, detail)| detail)
    .choreography_failed()
}

async fn exporter_role(
    role: &mut Exporter,
    route_id: String,
    hop_count: i64,
    route_class: String,
    partitioned: bool,
) -> ProtocolResult<()> {
    try_session(role, |s: ExporterSession<'_, _>| async move {
        let end = s
            .send(ExportRoute {
                route_id,
                hop_count,
                route_class,
                partitioned,
            })
            .await?;
        Ok(((), end))
    })
    .await
}

async fn neighbor_role(role: &mut Neighbor, partitioned: bool) -> ProtocolResult<()> {
    try_session(role, |s: NeighborSession<'_, _>| async move {
        let (ExportRoute { route_id, .. }, s) = s.receive().await?;
        let end = if partitioned {
            s.select(Ignored { route_id }).await?
        } else {
            s.select(Published { route_id }).await?
        };
        Ok(((), end))
    })
    .await
}

async fn observer_role(role: &mut Observer) -> ProtocolResult<&'static str> {
    try_session(role, |s: ObserverSession<'_, _>| async {
        match s.branch().await? {
            | ObserverChoice1::Published(Published { .. }, end) => {
                Ok(("exported", end))
            },
            | ObserverChoice1::Ignored(Ignored { .. }, end) => Ok(("ignored", end)),
        }
    })
    .await
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
