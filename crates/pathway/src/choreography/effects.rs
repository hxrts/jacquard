//! Mesh-owned choreography effect bridge.
//!
//! Control flow: choreography-facing runtime code talks only to this narrow
//! effect surface. The bridge stores protocol checkpoints and forwards
//! retention operations onto the shared transport sender, retention, and
//! runtime-effect traits.
//!
//! The boundary rule is that these mesh-private choreography effects are
//! not the shared Jacquard effect contract. Generated or protocol-local
//! effect interfaces stay private to `jacquard-pathway`. Concrete host/runtime
//! adapters implement the shared traits from `jacquard-traits`, and this
//! bridge interprets pathway choreography requests in terms of those stable
//! cross-engine traits after the router has already delivered any explicit
//! transport ingress for the current round.

use jacquard_core::{
    Blake3Digest, ContentId, LinkEndpoint, RouteError, RouteRuntimeError, StorageError,
    Tick,
};
use jacquard_traits::{
    RetentionStore, StorageEffects, TimeEffects, TransportSenderEffects,
};

/// Extension trait for converting choreography protocol errors into
/// `RouteError::Runtime(MaintenanceFailed)`.
pub(crate) trait ChoreographyResultExt<T> {
    fn choreography_failed(self) -> Result<T, RouteError>;
}

impl<T, E> ChoreographyResultExt<T> for Result<T, E> {
    fn choreography_failed(self) -> Result<T, RouteError> {
        match self {
            | Ok(value) => Ok(value),
            | Err(_) => Err(RouteError::Runtime(RouteRuntimeError::MaintenanceFailed)),
        }
    }
}

/// Extension trait for converting encoding/storage errors into
/// `RouteError::Runtime(Invalidated)`.
pub(crate) trait InvalidatedResultExt<T> {
    fn invalidated(self) -> Result<T, RouteError>;
}

impl<T, E> InvalidatedResultExt<T> for Result<T, E> {
    fn invalidated(self) -> Result<T, RouteError> {
        match self {
            | Ok(value) => Ok(value),
            | Err(_) => Err(RouteError::Runtime(RouteRuntimeError::Invalidated)),
        }
    }
}

use crate::choreography::artifacts::{PathwayProtocolKind, PathwayProtocolSessionKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathwayChoreoFrame {
    pub(crate) endpoint: LinkEndpoint,
    pub(crate) payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathwayHeldPayload {
    pub(crate) object_id: ContentId<Blake3Digest>,
    pub(crate) payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathwayCheckpointEnvelope {
    pub(crate) key: Vec<u8>,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathwayProtocolObservation {
    pub(crate) protocol: PathwayProtocolKind,
    pub(crate) protocol_name: String,
    pub(crate) role_names: Vec<String>,
    pub(crate) session: PathwayProtocolSessionKey,
    pub(crate) detail: &'static str,
}

pub(crate) trait PathwayProtocolRuntime {
    fn now_tick(&self) -> Tick;

    fn send_mesh_frame(
        &mut self,
        frame: &PathwayChoreoFrame,
    ) -> Result<(), jacquard_core::TransportError>;

    fn store_held_payload(
        &mut self,
        payload: &PathwayHeldPayload,
    ) -> Result<(), jacquard_core::RetentionError>;

    fn replay_held_payload(
        &mut self,
        payload: &PathwayHeldPayload,
    ) -> Result<(), jacquard_core::RetentionError>;

    fn take_held_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, jacquard_core::RetentionError>;

    fn load_protocol_checkpoint(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, StorageError>;

    fn store_protocol_checkpoint(
        &mut self,
        checkpoint: &PathwayCheckpointEnvelope,
    ) -> Result<(), StorageError>;

    fn remove_protocol_checkpoint(&mut self, key: &[u8]) -> Result<(), StorageError>;

    fn emit_protocol_observation(&mut self, observation: PathwayProtocolObservation);
}

// This adapter is the only place where mesh-private choreography requests are
// translated onto the shared Jacquard effect traits. The generated effect
// surface does not implement the shared traits directly; instead, one concrete
// host object supplies `TimeEffects`, `OrderEffects`, `StorageEffects`, and
// `RouteEventLogEffects`, and mesh interprets its private protocol requests in
// terms of that stable runtime boundary.
pub(crate) struct PathwayProtocolRuntimeAdapter<'a, T, R, E> {
    pub(crate) transport: &'a mut T,
    pub(crate) retention: &'a mut R,
    pub(crate) effects: &'a mut E,
}

impl<T, R, E> PathwayProtocolRuntime for PathwayProtocolRuntimeAdapter<'_, T, R, E>
where
    T: TransportSenderEffects,
    R: RetentionStore,
    E: StorageEffects + TimeEffects,
{
    fn now_tick(&self) -> Tick {
        self.effects.now_tick()
    }

    fn send_mesh_frame(
        &mut self,
        frame: &PathwayChoreoFrame,
    ) -> Result<(), jacquard_core::TransportError> {
        self.transport
            .send_transport(&frame.endpoint, &frame.payload)
    }

    fn store_held_payload(
        &mut self,
        payload: &PathwayHeldPayload,
    ) -> Result<(), jacquard_core::RetentionError> {
        self.retention
            .retain_payload(payload.object_id, payload.payload.clone())
    }

    fn replay_held_payload(
        &mut self,
        payload: &PathwayHeldPayload,
    ) -> Result<(), jacquard_core::RetentionError> {
        if self
            .retention
            .contains_retained_payload(&payload.object_id)?
        {
            let _ = self.retention.take_retained_payload(&payload.object_id)?;
        }
        Ok(())
    }

    fn take_held_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, jacquard_core::RetentionError> {
        self.retention.take_retained_payload(object_id)
    }

    fn load_protocol_checkpoint(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, StorageError> {
        self.effects.load_bytes(key)
    }

    fn store_protocol_checkpoint(
        &mut self,
        checkpoint: &PathwayCheckpointEnvelope,
    ) -> Result<(), StorageError> {
        self.effects.store_bytes(&checkpoint.key, &checkpoint.bytes)
    }

    fn remove_protocol_checkpoint(&mut self, key: &[u8]) -> Result<(), StorageError> {
        self.effects.remove_bytes(key)
    }

    fn emit_protocol_observation(&mut self, _observation: PathwayProtocolObservation) {}
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        Blake3Digest, ByteCount, ContentId, EndpointLocator, LinkEndpoint,
        StorageError, Tick, TransportKind,
    };
    use jacquard_traits::{
        effect_handler, StorageEffects, TimeEffects, TransportSenderEffects,
    };

    use super::{
        PathwayCheckpointEnvelope, PathwayChoreoFrame, PathwayHeldPayload,
        PathwayProtocolObservation, PathwayProtocolRuntime,
        PathwayProtocolRuntimeAdapter,
    };
    use crate::choreography::artifacts::{
        PathwayProtocolKind, PathwayProtocolSessionKey,
    };

    #[derive(Default)]
    struct FakeTransport {
        sent: Vec<(TransportKind, Vec<u8>)>,
    }

    #[effect_handler]
    impl TransportSenderEffects for FakeTransport {
        fn send_transport(
            &mut self,
            endpoint: &LinkEndpoint,
            payload: &[u8],
        ) -> Result<(), jacquard_core::TransportError> {
            self.sent
                .push((endpoint.transport_kind.clone(), payload.to_vec()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeRetention {
        payloads: BTreeMap<Vec<u8>, Vec<u8>>,
    }

    #[effect_handler]
    impl jacquard_traits::RetentionStore for FakeRetention {
        fn retain_payload(
            &mut self,
            object_id: ContentId<Blake3Digest>,
            payload: Vec<u8>,
        ) -> Result<(), jacquard_core::RetentionError> {
            self.payloads.insert(object_id.digest.0.to_vec(), payload);
            Ok(())
        }

        fn take_retained_payload(
            &mut self,
            object_id: &ContentId<Blake3Digest>,
        ) -> Result<Option<Vec<u8>>, jacquard_core::RetentionError> {
            Ok(self.payloads.remove(object_id.digest.0.as_slice()))
        }

        fn contains_retained_payload(
            &self,
            object_id: &ContentId<Blake3Digest>,
        ) -> Result<bool, jacquard_core::RetentionError> {
            Ok(self.payloads.contains_key(object_id.digest.0.as_slice()))
        }
    }

    #[derive(Default)]
    struct FakeEffects {
        payloads: BTreeMap<Vec<u8>, Vec<u8>>,
    }

    #[effect_handler]
    impl TimeEffects for FakeEffects {
        fn now_tick(&self) -> Tick {
            Tick(1)
        }
    }

    #[effect_handler]
    impl StorageEffects for FakeEffects {
        fn load_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
            Ok(self.payloads.get(key).cloned())
        }

        fn store_bytes(
            &mut self,
            key: &[u8],
            value: &[u8],
        ) -> Result<(), StorageError> {
            self.payloads.insert(key.to_vec(), value.to_vec());
            Ok(())
        }

        fn remove_bytes(&mut self, key: &[u8]) -> Result<(), StorageError> {
            self.payloads.remove(key);
            Ok(())
        }
    }

    // long-block-exception: comprehensive adapter contract verification
    #[test]
    fn fake_mesh_choreo_adapter_maps_runtime_actions() {
        let endpoint = LinkEndpoint::new(
            TransportKind::WifiAware,
            EndpointLocator::Opaque(vec![1]),
            ByteCount(128),
        );
        let mut transport = FakeTransport::default();
        let mut retention = FakeRetention::default();
        let mut effects = FakeEffects::default();
        let mut adapter = PathwayProtocolRuntimeAdapter {
            transport: &mut transport,
            retention: &mut retention,
            effects: &mut effects,
        };

        let frame = PathwayChoreoFrame {
            endpoint: endpoint.clone(),
            payload: b"frame".to_vec(),
        };
        adapter.send_mesh_frame(&frame).expect("send mesh frame");

        let object_id = ContentId { digest: Blake3Digest([7; 32]) };
        let payload = PathwayHeldPayload { object_id, payload: b"payload".to_vec() };
        adapter
            .store_held_payload(&payload)
            .expect("store held payload");
        adapter
            .replay_held_payload(&payload)
            .expect("replay held payload");
        let recovered = adapter
            .take_held_payload(&object_id)
            .expect("take held payload");

        let checkpoint = PathwayCheckpointEnvelope {
            key: b"mesh/choreo/activation".to_vec(),
            bytes: b"checkpoint".to_vec(),
        };
        adapter
            .store_protocol_checkpoint(&checkpoint)
            .expect("store checkpoint");
        let loaded = adapter
            .load_protocol_checkpoint(&checkpoint.key)
            .expect("load checkpoint");
        adapter
            .remove_protocol_checkpoint(&checkpoint.key)
            .expect("remove checkpoint");

        adapter.emit_protocol_observation(PathwayProtocolObservation {
            protocol: PathwayProtocolKind::Activation,
            protocol_name: "ActivationHandshake".into(),
            role_names: vec!["CurrentOwner".into(), "Destination".into()],
            session: PathwayProtocolSessionKey("activation#1".into()),
            detail: "accepted",
        });

        assert_eq!(adapter.transport.sent.len(), 1);
        assert!(loaded.is_some());
        assert!(recovered.is_none());
        assert!(adapter.retention.payloads.is_empty());
        assert!(!adapter.effects.payloads.contains_key(&checkpoint.key));
    }
}
