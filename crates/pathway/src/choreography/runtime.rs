//! Pathway guest-runtime entry points over the choreography effect bridge.
//!
//! Control flow: pathway runtime code enters this module at protocol boundaries
//! such as activation, repair, handoff, forwarding, hold/replay, and tick
//! ingress. The guest runtime stores small protocol checkpoints, emits
//! pathway-local protocol observations, and funnels transport/retention/runtime
//! side effects through `PathwayProtocolRuntime` instead of scattering those
//! calls across imperative engine helpers.

use bincode::Options;
use jacquard_core::{
    Blake3Digest, ContentId, HealthScore, LinkEndpoint, NodeId, RouteEpoch, RouteError, RouteId,
    RouteRuntimeError, Tick, TransportObservation,
};
use jacquard_traits::{RetentionStore, StorageEffects, TimeEffects, TransportSenderEffects};
use serde::{Deserialize, Serialize};

use super::{
    activation, anti_entropy,
    artifacts::{
        protocol_spec, PathwayProtocolKind, PathwayProtocolSessionKey, PathwayProtocolSpec,
    },
    effects::{PathwayCheckpointEnvelope, PathwayProtocolObservation, PathwayProtocolRuntime},
    forwarding, handoff, hold_replay, neighbor_advertisement, repair, route_export,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PathwayProtocolCheckpoint {
    protocol: PathwayProtocolKind,
    protocol_name: String,
    role_names: Vec<String>,
    source_path: String,
    session: PathwayProtocolSessionKey,
    detail: String,
    last_updated_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathwayRouteExportSnapshot {
    pub(crate) route_class: String,
    pub(crate) hop_count: u32,
    pub(crate) partition_mode: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathwayNeighborAdvertisementSnapshot {
    pub(crate) local_node_id: NodeId,
    pub(crate) service_count: u32,
    pub(crate) adjacent_neighbor_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathwayAntiEntropySnapshot {
    pub(crate) retained_count: u32,
    pub(crate) pressure_score: HealthScore,
    pub(crate) partition_mode: bool,
}

type RuntimeSpecResolver = fn(PathwayProtocolKind) -> Result<&'static PathwayProtocolSpec, String>;

pub(crate) struct PathwayGuestRuntime<E> {
    effects: E,
    resolve_spec: RuntimeSpecResolver,
}

impl<E> PathwayGuestRuntime<E>
where
    E: PathwayProtocolRuntime,
{
    pub(crate) fn new(effects: E) -> Self {
        Self::with_spec_resolver(effects, protocol_spec)
    }

    pub(crate) fn with_spec_resolver(effects: E, resolve_spec: RuntimeSpecResolver) -> Self {
        Self {
            effects,
            resolve_spec,
        }
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
            PathwayProtocolKind::Activation,
            route_session(PathwayProtocolKind::Activation, route_id),
            |runtime| activation::execute(runtime, route_id, epoch),
        )?;
        self.protocol_detail_checkpoint(
            PathwayProtocolKind::Activation,
            route_session(PathwayProtocolKind::Activation, route_id),
            "activated",
        )
    }

    pub(crate) fn repair_exchange(&mut self, route_id: &RouteId) -> Result<(), RouteError> {
        self.protocol_step(
            PathwayProtocolKind::Repair,
            route_session(PathwayProtocolKind::Repair, route_id),
            |runtime| repair::execute(runtime, route_id),
        )?;
        self.protocol_detail_checkpoint(
            PathwayProtocolKind::Repair,
            route_session(PathwayProtocolKind::Repair, route_id),
            "repaired",
        )
    }

    pub(crate) fn handoff_exchange(&mut self, route_id: &RouteId) -> Result<(), RouteError> {
        self.protocol_step(
            PathwayProtocolKind::Handoff,
            route_session(PathwayProtocolKind::Handoff, route_id),
            |runtime| handoff::execute(runtime, route_id),
        )?;
        self.protocol_detail_checkpoint(
            PathwayProtocolKind::Handoff,
            route_session(PathwayProtocolKind::Handoff, route_id),
            "handed-off",
        )
    }

    pub(crate) fn clear_route_protocols(&mut self, route_id: &RouteId) -> Result<(), RouteError> {
        for kind in [
            PathwayProtocolKind::Activation,
            PathwayProtocolKind::Repair,
            PathwayProtocolKind::Handoff,
            PathwayProtocolKind::ForwardingHop,
            PathwayProtocolKind::HoldReplay,
            PathwayProtocolKind::RouteExport,
            PathwayProtocolKind::AntiEntropy,
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
            PathwayProtocolKind::ForwardingHop,
            route_session(PathwayProtocolKind::ForwardingHop, route_id),
            |runtime| forwarding::execute(runtime, route_id, endpoint, payload),
        )?;
        self.protocol_detail_checkpoint(
            PathwayProtocolKind::ForwardingHop,
            route_session(PathwayProtocolKind::ForwardingHop, route_id),
            "sent",
        )
    }

    pub(crate) fn route_export_exchange(
        &mut self,
        route_id: &RouteId,
        snapshot: &PathwayRouteExportSnapshot,
    ) -> Result<(), RouteError> {
        let detail = self.protocol_step(
            PathwayProtocolKind::RouteExport,
            route_session(PathwayProtocolKind::RouteExport, route_id),
            |runtime| route_export::execute(runtime, route_id, snapshot),
        )?;
        self.protocol_detail_checkpoint(
            PathwayProtocolKind::RouteExport,
            route_session(PathwayProtocolKind::RouteExport, route_id),
            detail,
        )
    }

    pub(crate) fn neighbor_advertisement_exchange(
        &mut self,
        epoch: RouteEpoch,
        snapshot: &PathwayNeighborAdvertisementSnapshot,
    ) -> Result<(), RouteError> {
        let detail = self.protocol_step(
            PathwayProtocolKind::NeighborAdvertisement,
            tick_session(epoch),
            |runtime| neighbor_advertisement::execute(runtime, epoch, snapshot),
        )?;
        self.protocol_detail_checkpoint(
            PathwayProtocolKind::NeighborAdvertisement,
            tick_session(epoch),
            detail,
        )
    }

    pub(crate) fn anti_entropy_exchange(
        &mut self,
        route_id: &RouteId,
        snapshot: &PathwayAntiEntropySnapshot,
    ) -> Result<(), RouteError> {
        let detail = self.protocol_step(
            PathwayProtocolKind::AntiEntropy,
            route_session(PathwayProtocolKind::AntiEntropy, route_id),
            |runtime| anti_entropy::execute(runtime, route_id, snapshot),
        )?;
        self.protocol_detail_checkpoint(
            PathwayProtocolKind::AntiEntropy,
            route_session(PathwayProtocolKind::AntiEntropy, route_id),
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
            PathwayProtocolKind::HoldReplay,
            route_session(PathwayProtocolKind::HoldReplay, route_id),
            |runtime| hold_replay::retain(runtime, route_id, object_id, payload),
        )
        .map_err(|_| jacquard_core::RetentionError::Unavailable)?;
        self.protocol_detail_checkpoint(
            PathwayProtocolKind::HoldReplay,
            route_session(PathwayProtocolKind::HoldReplay, route_id),
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
            PathwayProtocolKind::HoldReplay,
            route_session(PathwayProtocolKind::HoldReplay, route_id),
            |runtime| hold_replay::replay(runtime, route_id, object_id, endpoint, payload),
        )?;
        self.protocol_detail_checkpoint(
            PathwayProtocolKind::HoldReplay,
            route_session(PathwayProtocolKind::HoldReplay, route_id),
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
            let spec = (self.resolve_spec)(PathwayProtocolKind::HoldReplay)
                .map_err(|_| jacquard_core::RetentionError::Unavailable)?;
            self.protocol_checkpoint(
                spec,
                route_session(PathwayProtocolKind::HoldReplay, route_id),
                Some("released"),
            )
            .map_err(|_| jacquard_core::RetentionError::Unavailable)?;
        }
        Ok(payload)
    }

    pub(crate) fn record_tick_ingress(
        &mut self,
        epoch: RouteEpoch,
        observations: &[TransportObservation],
    ) -> Result<(), RouteError> {
        let detail = self.protocol_step(
            PathwayProtocolKind::ForwardingHop,
            tick_session(epoch),
            |_runtime| Ok("tick"),
        )?;
        self.protocol_detail_checkpoint(
            PathwayProtocolKind::ForwardingHop,
            tick_session(epoch),
            detail,
        )?;
        if !observations.is_empty() {
            self.effects
                .emit_protocol_observation(PathwayProtocolObservation {
                    protocol: PathwayProtocolKind::ForwardingHop,
                    protocol_name: "ForwardingHop".to_owned(),
                    role_names: vec!["transport-ingress".to_owned()],
                    session: tick_session(epoch),
                    detail: "ingested",
                });
        }
        Ok(())
    }

    fn protocol_step<T, F>(
        &mut self,
        protocol: PathwayProtocolKind,
        session: PathwayProtocolSessionKey,
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
        protocol: PathwayProtocolKind,
        session: PathwayProtocolSessionKey,
        detail: &'static str,
    ) -> Result<(), RouteError> {
        let spec = (self.resolve_spec)(protocol).map_err(protocol_failure)?;
        self.protocol_checkpoint(spec, session, Some(detail))
    }

    fn protocol_checkpoint(
        &mut self,
        spec: &PathwayProtocolSpec,
        session: PathwayProtocolSessionKey,
        detail: Option<&'static str>,
    ) -> Result<(), RouteError> {
        let checkpoint = PathwayProtocolCheckpoint {
            protocol: spec.kind,
            protocol_name: spec.protocol_name.clone(),
            role_names: spec.role_names.clone(),
            source_path: spec.source_path.to_owned(),
            session: session.clone(),
            detail: detail.unwrap_or("entered").to_owned(),
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
            .store_protocol_checkpoint(&PathwayCheckpointEnvelope { key, bytes })
            .map_err(storage_failure)?;
        self.effects
            .emit_protocol_observation(PathwayProtocolObservation {
                protocol: spec.kind,
                protocol_name: spec.protocol_name.clone(),
                role_names: spec.role_names.clone(),
                session,
                detail: detail.unwrap_or("entered"),
            });
        Ok(())
    }
}

fn with_guest_runtime<T, R, E, F, Out, Err>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    step: F,
) -> Result<Out, Err>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
    F: FnOnce(
        &mut PathwayGuestRuntime<super::effects::PathwayProtocolRuntimeAdapter<'_, T, R, E>>,
    ) -> Result<Out, Err>,
{
    let mut runtime = PathwayGuestRuntime::new(super::effects::PathwayProtocolRuntimeAdapter {
        transport,
        retention,
        effects,
    });
    step(&mut runtime)
}

pub(crate) fn activation_handshake<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    route_id: &RouteId,
    epoch: RouteEpoch,
) -> Result<(), RouteError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.activation_handshake(route_id, epoch)
    })
}

pub(crate) fn repair_exchange<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    route_id: &RouteId,
) -> Result<(), RouteError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.repair_exchange(route_id)
    })
}

pub(crate) fn handoff_exchange<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    route_id: &RouteId,
) -> Result<(), RouteError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.handoff_exchange(route_id)
    })
}

pub(crate) fn clear_route_protocols<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    route_id: &RouteId,
) -> Result<(), RouteError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.clear_route_protocols(route_id)
    })
}

pub(crate) fn forwarding_hop<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    route_id: &RouteId,
    endpoint: LinkEndpoint,
    payload: &[u8],
) -> Result<(), RouteError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.forwarding_hop(route_id, endpoint, payload)
    })
}

pub(crate) fn route_export_exchange<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    route_id: &RouteId,
    snapshot: &PathwayRouteExportSnapshot,
) -> Result<(), RouteError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.route_export_exchange(route_id, snapshot)
    })
}

pub(crate) fn neighbor_advertisement_exchange<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    epoch: RouteEpoch,
    snapshot: &PathwayNeighborAdvertisementSnapshot,
) -> Result<(), RouteError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.neighbor_advertisement_exchange(epoch, snapshot)
    })
}

pub(crate) fn anti_entropy_exchange<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    route_id: &RouteId,
    snapshot: &PathwayAntiEntropySnapshot,
) -> Result<(), RouteError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.anti_entropy_exchange(route_id, snapshot)
    })
}

pub(crate) fn retain_for_replay<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    route_id: &RouteId,
    object_id: ContentId<Blake3Digest>,
    payload: &[u8],
) -> Result<(), jacquard_core::RetentionError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.retain_for_replay(route_id, object_id, payload)
    })
}

pub(crate) fn replay_to_next_hop<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    route_id: &RouteId,
    object_id: ContentId<Blake3Digest>,
    endpoint: LinkEndpoint,
    payload: Vec<u8>,
) -> Result<(), RouteError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.replay_to_next_hop(route_id, object_id, endpoint, payload)
    })
}

pub(crate) fn recover_held_payload<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    route_id: &RouteId,
    object_id: &ContentId<Blake3Digest>,
) -> Result<Option<Vec<u8>>, jacquard_core::RetentionError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.recover_held_payload(route_id, object_id)
    })
}

pub(crate) fn record_tick_ingress<T, R, E>(
    transport: &mut T,
    retention: &mut R,
    effects: &mut E,
    epoch: RouteEpoch,
    observations: &[TransportObservation],
) -> Result<(), RouteError>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    with_guest_runtime(transport, retention, effects, |runtime| {
        runtime.record_tick_ingress(epoch, observations)
    })
}

pub(crate) fn route_session(
    protocol: PathwayProtocolKind,
    route_id: &RouteId,
) -> PathwayProtocolSessionKey {
    PathwayProtocolSessionKey(format!("{}-{}", protocol.as_str(), hex_bytes(&route_id.0)))
}

pub(crate) fn tick_session(epoch: RouteEpoch) -> PathwayProtocolSessionKey {
    PathwayProtocolSessionKey(format!("tick-epoch-{}", epoch.0))
}

fn protocol_checkpoint_key(
    protocol: PathwayProtocolKind,
    session: &PathwayProtocolSessionKey,
) -> Vec<u8> {
    format!("pathway/protocol/{}/{}", protocol.as_str(), session.0).into_bytes()
}

fn checkpoint_bytes(checkpoint: &PathwayProtocolCheckpoint) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(32);
    bytes.push(1);
    bytes.extend(
        bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .serialize(checkpoint)
            .expect("pathway protocol checkpoints are always serializable"),
    );
    bytes
}

fn checkpoint_matches_without_timestamp(
    existing: &PathwayProtocolCheckpoint,
    candidate: &PathwayProtocolCheckpoint,
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

fn decode_checkpoint_bytes(bytes: &[u8]) -> Option<PathwayProtocolCheckpoint> {
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
        Blake3Digest, ContentId, RouteEpoch, RouteError, RouteId, RouteRuntimeError, StorageError,
        Tick,
    };
    use jacquard_traits::{effect_handler, StorageEffects, TimeEffects};

    use super::{
        protocol_checkpoint_key, route_session, tick_session, PathwayGuestRuntime,
        PathwayProtocolCheckpoint,
    };
    use crate::choreography::{
        artifacts::{PathwayProtocolKind, PathwayProtocolSessionKey, PathwayProtocolSpec},
        effects::{PathwayCheckpointEnvelope, PathwayProtocolObservation, PathwayProtocolRuntime},
    };

    #[derive(Default)]
    struct FakeEffects {
        checkpoints: BTreeMap<Vec<u8>, Vec<u8>>,
        observations: Vec<PathwayProtocolObservation>,
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

        fn store_bytes(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
            self.checkpoints.insert(key.to_vec(), value.to_vec());
            Ok(())
        }

        fn remove_bytes(&mut self, _key: &[u8]) -> Result<(), StorageError> {
            Ok(())
        }
    }

    impl PathwayProtocolRuntime for FakeEffects {
        fn now_tick(&self) -> Tick {
            Tick(4)
        }

        fn send_frame(
            &mut self,
            _frame: &crate::choreography::effects::PathwayChoreoFrame,
        ) -> Result<(), jacquard_core::TransportError> {
            Ok(())
        }

        fn store_held_payload(
            &mut self,
            _payload: &crate::choreography::effects::PathwayHeldPayload,
        ) -> Result<(), jacquard_core::RetentionError> {
            Ok(())
        }

        fn replay_held_payload(
            &mut self,
            _payload: &crate::choreography::effects::PathwayHeldPayload,
        ) -> Result<(), jacquard_core::RetentionError> {
            Ok(())
        }

        fn take_held_payload(
            &mut self,
            _object_id: &ContentId<Blake3Digest>,
        ) -> Result<Option<Vec<u8>>, jacquard_core::RetentionError> {
            Ok(None)
        }

        fn load_protocol_checkpoint(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
            Ok(self.checkpoints.get(key).cloned())
        }

        fn store_protocol_checkpoint(
            &mut self,
            checkpoint: &PathwayCheckpointEnvelope,
        ) -> Result<(), StorageError> {
            self.checkpoints
                .insert(checkpoint.key.clone(), checkpoint.bytes.clone());
            Ok(())
        }

        fn remove_protocol_checkpoint(&mut self, key: &[u8]) -> Result<(), StorageError> {
            self.checkpoints.remove(key);
            Ok(())
        }

        fn emit_protocol_observation(&mut self, observation: PathwayProtocolObservation) {
            self.observations.push(observation);
        }
    }

    #[test]
    fn route_and_tick_sessions_are_stable() {
        assert_eq!(
            route_session(PathwayProtocolKind::Activation, &RouteId([0xab; 16]),),
            PathwayProtocolSessionKey("activation-abababababababababababababababab".into())
        );
        assert_eq!(
            tick_session(RouteEpoch(7)),
            PathwayProtocolSessionKey("tick-epoch-7".into())
        );
    }

    #[test]
    fn guest_runtime_records_protocol_checkpoints() {
        let object_id = ContentId {
            digest: Blake3Digest([9; 32]),
        };
        let mut runtime = PathwayGuestRuntime::new(FakeEffects::default());
        runtime
            .retain_for_replay(&RouteId([3; 16]), object_id, b"payload")
            .expect("retain");
        runtime
            .activation_handshake(&RouteId([3; 16]), RouteEpoch(3))
            .expect("activation");
        runtime
            .record_tick_ingress(RouteEpoch(2), &[])
            .expect("record tick ingress");
        let hold_checkpoint = load_checkpoint(
            &runtime,
            PathwayProtocolKind::HoldReplay,
            &route_session(PathwayProtocolKind::HoldReplay, &RouteId([3; 16])),
        )
        .expect("hold checkpoint present");
        let tick_checkpoint = load_checkpoint(
            &runtime,
            PathwayProtocolKind::ForwardingHop,
            &tick_session(RouteEpoch(2)),
        )
        .expect("tick checkpoint present");

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
        fn failing_spec(_: PathwayProtocolKind) -> Result<&'static PathwayProtocolSpec, String> {
            Err("broken artifact".into())
        }

        let mut runtime =
            PathwayGuestRuntime::with_spec_resolver(FakeEffects::default(), failing_spec);
        let error = runtime
            .activation_handshake(&RouteId([7; 16]), RouteEpoch(2))
            .expect_err("invalid protocol artifact should fail closed");

        assert_eq!(error, RouteError::Runtime(RouteRuntimeError::Invalidated));
        assert!(runtime.effects.checkpoints.is_empty());
        assert!(runtime.effects.observations.is_empty());
    }

    fn load_checkpoint(
        runtime: &PathwayGuestRuntime<FakeEffects>,
        protocol: PathwayProtocolKind,
        session: &PathwayProtocolSessionKey,
    ) -> Option<PathwayProtocolCheckpoint> {
        let bytes = runtime
            .effects
            .checkpoints
            .get(&protocol_checkpoint_key(protocol, session))?
            .clone();
        decode_checkpoint_bytes(&bytes)
    }

    fn decode_checkpoint_bytes(bytes: &[u8]) -> Option<PathwayProtocolCheckpoint> {
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
