//! Bounded transport ingress mailbox for staging raw ingress events before
//! they are stamped with Jacquard logical time.
//!
//! The mailbox is created via `transport_ingress_mailbox(capacity)`, which
//! returns three handles that together cover the full adapter-side lifecycle:
//! - `TransportIngressSender` — cloneable write handle used by the transport
//!   driver to emit raw ingress events from any thread.
//! - `TransportIngressReceiver` — single-owner drain handle used by the host
//!   bridge to collect and stamp events before routing.
//! - `TransportIngressNotifier` — cloneable generation-stamp handle that lets a
//!   bridge or scheduler observe whether the mailbox has changed since the last
//!   drain, enabling efficient blocking waits without polling.
//!
//! `TransportIngressClass` distinguishes payload frames from control frames.
//! Payload overflow is fail-open: excess frames are counted in
//! `TransportIngressDrain::dropped_payload_count` and silently discarded.
//! Control overflow is fail-closed: `ControlIngressOverflow` is returned so
//! the caller can take corrective action.

use std::{
    collections::VecDeque,
    fmt,
    sync::{Arc, Condvar, Mutex},
};

use jacquard_core::TransportIngressEvent;
use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportIngressClass {
    Payload,
    Control,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportIngressSendOutcome {
    Enqueued,
    DroppedPayload,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportIngressDrain {
    pub events: Vec<TransportIngressEvent>,
    pub dropped_payload_count: u64,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlIngressOverflow;

impl fmt::Display for ControlIngressOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("control ingress queue is full")
    }
}

impl std::error::Error for ControlIngressOverflow {}

#[derive(Default)]
struct MailboxState {
    events: VecDeque<TransportIngressEvent>,
    dropped_payload_count: u64,
    generation: u64,
}

struct SharedMailbox {
    state: Mutex<MailboxState>,
    changed: Condvar,
    capacity: usize,
}

impl SharedMailbox {
    fn bump_generation(state: &mut MailboxState) {
        state.generation = state.generation.saturating_add(1);
    }
}

#[derive(Clone)]
pub struct TransportIngressSender {
    shared: Arc<SharedMailbox>,
}

pub struct TransportIngressReceiver {
    shared: Arc<SharedMailbox>,
}

#[derive(Clone)]
pub struct TransportIngressNotifier {
    shared: Arc<SharedMailbox>,
}

#[must_use]
pub fn transport_ingress_mailbox(
    capacity: usize,
) -> (
    TransportIngressSender,
    TransportIngressReceiver,
    TransportIngressNotifier,
) {
    assert!(
        capacity > 0,
        "transport ingress mailbox capacity must be non-zero"
    );
    let shared = Arc::new(SharedMailbox {
        state: Mutex::new(MailboxState::default()),
        changed: Condvar::new(),
        capacity,
    });
    (
        TransportIngressSender { shared: Arc::clone(&shared) },
        TransportIngressReceiver { shared: Arc::clone(&shared) },
        TransportIngressNotifier { shared },
    )
}

impl TransportIngressSender {
    pub fn emit(
        &self,
        class: TransportIngressClass,
        event: TransportIngressEvent,
    ) -> Result<TransportIngressSendOutcome, ControlIngressOverflow> {
        let mut guard = self.shared.state.lock().expect("transport ingress lock");
        if guard.events.len() >= self.shared.capacity {
            if class == TransportIngressClass::Payload {
                guard.dropped_payload_count =
                    guard.dropped_payload_count.saturating_add(1);
                SharedMailbox::bump_generation(&mut guard);
                self.shared.changed.notify_all();
                return Ok(TransportIngressSendOutcome::DroppedPayload);
            }
            return Err(ControlIngressOverflow);
        }

        guard.events.push_back(event);
        SharedMailbox::bump_generation(&mut guard);
        self.shared.changed.notify_all();
        Ok(TransportIngressSendOutcome::Enqueued)
    }
}

impl TransportIngressReceiver {
    #[must_use]
    pub fn drain(&mut self) -> TransportIngressDrain {
        let mut guard = self.shared.state.lock().expect("transport ingress lock");
        let events = guard.events.drain(..).collect();
        let dropped_payload_count = std::mem::take(&mut guard.dropped_payload_count);
        TransportIngressDrain { events, dropped_payload_count }
    }
}

impl TransportIngressNotifier {
    #[must_use]
    pub fn snapshot(&self) -> u64 {
        self.shared
            .state
            .lock()
            .expect("transport ingress lock")
            .generation
    }

    #[must_use]
    pub fn has_changed_since(&self, snapshot: u64) -> bool {
        self.snapshot() != snapshot
    }

    pub fn wait_for_change(&self, snapshot: u64) {
        let mut guard = self.shared.state.lock().expect("transport ingress lock");
        while guard.generation == snapshot {
            guard = self
                .shared
                .changed
                .wait(guard)
                .expect("transport ingress condvar");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Barrier},
        thread,
    };

    use jacquard_core::{ByteCount, EndpointLocator, NodeId, TransportKind};

    use super::{
        transport_ingress_mailbox, TransportIngressClass, TransportIngressSendOutcome,
    };

    fn payload(byte: u8) -> jacquard_core::TransportIngressEvent {
        jacquard_core::TransportIngressEvent::PayloadReceived {
            from_node_id: NodeId([byte; 32]),
            endpoint: jacquard_core::LinkEndpoint::new(
                TransportKind::WifiAware,
                EndpointLocator::Opaque(vec![byte]),
                ByteCount(64),
            ),
            payload: vec![byte],
        }
    }

    #[test]
    fn payload_overflow_is_accounted_for_explicitly() {
        let (sender, mut receiver, _) = transport_ingress_mailbox(1);

        assert_eq!(
            sender
                .emit(TransportIngressClass::Payload, payload(1))
                .expect("enqueue payload"),
            TransportIngressSendOutcome::Enqueued
        );
        assert_eq!(
            sender
                .emit(TransportIngressClass::Payload, payload(2))
                .expect("drop payload"),
            TransportIngressSendOutcome::DroppedPayload
        );

        let drain = receiver.drain();
        assert_eq!(drain.events.len(), 1);
        assert_eq!(drain.dropped_payload_count, 1);
    }

    #[test]
    fn control_path_overflow_fails_closed() {
        let (sender, _, _) = transport_ingress_mailbox(1);

        sender
            .emit(TransportIngressClass::Control, payload(1))
            .expect("enqueue control");
        let error = sender
            .emit(TransportIngressClass::Control, payload(2))
            .expect_err("control overflow must fail closed");

        assert_eq!(error.to_string(), "control ingress queue is full");
    }

    #[test]
    fn receiver_drain_clears_events_and_drop_counts() {
        let (sender, mut receiver, _) = transport_ingress_mailbox(2);
        sender
            .emit(TransportIngressClass::Payload, payload(1))
            .expect("enqueue payload");
        sender
            .emit(TransportIngressClass::Payload, payload(2))
            .expect("enqueue payload");

        let first = receiver.drain();
        assert_eq!(first.events.len(), 2);
        assert_eq!(first.dropped_payload_count, 0);

        let second = receiver.drain();
        assert!(second.events.is_empty());
        assert_eq!(second.dropped_payload_count, 0);
    }

    #[test]
    fn notifier_wakes_after_ingress_change() {
        let (sender, _, notifier) = transport_ingress_mailbox(1);
        let snapshot = notifier.snapshot();
        let start = Arc::new(Barrier::new(2));
        let ready = Arc::clone(&start);
        let wait_notifier = notifier.clone();

        let handle = thread::spawn(move || {
            ready.wait();
            wait_notifier.wait_for_change(snapshot);
        });

        start.wait();
        sender
            .emit(TransportIngressClass::Payload, payload(9))
            .expect("enqueue payload");

        handle.join().expect("notifier waiter");
        assert!(notifier.has_changed_since(snapshot));
    }
}
