//! Mesh guest-runtime entry points over the choreography effect bridge.
//!
//! Control flow: mesh runtime code enters this module at protocol boundaries
//! such as activation, repair, handoff, forwarding, hold/replay, and tick
//! ingress. The guest runtime stores small protocol checkpoints, emits
//! mesh-local protocol observations, and funnels transport/retention/runtime
//! side effects through `MeshProtocolRuntime` instead of scattering those calls
//! across imperative engine helpers.

use bincode::Options;
use jacquard_core::{
    Blake3Digest, ContentId, HealthScore, LinkEndpoint, NodeId, RouteEpoch, RouteError,
    RouteId, RouteRuntimeError, Tick, TransportObservation,
};
use serde::{Deserialize, Serialize};

use super::{
    activation, anti_entropy,
    artifacts::{
        protocol_spec, MeshProtocolKind, MeshProtocolSessionKey, MeshProtocolSpec,
    },
    effects::{MeshCheckpointEnvelope, MeshProtocolObservation, MeshProtocolRuntime},
    forwarding, handoff, hold_replay, neighbor_advertisement, repair, route_export,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MeshProtocolCheckpoint {
    protocol:        MeshProtocolKind,
    protocol_name:   String,
    role_names:      Vec<String>,
    source_path:     String,
    session:         MeshProtocolSessionKey,
    detail:          String,
    last_updated_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshRouteExportSnapshot {
    pub(crate) route_class:    String,
    pub(crate) hop_count:      u32,
    pub(crate) partition_mode: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshNeighborAdvertisementSnapshot {
    pub(crate) local_node_id:           NodeId,
    pub(crate) service_count:           u32,
    pub(crate) adjacent_neighbor_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshAntiEntropySnapshot {
    pub(crate) retained_count: u32,
    pub(crate) pressure_score: HealthScore,
    pub(crate) partition_mode: bool,
}

type RuntimeSpecResolver =
    fn(MeshProtocolKind) -> Result<&'static MeshProtocolSpec, String>;

pub(crate) struct MeshGuestRuntime<E> {
    effects:      E,
    resolve_spec: RuntimeSpecResolver,
}

impl<E> MeshGuestRuntime<E>
where
    E: MeshProtocolRuntime,
{
    pub(crate) fn new(effects: E) -> Self {
        Self::with_spec_resolver(effects, protocol_spec)
    }

    pub(crate) fn with_spec_resolver(
        effects: E,
        resolve_spec: RuntimeSpecResolver,
    ) -> Self {
        Self { effects, resolve_spec }
    }

    pub(super) fn protocol_runtime_mut(&mut self) -> &mut E {
        &mut self.effects
    }

    #[cfg(test)]
    pub(super) fn protocol_runtime_ref(&self) -> &E {
        &self.effects
    }

    pub(crate) fn activation_handshake(
        &mut self,
        route_id: &RouteId,
        epoch: RouteEpoch,
    ) -> Result<(), RouteError> {
        self.protocol_step(
            MeshProtocolKind::Activation,
            route_session(MeshProtocolKind::Activation, route_id),
            |runtime| activation::execute(runtime, route_id, epoch),
        )?;
        self.protocol_detail_checkpoint(
            MeshProtocolKind::Activation,
            route_session(MeshProtocolKind::Activation, route_id),
            "activated",
        )
    }

    pub(crate) fn repair_exchange(
        &mut self,
        route_id: &RouteId,
    ) -> Result<(), RouteError> {
        self.protocol_step(
            MeshProtocolKind::Repair,
            route_session(MeshProtocolKind::Repair, route_id),
            |runtime| repair::execute(runtime, route_id),
        )?;
        self.protocol_detail_checkpoint(
            MeshProtocolKind::Repair,
            route_session(MeshProtocolKind::Repair, route_id),
            "repaired",
        )
    }

    pub(crate) fn handoff_exchange(
        &mut self,
        route_id: &RouteId,
    ) -> Result<(), RouteError> {
        self.protocol_step(
            MeshProtocolKind::Handoff,
            route_session(MeshProtocolKind::Handoff, route_id),
            |runtime| handoff::execute(runtime, route_id),
        )?;
        self.protocol_detail_checkpoint(
            MeshProtocolKind::Handoff,
            route_session(MeshProtocolKind::Handoff, route_id),
            "handed-off",
        )
    }

    pub(crate) fn clear_route_protocols(
        &mut self,
        route_id: &RouteId,
    ) -> Result<(), RouteError> {
        for kind in [
            MeshProtocolKind::Activation,
            MeshProtocolKind::Repair,
            MeshProtocolKind::Handoff,
            MeshProtocolKind::ForwardingHop,
            MeshProtocolKind::HoldReplay,
            MeshProtocolKind::RouteExport,
            MeshProtocolKind::AntiEntropy,
        ] {
            self.effects
                .remove_protocol_checkpoint(&protocol_checkpoint_key(
                    kind,
                    &route_session(kind, route_id),
                ))
                .map_err(storage_failure)?;
        }
        Ok(())
    }

    pub(crate) fn forwarding_hop(
        &mut self,
        route_id: &RouteId,
        endpoint: LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        self.protocol_step(
            MeshProtocolKind::ForwardingHop,
            route_session(MeshProtocolKind::ForwardingHop, route_id),
            |runtime| forwarding::execute(runtime, route_id, endpoint, payload),
        )?;
        self.protocol_detail_checkpoint(
            MeshProtocolKind::ForwardingHop,
            route_session(MeshProtocolKind::ForwardingHop, route_id),
            "sent",
        )
    }

    pub(crate) fn route_export_exchange(
        &mut self,
        route_id: &RouteId,
        snapshot: &MeshRouteExportSnapshot,
    ) -> Result<(), RouteError> {
        let detail = self.protocol_step(
            MeshProtocolKind::RouteExport,
            route_session(MeshProtocolKind::RouteExport, route_id),
            |runtime| route_export::execute(runtime, route_id, snapshot),
        )?;
        self.protocol_detail_checkpoint(
            MeshProtocolKind::RouteExport,
            route_session(MeshProtocolKind::RouteExport, route_id),
            detail,
        )
    }

    pub(crate) fn neighbor_advertisement_exchange(
        &mut self,
        epoch: RouteEpoch,
        snapshot: &MeshNeighborAdvertisementSnapshot,
    ) -> Result<(), RouteError> {
        let detail = self.protocol_step(
            MeshProtocolKind::NeighborAdvertisement,
            tick_session(epoch),
            |runtime| neighbor_advertisement::execute(runtime, epoch, snapshot),
        )?;
        self.protocol_detail_checkpoint(
            MeshProtocolKind::NeighborAdvertisement,
            tick_session(epoch),
            detail,
        )
    }

    pub(crate) fn anti_entropy_exchange(
        &mut self,
        route_id: &RouteId,
        snapshot: &MeshAntiEntropySnapshot,
    ) -> Result<(), RouteError> {
        let detail = self.protocol_step(
            MeshProtocolKind::AntiEntropy,
            route_session(MeshProtocolKind::AntiEntropy, route_id),
            |runtime| anti_entropy::execute(runtime, route_id, snapshot),
        )?;
        self.protocol_detail_checkpoint(
            MeshProtocolKind::AntiEntropy,
            route_session(MeshProtocolKind::AntiEntropy, route_id),
            detail,
        )
    }

    pub(crate) fn retain_for_replay(
        &mut self,
        route_id: &RouteId,
        object_id: ContentId<Blake3Digest>,
        payload: &[u8],
    ) -> Result<(), jacquard_core::RetentionError> {
        self.protocol_step(
            MeshProtocolKind::HoldReplay,
            route_session(MeshProtocolKind::HoldReplay, route_id),
            |runtime| hold_replay::retain(runtime, route_id, object_id, payload),
        )
        .map_err(|_| jacquard_core::RetentionError::Unavailable)?;
        self.protocol_detail_checkpoint(
            MeshProtocolKind::HoldReplay,
            route_session(MeshProtocolKind::HoldReplay, route_id),
            "retained",
        )
        .map_err(|_| jacquard_core::RetentionError::Unavailable)
    }

    pub(crate) fn replay_to_next_hop(
        &mut self,
        route_id: &RouteId,
        object_id: ContentId<Blake3Digest>,
        endpoint: LinkEndpoint,
        payload: Vec<u8>,
    ) -> Result<(), RouteError> {
        self.protocol_step(
            MeshProtocolKind::HoldReplay,
            route_session(MeshProtocolKind::HoldReplay, route_id),
            |runtime| {
                hold_replay::replay(runtime, route_id, object_id, endpoint, payload)
            },
        )?;
        self.protocol_detail_checkpoint(
            MeshProtocolKind::HoldReplay,
            route_session(MeshProtocolKind::HoldReplay, route_id),
            "replayed",
        )
    }

    pub(crate) fn recover_held_payload(
        &mut self,
        route_id: &RouteId,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, jacquard_core::RetentionError> {
        let payload = self.effects.take_held_payload(object_id)?;
        if payload.is_some() {
            let spec = (self.resolve_spec)(MeshProtocolKind::HoldReplay)
                .map_err(|_| jacquard_core::RetentionError::Unavailable)?;
            self.protocol_checkpoint(
                spec,
                route_session(MeshProtocolKind::HoldReplay, route_id),
                Some("released"),
            )
            .map_err(|_| jacquard_core::RetentionError::Unavailable)?;
        }
        Ok(payload)
    }

    pub(crate) fn poll_tick_ingress(
        &mut self,
        epoch: RouteEpoch,
    ) -> Result<Vec<TransportObservation>, RouteError> {
        let mut observations = Vec::new();
        let detail = self.protocol_step(
            MeshProtocolKind::ForwardingHop,
            tick_session(epoch),
            |runtime| {
                observations = runtime
                    .effects
                    .poll_mesh_ingress()
                    .map_err(RouteError::from)?;
                Ok("tick")
            },
        )?;
        self.protocol_detail_checkpoint(
            MeshProtocolKind::ForwardingHop,
            tick_session(epoch),
            detail,
        )?;
        Ok(observations)
    }

    fn protocol_step<T, F>(
        &mut self,
        protocol: MeshProtocolKind,
        session: MeshProtocolSessionKey,
        action: F,
    ) -> Result<T, RouteError>
    where
        F: FnOnce(&mut Self) -> Result<T, RouteError>,
    {
        let spec = (self.resolve_spec)(protocol).map_err(protocol_failure)?;
        let result = action(self)?;
        self.protocol_checkpoint(spec, session, None)?;
        Ok(result)
    }

    fn protocol_detail_checkpoint(
        &mut self,
        protocol: MeshProtocolKind,
        session: MeshProtocolSessionKey,
        detail: &'static str,
    ) -> Result<(), RouteError> {
        let spec = (self.resolve_spec)(protocol).map_err(protocol_failure)?;
        self.protocol_checkpoint(spec, session, Some(detail))
    }

    fn protocol_checkpoint(
        &mut self,
        spec: &MeshProtocolSpec,
        session: MeshProtocolSessionKey,
        detail: Option<&'static str>,
    ) -> Result<(), RouteError> {
        let checkpoint = MeshProtocolCheckpoint {
            protocol:        spec.kind,
            protocol_name:   spec.protocol_name.clone(),
            role_names:      spec.role_names.clone(),
            source_path:     spec.source_path.to_owned(),
            session:         session.clone(),
            detail:          detail.unwrap_or("entered").to_owned(),
            last_updated_at: self.effects.now_tick(),
        };
        let key = protocol_checkpoint_key(spec.kind, &session);
        let bytes = checkpoint_bytes(&checkpoint);
        if let Some(existing) = self
            .effects
            .load_protocol_checkpoint(&key)
            .map_err(storage_failure)?
            .and_then(|bytes| decode_checkpoint_bytes(&bytes))
        {
            if checkpoint_matches_without_timestamp(&existing, &checkpoint) {
                return Ok(());
            }
        }
        self.effects
            .store_protocol_checkpoint(&MeshCheckpointEnvelope { key, bytes })
            .map_err(storage_failure)?;
        self.effects
            .emit_protocol_observation(MeshProtocolObservation {
                protocol: spec.kind,
                protocol_name: spec.protocol_name.clone(),
                role_names: spec.role_names.clone(),
                session,
                detail: detail.unwrap_or("entered"),
            });
        Ok(())
    }
}

pub(crate) fn route_session(
    protocol: MeshProtocolKind,
    route_id: &RouteId,
) -> MeshProtocolSessionKey {
    MeshProtocolSessionKey(format!("{}-{}", protocol.as_str(), hex_bytes(&route_id.0)))
}

pub(crate) fn tick_session(epoch: RouteEpoch) -> MeshProtocolSessionKey {
    MeshProtocolSessionKey(format!("tick-epoch-{}", epoch.0))
}

fn protocol_checkpoint_key(
    protocol: MeshProtocolKind,
    session: &MeshProtocolSessionKey,
) -> Vec<u8> {
    format!("mesh/protocol/{}/{}", protocol.as_str(), session.0).into_bytes()
}

fn checkpoint_bytes(checkpoint: &MeshProtocolCheckpoint) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(32);
    bytes.push(1);
    bytes.extend(
        bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .serialize(checkpoint)
            .expect("mesh protocol checkpoints are always serializable"),
    );
    bytes
}

fn checkpoint_matches_without_timestamp(
    existing: &MeshProtocolCheckpoint,
    candidate: &MeshProtocolCheckpoint,
) -> bool {
    existing.protocol == candidate.protocol
        && existing.protocol_name == candidate.protocol_name
        && existing.role_names == candidate.role_names
        && existing.source_path == candidate.source_path
        && existing.session == candidate.session
        && existing.detail == candidate.detail
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn storage_failure(_: jacquard_core::StorageError) -> RouteError {
    RouteError::Runtime(RouteRuntimeError::Invalidated)
}

fn protocol_failure(_: String) -> RouteError {
    RouteError::Runtime(RouteRuntimeError::Invalidated)
}

fn decode_checkpoint_bytes(bytes: &[u8]) -> Option<MeshProtocolCheckpoint> {
    let (version, body) = bytes.split_first()?;
    if *version != 1 {
        return None;
    }
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .deserialize(body)
        .ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bincode::Options;
    use jacquard_core::{
        Blake3Digest, ContentId, RouteEpoch, RouteError, RouteId, RouteRuntimeError,
        StorageError, Tick, TransportObservation,
    };
    use jacquard_traits::{effect_handler, StorageEffects, TimeEffects};

    use super::{
        protocol_checkpoint_key, route_session, tick_session, MeshGuestRuntime,
        MeshProtocolCheckpoint,
    };
    use crate::choreography::{
        artifacts::{MeshProtocolKind, MeshProtocolSessionKey, MeshProtocolSpec},
        effects::{
            MeshCheckpointEnvelope, MeshProtocolObservation, MeshProtocolRuntime,
        },
    };

    #[derive(Default)]
    struct FakeEffects {
        checkpoints:  BTreeMap<Vec<u8>, Vec<u8>>,
        observations: Vec<MeshProtocolObservation>,
        ingress:      Vec<TransportObservation>,
    }

    #[effect_handler]
    impl TimeEffects for FakeEffects {
        fn now_tick(&self) -> Tick {
            Tick(4)
        }
    }

    #[effect_handler]
    impl StorageEffects for FakeEffects {
        fn load_bytes(&self, _key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
            Ok(None)
        }

        fn store_bytes(
            &mut self,
            key: &[u8],
            value: &[u8],
        ) -> Result<(), StorageError> {
            self.checkpoints.insert(key.to_vec(), value.to_vec());
            Ok(())
        }

        fn remove_bytes(&mut self, _key: &[u8]) -> Result<(), StorageError> {
            Ok(())
        }
    }

    impl MeshProtocolRuntime for FakeEffects {
        fn now_tick(&self) -> Tick {
            Tick(4)
        }

        fn send_mesh_frame(
            &mut self,
            _frame: &crate::choreography::effects::MeshChoreoFrame,
        ) -> Result<(), jacquard_core::TransportError> {
            Ok(())
        }

        fn poll_mesh_ingress(
            &mut self,
        ) -> Result<Vec<TransportObservation>, jacquard_core::TransportError> {
            Ok(std::mem::take(&mut self.ingress))
        }

        fn store_held_payload(
            &mut self,
            _payload: &crate::choreography::effects::MeshHeldPayload,
        ) -> Result<(), jacquard_core::RetentionError> {
            Ok(())
        }

        fn replay_held_payload(
            &mut self,
            _payload: &crate::choreography::effects::MeshHeldPayload,
        ) -> Result<(), jacquard_core::RetentionError> {
            Ok(())
        }

        fn take_held_payload(
            &mut self,
            _object_id: &ContentId<Blake3Digest>,
        ) -> Result<Option<Vec<u8>>, jacquard_core::RetentionError> {
            Ok(None)
        }

        fn load_protocol_checkpoint(
            &self,
            key: &[u8],
        ) -> Result<Option<Vec<u8>>, StorageError> {
            Ok(self.checkpoints.get(key).cloned())
        }

        fn store_protocol_checkpoint(
            &mut self,
            checkpoint: &MeshCheckpointEnvelope,
        ) -> Result<(), StorageError> {
            self.checkpoints
                .insert(checkpoint.key.clone(), checkpoint.bytes.clone());
            Ok(())
        }

        fn remove_protocol_checkpoint(
            &mut self,
            key: &[u8],
        ) -> Result<(), StorageError> {
            self.checkpoints.remove(key);
            Ok(())
        }

        fn emit_protocol_observation(&mut self, observation: MeshProtocolObservation) {
            self.observations.push(observation);
        }
    }

    #[test]
    fn route_and_tick_sessions_are_stable() {
        assert_eq!(
            route_session(MeshProtocolKind::Activation, &RouteId([0xab; 16]),),
            MeshProtocolSessionKey(
                "activation-abababababababababababababababab".into()
            )
        );
        assert_eq!(
            tick_session(RouteEpoch(7)),
            MeshProtocolSessionKey("tick-epoch-7".into())
        );
    }

    #[test]
    fn guest_runtime_records_protocol_checkpoints() {
        let object_id = ContentId { digest: Blake3Digest([9; 32]) };
        let mut runtime = MeshGuestRuntime::new(FakeEffects::default());
        runtime
            .retain_for_replay(&RouteId([3; 16]), object_id, b"payload")
            .expect("retain");
        runtime
            .activation_handshake(&RouteId([3; 16]), RouteEpoch(3))
            .expect("activation");
        let ingress = runtime
            .poll_tick_ingress(RouteEpoch(2))
            .expect("tick ingress");
        let hold_checkpoint = load_checkpoint(
            &runtime,
            MeshProtocolKind::HoldReplay,
            &route_session(MeshProtocolKind::HoldReplay, &RouteId([3; 16])),
        )
        .expect("hold checkpoint present");
        let tick_checkpoint = load_checkpoint(
            &runtime,
            MeshProtocolKind::ForwardingHop,
            &tick_session(RouteEpoch(2)),
        )
        .expect("tick checkpoint present");

        assert!(ingress.is_empty());
        assert!(runtime.effects.checkpoints.len() >= 3);
        assert!(runtime.effects.observations.len() >= 2);
        assert_eq!(hold_checkpoint.detail, "retained");
        assert_eq!(hold_checkpoint.protocol_name, "HoldReplayExchange");
        assert!(hold_checkpoint
            .role_names
            .iter()
            .any(|role| role == "PartitionedOwner"));
        assert_eq!(tick_checkpoint.detail, "tick");
        assert!(runtime
            .effects
            .observations
            .iter()
            .any(|observation| observation.protocol_name == "ActivationHandshake"));
    }

    #[test]
    fn guest_runtime_fails_closed_when_protocol_spec_resolution_fails() {
        fn failing_spec(
            _: MeshProtocolKind,
        ) -> Result<&'static MeshProtocolSpec, String> {
            Err("broken artifact".into())
        }

        let mut runtime =
            MeshGuestRuntime::with_spec_resolver(FakeEffects::default(), failing_spec);
        let error = runtime
            .activation_handshake(&RouteId([7; 16]), RouteEpoch(2))
            .expect_err("invalid protocol artifact should fail closed");

        assert_eq!(error, RouteError::Runtime(RouteRuntimeError::Invalidated));
        assert!(runtime.effects.checkpoints.is_empty());
        assert!(runtime.effects.observations.is_empty());
    }

    fn load_checkpoint(
        runtime: &MeshGuestRuntime<FakeEffects>,
        protocol: MeshProtocolKind,
        session: &MeshProtocolSessionKey,
    ) -> Option<MeshProtocolCheckpoint> {
        let bytes = runtime
            .effects
            .checkpoints
            .get(&protocol_checkpoint_key(protocol, session))?
            .clone();
        decode_checkpoint_bytes(&bytes)
    }

    fn decode_checkpoint_bytes(bytes: &[u8]) -> Option<MeshProtocolCheckpoint> {
        let (version, rest) = bytes.split_first()?;
        if *version != 1 {
            return None;
        }
        bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .deserialize(rest)
            .ok()
    }
}
