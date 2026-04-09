//! Inline Telltale definition and generated execution for forwarding hops.
//!
//! Control flow: the current owner sends a forwarding request to the
//! next hop, the next hop decides accept/reject through the generated protocol
//! branch, and the observer records the same visible outcome. The only
//! handwritten logic here is host adaptation around the generated session code.

use std::{cell::RefCell, error::Error, marker, rc::Rc, result};

use jacquard_core::{LinkEndpoint, RouteError, RouteId};
use serde_json::json;
use telltale::{
    futures::{executor, try_join},
    tell, try_session,
};

#[cfg(test)]
use super::effects::{PathwayCheckpointEnvelope, PathwayHeldPayload};
use super::{
    artifacts::{protocol_spec, PathwayProtocolKind},
    effects::{
        ChoreographyResultExt, PathwayChoreoFrame, PathwayProtocolObservation,
        PathwayProtocolRuntime,
    },
    runtime::{route_session, PathwayGuestRuntime},
};

pub(crate) const SOURCE_PATH: &str = "crates/pathway/src/choreography/forwarding.rs";
pub(crate) const PROTOCOL_NAME: &str = "ForwardingHop";
pub(crate) const ROLE_NAMES: &[&str] = &["CurrentOwner", "NextHop", "Observer"];

type ProtocolResult<T> = result::Result<T, Box<dyn Error + marker::Send + Sync>>;

tell! {
    profile Replay fairness eventual admissibility replay escalation_window bounded

    type PathwayProtocolError =
      | Unavailable
      | Rejected

    type alias HopFrame =
    {
      routeId : String
      payloadDigest : String
    }

    type alias ForwardReceipt =
    {
      routeId : String
      acceptedBy : Role
    }

    effect PathwayRuntime
      command forwardFrame : HopFrame -> Result PathwayProtocolError ForwardReceipt
      {
        class : best_effort
        progress : immediate
        region : fragment
        agreement_use : none
        reentrancy : allow
      }
      observe pollIngress : Session -> PresenceView
      {
        class : observational
        progress : immediate
        region : fragment
        agreement_use : forbidden
        reentrancy : allow
      }

    effect PathwayAudit
      observe record : AuditEvent -> Unit
      {
        class : observational
        progress : immediate
        region : global
        agreement_use : forbidden
        reentrancy : allow
      }

    protocol ForwardingHop uses PathwayRuntime, PathwayAudit under Replay =
      roles CurrentOwner, NextHop, Observer
      CurrentOwner -> NextHop : Forward { routeId : String, payloadDigest : String }
      choice NextHop at
        | Accepted =>
          NextHop -> CurrentOwner : Accepted { routeId : String }
          NextHop -> Observer : Accepted { routeId : String }
        | Rejected =>
          NextHop -> CurrentOwner : Rejected { routeId : String }
          NextHop -> Observer : Rejected { routeId : String }
}

use ForwardingHop::{
    effects,
    sessions::{
        Accepted, CurrentOwner, CurrentOwnerChoice1, CurrentOwnerSession, Forward,
        NextHop, NextHopSession, Observer, ObserverChoice1, ObserverSession, Rejected,
        Roles,
    },
};

struct SharedRuntime<'a, E> {
    effects: &'a mut E,
    route_id: RouteId,
    endpoint: LinkEndpoint,
}

struct NextHopHost<'a, E> {
    shared: Rc<RefCell<SharedRuntime<'a, E>>>,
}

struct ObserverHost<'a, E> {
    shared: Rc<RefCell<SharedRuntime<'a, E>>>,
}

impl<E> effects::PathwayRuntime for NextHopHost<'_, E>
where
    E: PathwayProtocolRuntime,
{
    fn forward_frame(
        &mut self,
        input: effects::HopFrame,
    ) -> Result<effects::ForwardReceipt, effects::PathwayProtocolError> {
        let mut shared = self.shared.borrow_mut();
        let endpoint = shared.endpoint.clone();
        shared
            .effects
            .send_frame(&PathwayChoreoFrame {
                endpoint,
                payload: input.payload_digest.as_bytes().to_vec(),
            })
            .map_err(|_| effects::PathwayProtocolError::Unavailable)?;
        Ok(effects::ForwardReceipt {
            route_id: input.route_id,
            accepted_by: effects::Role::new("NextHop"),
        })
    }

    fn poll_ingress(&mut self, _input: effects::Session) -> effects::PresenceView {
        json!({ "present": true })
    }
}

impl<E> effects::PathwayAudit for ObserverHost<'_, E>
where
    E: PathwayProtocolRuntime,
{
    fn record(&mut self, input: effects::AuditEvent) {
        let event = input.get("event").and_then(serde_json::Value::as_str);
        let detail = match event {
            | Some("generated-forwarded") => "generated-forwarded",
            | Some("generated-dropped") => "generated-dropped",
            | _ => "generated-observed",
        };
        let Ok(spec) = protocol_spec(PathwayProtocolKind::ForwardingHop) else {
            return;
        };
        let mut shared = self.shared.borrow_mut();
        let route_id = shared.route_id;
        shared
            .effects
            .emit_protocol_observation(PathwayProtocolObservation {
                protocol: PathwayProtocolKind::ForwardingHop,
                protocol_name: spec.protocol_name.clone(),
                role_names: spec.role_names.clone(),
                session: route_session(PathwayProtocolKind::ForwardingHop, &route_id),
                detail,
            });
    }
}

pub(crate) fn execute<E>(
    runtime: &mut PathwayGuestRuntime<E>,
    route_id: &RouteId,
    endpoint: LinkEndpoint,
    payload: &[u8],
) -> Result<(), RouteError>
where
    E: PathwayProtocolRuntime,
{
    let shared = Rc::new(RefCell::new(SharedRuntime {
        effects: runtime.protocol_runtime_mut(),
        route_id: *route_id,
        endpoint,
    }));
    let route_id_hex = hex_bytes(&route_id.0);
    let payload_digest = hex_bytes(payload);
    let Roles(mut current_owner, mut next_hop, mut observer) = Roles::default();
    let mut next_hop_host = NextHopHost { shared: Rc::clone(&shared) };
    let mut observer_host = ObserverHost { shared };

    executor::block_on(async {
        try_join!(
            current_owner_role(
                &mut current_owner,
                route_id_hex.clone(),
                payload_digest.clone()
            ),
            next_hop_role(&mut next_hop, &mut next_hop_host),
            observer_role(&mut observer, &mut observer_host),
        )
    })
    .map(|_| ())
    .choreography_failed()
}

async fn current_owner_role(
    role: &mut CurrentOwner,
    route_id: String,
    payload_digest: String,
) -> ProtocolResult<()> {
    try_session(role, |s: CurrentOwnerSession<'_, _>| async move {
        let s = s.send(Forward { route_id, payload_digest }).await?;
        match s.branch().await? {
            | CurrentOwnerChoice1::Accepted(_accepted, s) => Ok(((), s)),
            | CurrentOwnerChoice1::Rejected(_rejected, s) => Ok(((), s)),
        }
    })
    .await
}

async fn next_hop_role<E>(
    role: &mut NextHop,
    host: &mut NextHopHost<'_, E>,
) -> ProtocolResult<()>
where
    E: PathwayProtocolRuntime,
{
    try_session(role, |s: NextHopSession<'_, _>| async {
        let (Forward { route_id, payload_digest }, s) = s.receive().await?;
        let outcome = effects::PathwayRuntime::forward_frame(
            host,
            effects::HopFrame {
                route_id: route_id.clone(),
                payload_digest,
            },
        );
        match outcome {
            | Ok(_) => {
                let s = s.select(Accepted { route_id: route_id.clone() }).await?;
                let end = s.send(Accepted { route_id }).await?;
                Ok(((), end))
            },
            | Err(_) => {
                let s = s.select(Rejected { route_id: route_id.clone() }).await?;
                let end = s.send(Rejected { route_id }).await?;
                Ok(((), end))
            },
        }
    })
    .await
}

async fn observer_role<E>(
    role: &mut Observer,
    host: &mut ObserverHost<'_, E>,
) -> ProtocolResult<()>
where
    E: PathwayProtocolRuntime,
{
    try_session(role, |s: ObserverSession<'_, _>| async {
        match s.branch().await? {
            | ObserverChoice1::Accepted(Accepted { route_id }, end) => {
                effects::PathwayAudit::record(
                    host,
                    json!({ "event": "generated-forwarded", "route_id": route_id }),
                );
                Ok(((), end))
            },
            | ObserverChoice1::Rejected(Rejected { route_id }, end) => {
                effects::PathwayAudit::record(
                    host,
                    json!({ "event": "generated-dropped", "route_id": route_id }),
                );
                Ok(((), end))
            },
        }
    })
    .await
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        ByteCount, EndpointLocator, LinkEndpoint, Tick, TransportKind,
    };

    use super::*;

    #[derive(Default)]
    struct FakeEffects {
        sent_frames: Vec<Vec<u8>>,
        observations: Vec<PathwayProtocolObservation>,
    }

    impl PathwayProtocolRuntime for FakeEffects {
        fn now_tick(&self) -> Tick {
            Tick(1)
        }

        fn send_frame(
            &mut self,
            frame: &PathwayChoreoFrame,
        ) -> Result<(), jacquard_core::TransportError> {
            self.sent_frames.push(frame.payload.clone());
            Ok(())
        }

        fn store_held_payload(
            &mut self,
            _payload: &PathwayHeldPayload,
        ) -> Result<(), jacquard_core::RetentionError> {
            Ok(())
        }

        fn replay_held_payload(
            &mut self,
            _payload: &PathwayHeldPayload,
        ) -> Result<(), jacquard_core::RetentionError> {
            Ok(())
        }

        fn take_held_payload(
            &mut self,
            _object_id: &jacquard_core::ContentId<jacquard_core::Blake3Digest>,
        ) -> Result<Option<Vec<u8>>, jacquard_core::RetentionError> {
            Ok(None)
        }

        fn load_protocol_checkpoint(
            &self,
            _key: &[u8],
        ) -> Result<Option<Vec<u8>>, jacquard_core::StorageError> {
            Ok(None)
        }

        fn store_protocol_checkpoint(
            &mut self,
            _checkpoint: &PathwayCheckpointEnvelope,
        ) -> Result<(), jacquard_core::StorageError> {
            Ok(())
        }

        fn remove_protocol_checkpoint(
            &mut self,
            _key: &[u8],
        ) -> Result<(), jacquard_core::StorageError> {
            Ok(())
        }

        fn emit_protocol_observation(
            &mut self,
            observation: PathwayProtocolObservation,
        ) {
            self.observations.push(observation);
        }
    }

    #[test]
    fn generated_forwarding_execution_emits_observer_side_observation() {
        let mut runtime = PathwayGuestRuntime::new(FakeEffects::default());
        execute(
            &mut runtime,
            &RouteId([7; 16]),
            LinkEndpoint::new(
                TransportKind::WifiAware,
                EndpointLocator::Opaque(vec![1]),
                ByteCount(128),
            ),
            b"frame",
        )
        .expect("generated forwarding executes");

        assert_eq!(runtime.protocol_runtime_ref().sent_frames.len(), 1);
        assert!(runtime
            .protocol_runtime_ref()
            .observations
            .iter()
            .any(|observation| observation.detail == "generated-forwarded"));
    }
}
