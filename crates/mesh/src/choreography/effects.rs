//! Mesh-owned choreography effect bridge.
//!
//! Control flow: choreography-facing runtime code talks only to this narrow
//! effect surface. The bridge stamps route events, stores protocol checkpoints,
//! polls ingress, and forwards retention operations onto the existing shared
//! mesh transport, retention, and runtime-effect traits.
//!
//! The boundary rule is that these mesh-private choreography effects are
//! not the shared Jacquard effect contract. Generated or protocol-local
//! effect interfaces stay private to `jacquard-mesh`. Concrete host/runtime
//! adapters implement the shared traits from `jacquard-traits`, and this
//! bridge interprets mesh choreography requests in terms of those stable
//! cross-engine traits.

use jacquard_core::{
    Blake3Digest, ContentId, LinkEndpoint, OrderStamp, RouteEvent, RouteEventStamped,
    StorageError, Tick, TransportObservation,
};
use jacquard_traits::{
    MeshFrame, MeshTransport, OrderEffects, RetentionStore, RouteEventLogEffects,
    StorageEffects, TimeEffects,
};

use crate::choreography::artifacts::{MeshProtocolKind, MeshProtocolSessionKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshChoreoFrame {
    pub(crate) endpoint: LinkEndpoint,
    pub(crate) payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshHeldPayload {
    pub(crate) object_id: ContentId<Blake3Digest>,
    pub(crate) payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshCheckpointEnvelope {
    pub(crate) key: Vec<u8>,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshProtocolObservation {
    pub(crate) protocol: MeshProtocolKind,
    pub(crate) session: MeshProtocolSessionKey,
    pub(crate) detail: &'static str,
}

pub(crate) trait MeshProtocolRuntime {
    fn now_tick(&self) -> Tick;

    fn next_order_stamp(&mut self) -> OrderStamp;

    fn send_mesh_frame(
        &mut self,
        frame: &MeshChoreoFrame,
    ) -> Result<(), jacquard_core::TransportError>;

    fn poll_mesh_ingress(
        &mut self,
    ) -> Result<Vec<TransportObservation>, jacquard_core::TransportError>;

    fn store_held_payload(
        &mut self,
        payload: &MeshHeldPayload,
    ) -> Result<(), jacquard_core::RetentionError>;

    fn replay_held_payload(
        &mut self,
        payload: &MeshHeldPayload,
    ) -> Result<(), jacquard_core::RetentionError>;

    fn take_held_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, jacquard_core::RetentionError>;

    fn record_protocol_event(
        &mut self,
        event: RouteEventStamped,
    ) -> Result<(), jacquard_core::RouteEventLogError>;

    fn load_protocol_checkpoint(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, StorageError>;

    fn store_protocol_checkpoint(
        &mut self,
        checkpoint: &MeshCheckpointEnvelope,
    ) -> Result<(), StorageError>;

    fn remove_protocol_checkpoint(&mut self, key: &[u8]) -> Result<(), StorageError>;

    fn emit_protocol_observation(&mut self, observation: MeshProtocolObservation);

    fn record_route_event(
        &mut self,
        event: RouteEvent,
    ) -> Result<(), jacquard_core::RouteEventLogError> {
        let order_stamp = self.next_order_stamp();
        let emitted_at_tick = self.now_tick();
        self.record_protocol_event(RouteEventStamped {
            order_stamp,
            emitted_at_tick,
            event,
        })
    }
}

// This adapter is the only place where mesh-private choreography requests are
// translated onto the shared Jacquard effect traits. The generated effect
// surface does not implement the shared traits directly; instead, one concrete
// host object supplies `TimeEffects`, `OrderEffects`, `StorageEffects`, and
// `RouteEventLogEffects`, and mesh interprets its private protocol requests in
// terms of that stable runtime boundary.
pub(crate) struct MeshProtocolRuntimeAdapter<'a, T, R, E> {
    pub(crate) transport: &'a mut T,
    pub(crate) retention: &'a mut R,
    pub(crate) effects: &'a mut E,
}

impl<T, R, E> MeshProtocolRuntime for MeshProtocolRuntimeAdapter<'_, T, R, E>
where
    T: MeshTransport,
    R: RetentionStore,
    E: RouteEventLogEffects + StorageEffects + TimeEffects + OrderEffects,
{
    fn now_tick(&self) -> Tick {
        self.effects.now_tick()
    }

    fn next_order_stamp(&mut self) -> OrderStamp {
        self.effects.next_order_stamp()
    }

    fn send_mesh_frame(
        &mut self,
        frame: &MeshChoreoFrame,
    ) -> Result<(), jacquard_core::TransportError> {
        self.transport.send_frame(MeshFrame {
            endpoint: &frame.endpoint,
            payload: &frame.payload,
        })
    }

    fn poll_mesh_ingress(
        &mut self,
    ) -> Result<Vec<TransportObservation>, jacquard_core::TransportError> {
        self.transport.poll_observations()
    }

    fn store_held_payload(
        &mut self,
        payload: &MeshHeldPayload,
    ) -> Result<(), jacquard_core::RetentionError> {
        self.retention
            .retain_payload(payload.object_id, payload.payload.clone())
    }

    fn replay_held_payload(
        &mut self,
        payload: &MeshHeldPayload,
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

    fn record_protocol_event(
        &mut self,
        event: RouteEventStamped,
    ) -> Result<(), jacquard_core::RouteEventLogError> {
        self.effects.record_route_event(event)
    }

    fn load_protocol_checkpoint(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, StorageError> {
        self.effects.load_bytes(key)
    }

    fn store_protocol_checkpoint(
        &mut self,
        checkpoint: &MeshCheckpointEnvelope,
    ) -> Result<(), StorageError> {
        self.effects.store_bytes(&checkpoint.key, &checkpoint.bytes)
    }

    fn remove_protocol_checkpoint(&mut self, key: &[u8]) -> Result<(), StorageError> {
        self.effects.remove_bytes(key)
    }

    fn emit_protocol_observation(&mut self, _observation: MeshProtocolObservation) {}
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        Blake3Digest, BleDeviceId, BleProfileId, ByteCount, ContentId, EndpointAddress,
        LinkEndpoint, NodeId, OrderStamp, RouteCommitmentId, RouteCommitmentResolution,
        RouteEvent, RouteEventStamped, RouteId, StorageError, Tick,
        TransportObservation, TransportProtocol,
    };
    use jacquard_traits::{effect_handler, OrderEffects, StorageEffects, TimeEffects};

    use super::{
        MeshCheckpointEnvelope, MeshChoreoFrame, MeshHeldPayload,
        MeshProtocolObservation, MeshProtocolRuntime, MeshProtocolRuntimeAdapter,
    };
    use crate::choreography::artifacts::{MeshProtocolKind, MeshProtocolSessionKey};

    #[derive(Default)]
    struct FakeTransport {
        sent: Vec<(TransportProtocol, Vec<u8>)>,
        observations: Vec<TransportObservation>,
    }

    impl jacquard_traits::MeshTransport for FakeTransport {
        fn transport_id(&self) -> TransportProtocol {
            TransportProtocol::BleGatt
        }

        fn send_frame(
            &mut self,
            frame: jacquard_traits::MeshFrame<'_>,
        ) -> Result<(), jacquard_core::TransportError> {
            self.sent
                .push((frame.endpoint.protocol.clone(), frame.payload.to_vec()));
            Ok(())
        }

        fn poll_observations(
            &mut self,
        ) -> Result<Vec<TransportObservation>, jacquard_core::TransportError> {
            Ok(self.observations.clone())
        }
    }

    #[derive(Default)]
    struct FakeRetention {
        payloads: BTreeMap<Vec<u8>, Vec<u8>>,
    }

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
        events: Vec<RouteEventStamped>,
        payloads: BTreeMap<Vec<u8>, Vec<u8>>,
        next_order: u64,
    }

    #[effect_handler]
    impl TimeEffects for FakeEffects {
        fn now_tick(&self) -> Tick {
            Tick(1)
        }
    }

    #[effect_handler]
    impl OrderEffects for FakeEffects {
        fn next_order_stamp(&mut self) -> OrderStamp {
            self.next_order += 1;
            OrderStamp(self.next_order)
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

    #[effect_handler]
    impl jacquard_traits::RouteEventLogEffects for FakeEffects {
        fn record_route_event(
            &mut self,
            event: RouteEventStamped,
        ) -> Result<(), jacquard_core::RouteEventLogError> {
            self.events.push(event);
            Ok(())
        }
    }

    #[test]
    fn fake_mesh_choreo_adapter_maps_runtime_actions() {
        let endpoint = LinkEndpoint {
            protocol: TransportProtocol::BleGatt,
            address: EndpointAddress::Ble {
                device_id: BleDeviceId(vec![1]),
                profile_id: BleProfileId([1; 16]),
            },
            mtu_bytes: ByteCount(128),
        };
        let mut transport = FakeTransport::default();
        transport
            .observations
            .push(TransportObservation::PayloadReceived {
                from_node_id: NodeId([9; 32]),
                endpoint: endpoint.clone(),
                payload: b"observed".to_vec(),
                observed_at_tick: Tick(1),
            });
        let mut retention = FakeRetention::default();
        let mut effects = FakeEffects::default();
        let mut adapter = MeshProtocolRuntimeAdapter {
            transport: &mut transport,
            retention: &mut retention,
            effects: &mut effects,
        };

        let frame = MeshChoreoFrame {
            endpoint: endpoint.clone(),
            payload: b"frame".to_vec(),
        };
        adapter.send_mesh_frame(&frame).expect("send mesh frame");
        let observations = adapter.poll_mesh_ingress().expect("poll ingress");

        let object_id = ContentId { digest: Blake3Digest([7; 32]) };
        let payload = MeshHeldPayload { object_id, payload: b"payload".to_vec() };
        adapter
            .store_held_payload(&payload)
            .expect("store held payload");
        adapter
            .replay_held_payload(&payload)
            .expect("replay held payload");
        let recovered = adapter
            .take_held_payload(&object_id)
            .expect("take held payload");

        let event = RouteEvent::RouteCommitmentUpdated {
            route_id: RouteId([3; 16]),
            commitment_id: RouteCommitmentId([4; 16]),
            resolution: RouteCommitmentResolution::Pending,
        };
        adapter
            .record_route_event(event.clone())
            .expect("record route event");

        let checkpoint = MeshCheckpointEnvelope {
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

        adapter.emit_protocol_observation(MeshProtocolObservation {
            protocol: MeshProtocolKind::Activation,
            session: MeshProtocolSessionKey("activation#1".into()),
            detail: "accepted",
        });

        assert_eq!(adapter.transport.sent.len(), 1);
        assert_eq!(observations.len(), 1);
        assert!(loaded.is_some());
        assert_eq!(adapter.effects.events.len(), 1);
        assert_eq!(adapter.effects.events[0].event, event);
        assert!(recovered.is_none());
        assert!(adapter.retention.payloads.is_empty());
        assert!(!adapter.effects.payloads.contains_key(&checkpoint.key));
    }
}
