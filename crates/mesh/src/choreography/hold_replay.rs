//! Telltale definition for deferred hold and replay.
//!
//! Control flow intuition: a partitioned owner stores a held payload with a
//! holder, the holder announces storage to the observer, and the recipient
//! either replays immediately or defers. Mesh keeps the owner-visible retained
//! object accounting in ordinary route runtime state rather than sending a
//! second protocol-level tail message back to the owner.

use std::{cell::RefCell, error::Error, marker, rc::Rc, result};

use jacquard_core::{ContentId, LinkEndpoint, RouteError, RouteId, RouteRuntimeError};
use serde_json::json;
use telltale::{
    futures::{executor, try_join},
    tell, try_session,
};

pub(crate) const SOURCE_PATH: &str = "crates/mesh/src/choreography/hold_replay.rs";
pub(crate) const PROTOCOL_NAME: &str = "HoldReplayExchange";
pub(crate) const ROLE_NAMES: &[&str] =
    &["PartitionedOwner", "Holder", "Recipient", "Observer"];

type ProtocolResult<T> = result::Result<T, Box<dyn Error + marker::Send + Sync>>;

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
        | Deferred =>
          Recipient -> Holder : ReplayDeferred { routeId : String }
}

use HoldReplayExchange::{
    effects,
    sessions::{
        Deferred, Holder, HolderChoice1, HolderSession, Observer, ObserverSession,
        PartitionedOwner, PartitionedOwnerSession, Recipient, RecipientSession,
        ReplayAccepted, ReplayDeferred, ReplayHeldPayload, Replayed, Roles,
        StoreHeldPayload, Stored,
    },
};

use super::{
    artifacts::{protocol_spec, MeshProtocolKind},
    effects::{
        MeshChoreoFrame, MeshHeldPayload, MeshProtocolObservation, MeshProtocolRuntime,
    },
    runtime::{route_session, MeshGuestRuntime},
};

struct SharedRuntime<'a, E> {
    effects: &'a mut E,
    route_id: RouteId,
    object_id: ContentId<jacquard_core::Blake3Digest>,
    payload: Vec<u8>,
    endpoint: Option<LinkEndpoint>,
}

#[derive(Clone, Copy)]
enum ReplayDisposition {
    Deferred,
    Replayed,
}

struct HolderHost<'a, E> {
    shared: Rc<RefCell<SharedRuntime<'a, E>>>,
}

struct ObserverHost<'a, E> {
    shared: Rc<RefCell<SharedRuntime<'a, E>>>,
}

impl<E> effects::MeshRuntime for HolderHost<'_, E>
where
    E: MeshProtocolRuntime,
{
    fn store_held_payload(
        &mut self,
        input: effects::HeldPayload,
    ) -> Result<effects::HoldReceipt, effects::MeshProtocolError> {
        let mut shared = self.shared.borrow_mut();
        let object_id = shared.object_id;
        let payload = shared.payload.clone();
        shared
            .effects
            .store_held_payload(&MeshHeldPayload { object_id, payload })
            .map_err(|_| effects::MeshProtocolError::Unavailable)?;
        Ok(effects::HoldReceipt {
            route_id: input.route_id,
            stored_by: effects::Role::new("Holder"),
        })
    }

    fn replay_held_payload(
        &mut self,
        input: effects::HeldPayload,
    ) -> Result<effects::HoldReceipt, effects::MeshProtocolError> {
        let mut shared = self.shared.borrow_mut();
        let object_id = shared.object_id;
        let payload = shared.payload.clone();
        let endpoint = shared.endpoint.clone();
        shared
            .effects
            .replay_held_payload(&MeshHeldPayload {
                object_id,
                payload: payload.clone(),
            })
            .map_err(|_| effects::MeshProtocolError::Unavailable)?;
        if let Some(endpoint) = endpoint {
            shared
                .effects
                .send_mesh_frame(&MeshChoreoFrame { endpoint, payload })
                .map_err(|_| effects::MeshProtocolError::Unavailable)?;
        }
        Ok(effects::HoldReceipt {
            route_id: input.route_id,
            stored_by: effects::Role::new("Holder"),
        })
    }
}

impl<E> effects::MeshAudit for ObserverHost<'_, E>
where
    E: MeshProtocolRuntime,
{
    fn record(&mut self, input: effects::AuditEvent) {
        let event = input.get("event").and_then(serde_json::Value::as_str);
        let detail = match event {
            | Some("generated-stored") => "generated-stored",
            | Some("generated-released") => "generated-released",
            | Some("generated-still-held") => "generated-still-held",
            | _ => "generated-observed",
        };
        let Ok(spec) = protocol_spec(MeshProtocolKind::HoldReplay) else {
            return;
        };
        let mut shared = self.shared.borrow_mut();
        let route_id = shared.route_id;
        shared
            .effects
            .emit_protocol_observation(MeshProtocolObservation {
                protocol: MeshProtocolKind::HoldReplay,
                protocol_name: spec.protocol_name.clone(),
                role_names: spec.role_names.clone(),
                session: route_session(MeshProtocolKind::HoldReplay, &route_id),
                detail,
            });
    }
}

pub(crate) fn retain<E>(
    runtime: &mut MeshGuestRuntime<E>,
    route_id: &RouteId,
    object_id: ContentId<jacquard_core::Blake3Digest>,
    payload: &[u8],
) -> Result<(), RouteError>
where
    E: MeshProtocolRuntime,
{
    execute(
        runtime,
        route_id,
        object_id,
        payload.to_vec(),
        None,
        ReplayDisposition::Deferred,
    )
}

pub(crate) fn replay<E>(
    runtime: &mut MeshGuestRuntime<E>,
    route_id: &RouteId,
    object_id: ContentId<jacquard_core::Blake3Digest>,
    endpoint: LinkEndpoint,
    payload: Vec<u8>,
) -> Result<(), RouteError>
where
    E: MeshProtocolRuntime,
{
    execute(
        runtime,
        route_id,
        object_id,
        payload,
        Some(endpoint),
        ReplayDisposition::Replayed,
    )
}

fn execute<E>(
    runtime: &mut MeshGuestRuntime<E>,
    route_id: &RouteId,
    object_id: ContentId<jacquard_core::Blake3Digest>,
    payload: Vec<u8>,
    endpoint: Option<LinkEndpoint>,
    disposition: ReplayDisposition,
) -> Result<(), RouteError>
where
    E: MeshProtocolRuntime,
{
    let route_id_hex = hex_bytes(&route_id.0);
    let payload_digest = hex_bytes(&payload);
    let shared = Rc::new(RefCell::new(SharedRuntime {
        effects: runtime.protocol_runtime_mut(),
        route_id: *route_id,
        object_id,
        payload,
        endpoint,
    }));
    let Roles(mut owner, mut holder, mut recipient, mut observer) = Roles::default();
    let mut holder_host = HolderHost { shared: Rc::clone(&shared) };
    let mut observer_host = ObserverHost { shared };

    executor::block_on(async {
        try_join!(
            owner_role(&mut owner, route_id_hex.clone(), payload_digest.clone()),
            holder_role(&mut holder, &mut holder_host),
            recipient_role(&mut recipient, disposition),
            observer_role(&mut observer, &mut observer_host),
        )
    })
    .map(|_| ())
    .map_err(|_| RouteError::Runtime(RouteRuntimeError::MaintenanceFailed))
}

async fn owner_role(
    role: &mut PartitionedOwner,
    route_id: String,
    payload_digest: String,
) -> ProtocolResult<()> {
    try_session(role, |s: PartitionedOwnerSession<'_, _>| async move {
        let end = s
            .send(StoreHeldPayload { route_id, payload_digest })
            .await?;
        Ok(((), end))
    })
    .await
}

async fn holder_role<E>(
    role: &mut Holder,
    host: &mut HolderHost<'_, E>,
) -> ProtocolResult<()>
where
    E: MeshProtocolRuntime,
{
    try_session(role, |s: HolderSession<'_, _>| async {
        let (StoreHeldPayload { route_id, payload_digest }, s) = s.receive().await?;
        let held_payload = effects::HeldPayload {
            route_id: route_id.clone(),
            payload_digest: payload_digest.clone(),
        };
        // Intentionally ignored: store_held_payload is a best-effort side effect.
        // The session type enforces protocol continuation regardless; if retention
        // fails here the payload will not be replayed but the hold exchange still
        // completes correctly.
        let _ = effects::MeshRuntime::store_held_payload(host, held_payload.clone());
        let s = s.send(Stored { route_id: route_id.clone() }).await?;
        let s = s
            .send(ReplayHeldPayload {
                route_id: route_id.clone(),
                payload_digest,
            })
            .await?;
        match s.branch().await? {
            | HolderChoice1::Replayed(Replayed, s) => {
                let (ReplayAccepted { route_id }, s) = s.receive().await?;
                // Intentionally ignored: replay_held_payload is best-effort. The
                // session already advanced to the Replayed branch;
                // retention cleanup is advisory.
                let _ = effects::MeshRuntime::replay_held_payload(
                    host,
                    effects::HeldPayload {
                        route_id: route_id.clone(),
                        payload_digest: String::new(),
                    },
                );
                Ok(((), s))
            },
            | HolderChoice1::Deferred(Deferred, s) => {
                let (ReplayDeferred { route_id }, s) = s.receive().await?;
                let _ = route_id;
                Ok(((), s))
            },
        }
    })
    .await
}

async fn recipient_role(
    role: &mut Recipient,
    disposition: ReplayDisposition,
) -> ProtocolResult<()> {
    try_session(role, |s: RecipientSession<'_, _>| async move {
        let (ReplayHeldPayload { route_id, .. }, s) = s.receive().await?;
        let end = match disposition {
            | ReplayDisposition::Deferred => {
                let s = s.select(Deferred).await?;
                s.send(ReplayDeferred { route_id }).await?
            },
            | ReplayDisposition::Replayed => {
                let s = s.select(Replayed).await?;
                s.send(ReplayAccepted { route_id }).await?
            },
        };
        Ok(((), end))
    })
    .await
}

async fn observer_role<E>(
    role: &mut Observer,
    host: &mut ObserverHost<'_, E>,
) -> ProtocolResult<()>
where
    E: MeshProtocolRuntime,
{
    try_session(role, |s: ObserverSession<'_, _>| async {
        let (Stored { route_id }, end) = s.receive().await?;
        effects::MeshAudit::record(
            host,
            json!({ "event": "generated-stored", "route_id": route_id }),
        );
        Ok(((), end))
    })
    .await
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
