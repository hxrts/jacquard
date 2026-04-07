//! Mesh-owned choreography interpreter surface.
//!
//! Generated Telltale effect interfaces should ultimately be interpreted
//! through a narrow mesh-local boundary like this one. The router remains above
//! this layer: it provides shared tick context and shared checkpoint
//! orchestration, while mesh translates protocol-local effect requests onto the
//! existing shared transport, retention, event-log, and storage boundaries.

use jacquard_core::{
    Blake3Digest, ContentId, LinkEndpoint, RouteEventStamped, StorageError,
    TransportObservation,
};
use jacquard_traits::{MeshFrame, MeshTransport, RetentionStore, RouteEventLogEffects};

use crate::choreography::artifacts::{MeshProtocolKind, MeshProtocolSessionKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshChoreoFrame {
    pub(crate) endpoint: LinkEndpoint,
    pub(crate) payload:  Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshHeldPayload {
    pub(crate) object_id: ContentId<Blake3Digest>,
    pub(crate) payload:   Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshCheckpointEnvelope {
    pub(crate) key:   Vec<u8>,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshProtocolObservation {
    pub(crate) protocol: MeshProtocolKind,
    pub(crate) session:  MeshProtocolSessionKey,
    pub(crate) detail:   &'static str,
}

pub(crate) trait MeshChoreoEffects {
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

    fn emit_protocol_observation(
        &mut self,
        observation: MeshProtocolObservation,
    );
}

pub(crate) struct MeshChoreoAdapter<'a, T, R, L, S> {
    pub(crate) transport: &'a mut T,
    pub(crate) retention: &'a mut R,
    pub(crate) route_events: &'a mut L,
    pub(crate) storage: &'a mut S,
}

pub(crate) trait MeshCheckpointStore {
    fn load_mesh_checkpoint(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, StorageError>;

    fn store_mesh_checkpoint(
        &mut self,
        key: &[u8],
        value: &[u8],
    ) -> Result<(), StorageError>;
}

impl<T, R, L, S> MeshChoreoEffects for MeshChoreoAdapter<'_, T, R, L, S>
where
    T: MeshTransport,
    R: RetentionStore,
    L: RouteEventLogEffects,
    S: MeshCheckpointStore,
{
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

    fn record_protocol_event(
        &mut self,
        event: RouteEventStamped,
    ) -> Result<(), jacquard_core::RouteEventLogError> {
        self.route_events.record_route_event(event)
    }

    fn load_protocol_checkpoint(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, StorageError> {
        self.storage.load_mesh_checkpoint(key)
    }

    fn store_protocol_checkpoint(
        &mut self,
        checkpoint: &MeshCheckpointEnvelope,
    ) -> Result<(), StorageError> {
        self.storage
            .store_mesh_checkpoint(&checkpoint.key, &checkpoint.bytes)
    }

    fn emit_protocol_observation(
        &mut self,
        _observation: MeshProtocolObservation,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        Blake3Digest, BleDeviceId, BleProfileId, ByteCount, ContentId,
        EndpointAddress, LinkEndpoint, OrderStamp, RouteCommitmentId,
        RouteCommitmentResolution, RouteEvent, RouteEventStamped, RouteId, Tick,
        TransportObservation, TransportProtocol, NodeId,
    };

    use super::{
        MeshCheckpointEnvelope, MeshCheckpointStore, MeshChoreoAdapter, MeshChoreoEffects,
        MeshChoreoFrame, MeshHeldPayload, MeshProtocolObservation,
    };
    use crate::choreography::artifacts::{MeshProtocolKind, MeshProtocolSessionKey};
    use jacquard_traits::effect_handler;

    #[derive(Default)]
    struct FakeTransport {
        sent:         Vec<(TransportProtocol, Vec<u8>)>,
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
        payloads: std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
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
    struct FakeEventLog {
        events: Vec<RouteEventStamped>,
    }

    #[effect_handler]
    impl jacquard_traits::RouteEventLogEffects for FakeEventLog {
        fn record_route_event(
            &mut self,
            event: RouteEventStamped,
        ) -> Result<(), jacquard_core::RouteEventLogError> {
            self.events.push(event);
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeCheckpointStore {
        payloads: std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
    }

    impl MeshCheckpointStore for FakeCheckpointStore {
        fn load_mesh_checkpoint(
            &self,
            key: &[u8],
        ) -> Result<Option<Vec<u8>>, jacquard_core::StorageError> {
            Ok(self.payloads.get(key).cloned())
        }

        fn store_mesh_checkpoint(
            &mut self,
            key: &[u8],
            value: &[u8],
        ) -> Result<(), jacquard_core::StorageError> {
            self.payloads.insert(key.to_vec(), value.to_vec());
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
        transport.observations.push(TransportObservation::PayloadReceived {
            from_node_id: NodeId([9; 32]),
            endpoint: endpoint.clone(),
            payload: b"observed".to_vec(),
            observed_at_tick: Tick(1),
        });
        let mut retention = FakeRetention::default();
        let mut events = FakeEventLog::default();
        let mut checkpoints = FakeCheckpointStore::default();
        let mut adapter = MeshChoreoAdapter {
            transport: &mut transport,
            retention: &mut retention,
            route_events: &mut events,
            storage: &mut checkpoints,
        };

        let frame = MeshChoreoFrame {
            endpoint: endpoint.clone(),
            payload: b"frame".to_vec(),
        };
        adapter.send_mesh_frame(&frame).expect("send mesh frame");
        let observations = adapter.poll_mesh_ingress().expect("poll ingress");

        let object_id = ContentId {
            digest: Blake3Digest([7; 32]),
        };
        let payload = MeshHeldPayload {
            object_id,
            payload: b"payload".to_vec(),
        };
        adapter
            .store_held_payload(&payload)
            .expect("store held payload");
        adapter
            .replay_held_payload(&payload)
            .expect("replay held payload");

        let event = RouteEventStamped {
            event: RouteEvent::RouteCommitmentUpdated {
                route_id: RouteId([3; 16]),
                commitment_id: RouteCommitmentId([4; 16]),
                resolution: RouteCommitmentResolution::Pending,
            },
            order_stamp: OrderStamp(1),
            emitted_at_tick: Tick(1),
        };
        adapter
            .record_protocol_event(event.clone())
            .expect("record protocol event");

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

        adapter.emit_protocol_observation(MeshProtocolObservation {
            protocol: MeshProtocolKind::Activation,
            session: MeshProtocolSessionKey("activation#1".into()),
            detail: "accepted",
        });

        assert_eq!(adapter.transport.sent.len(), 1);
        assert_eq!(observations.len(), 1);
        assert!(loaded.is_some());
        assert_eq!(adapter.route_events.events, vec![event]);
        assert!(adapter.retention.payloads.is_empty());
    }
}
