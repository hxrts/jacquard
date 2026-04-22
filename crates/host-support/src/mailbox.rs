//! Bounded transport ingress mailbox for staging raw ingress events before
//! they are stamped with Jacquard logical time.
//!
//! The mailbox is created via `transport_ingress_mailbox(capacity)`, which
//! returns three handles that together cover the full host-side lifecycle:
//! - `TransportIngressSender` — cloneable write handle used by the transport
//!   driver to emit raw ingress events from any thread.
//! - `TransportIngressReceiver` — single-owner drain handle used by the host
//!   bridge to collect and stamp events before routing.
//! - `TransportIngressNotifier` — cloneable generation-stamp handle that lets a
//!   bridge or scheduler observe whether the mailbox has changed since the last
//!   drain, enabling efficient blocking waits without polling and
//!   runtime-agnostic async waiting via [`TransportIngressNotifier::changed`].
//!
//! `TransportIngressClass` distinguishes payload frames from control frames.
//! Payload overflow is fail-open: excess frames are counted in
//! `TransportIngressDrain::dropped_payload_count` and silently discarded.
//! Control overflow is fail-closed: `ControlIngressOverflow` is returned so
//! the caller can take corrective action.

use alloc::{collections::VecDeque, vec::Vec};
use core::{
    fmt,
    future::Future,
    mem,
    pin::Pin,
    task::{Context, Poll, Waker},
};

#[cfg(not(feature = "std"))]
use alloc::rc::Rc;
#[cfg(not(feature = "std"))]
use core::cell::RefCell;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use std::sync::Condvar;
#[cfg(feature = "std")]
use std::sync::{Arc, Mutex, MutexGuard};

use jacquard_core::TransportIngressEvent;
use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use jacquard_core::DurationMs;

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

#[cfg(feature = "std")]
impl std::error::Error for ControlIngressOverflow {}

#[derive(Default)]
struct MailboxState {
    events: VecDeque<TransportIngressEvent>,
    dropped_payload_count: u64,
    generation: u64,
    waiter: Option<Waker>,
}

#[cfg(feature = "std")]
type SharedMailboxHandle = Arc<SharedMailbox>;

#[cfg(not(feature = "std"))]
type SharedMailboxHandle = Rc<SharedMailbox>;

struct SharedMailbox {
    storage: MailboxStorage,
    capacity: usize,
    notifier: MailboxChangeNotifier,
}

#[cfg(feature = "std")]
struct MailboxStorage {
    state: Mutex<MailboxState>,
}

#[cfg(not(feature = "std"))]
struct MailboxStorage {
    state: RefCell<MailboxState>,
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
struct MailboxChangeNotifier {
    changed: Condvar,
}

#[cfg(not(all(feature = "std", not(target_arch = "wasm32"))))]
#[derive(Clone, Copy, Debug, Default)]
struct MailboxChangeNotifier;

trait TransportIngressWake {
    fn wake_ingress_waiters(&self);
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
impl TransportIngressWake for MailboxChangeNotifier {
    fn wake_ingress_waiters(&self) {
        self.changed.notify_all();
    }
}

#[cfg(not(all(feature = "std", not(target_arch = "wasm32"))))]
impl TransportIngressWake for MailboxChangeNotifier {
    fn wake_ingress_waiters(&self) {}
}

impl SharedMailbox {
    fn bump_generation(state: &mut MailboxState) {
        state.generation = state.generation.saturating_add(1);
    }

    fn take_waiter(state: &mut MailboxState) -> Option<Waker> {
        state.waiter.take()
    }

    fn wake_waiter(waiter: Option<Waker>) {
        if let Some(waiter) = waiter {
            waiter.wake();
        }
    }

    fn notify_changed(&self) {
        self.notifier.wake_ingress_waiters();
    }

    #[cfg(feature = "std")]
    fn with_state<Output>(&self, operation: impl FnOnce(&mut MailboxState) -> Output) -> Output {
        let mut guard = self.lock_state();
        operation(&mut guard)
    }

    #[cfg(feature = "std")]
    fn lock_state(&self) -> MutexGuard<'_, MailboxState> {
        self.storage
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[cfg(not(feature = "std"))]
    fn with_state<Output>(&self, operation: impl FnOnce(&mut MailboxState) -> Output) -> Output {
        let mut guard = self.storage.state.borrow_mut();
        operation(&mut guard)
    }

    #[cfg(all(feature = "std", not(target_arch = "wasm32")))]
    #[expect(
        clippy::disallowed_types,
        reason = "Condvar and thread-parking APIs require std::time::Duration internally"
    )]
    fn std_duration(duration_ms: DurationMs) -> std::time::Duration {
        std::time::Duration::from_millis(u64::from(duration_ms.0))
    }
}

#[derive(Clone)]
pub struct TransportIngressSender {
    shared: SharedMailboxHandle,
}

pub struct TransportIngressReceiver {
    shared: SharedMailboxHandle,
}

#[derive(Clone)]
pub struct TransportIngressNotifier {
    shared: SharedMailboxHandle,
}

pub struct TransportIngressChanged<'a> {
    notifier: &'a TransportIngressNotifier,
    snapshot: u64,
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
    let shared = new_shared_mailbox(capacity);
    (
        TransportIngressSender {
            shared: shared.clone(),
        },
        TransportIngressReceiver {
            shared: shared.clone(),
        },
        TransportIngressNotifier { shared },
    )
}

#[cfg(feature = "std")]
fn new_shared_mailbox(capacity: usize) -> SharedMailboxHandle {
    Arc::new(SharedMailbox {
        storage: MailboxStorage {
            state: Mutex::new(MailboxState::default()),
        },
        capacity,
        notifier: new_change_notifier(),
    })
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
fn new_change_notifier() -> MailboxChangeNotifier {
    MailboxChangeNotifier {
        changed: Condvar::new(),
    }
}

#[cfg(all(feature = "std", target_arch = "wasm32"))]
fn new_change_notifier() -> MailboxChangeNotifier {
    MailboxChangeNotifier
}

#[cfg(not(feature = "std"))]
fn new_shared_mailbox(capacity: usize) -> SharedMailboxHandle {
    Rc::new(SharedMailbox {
        storage: MailboxStorage {
            state: RefCell::new(MailboxState::default()),
        },
        capacity,
        notifier: MailboxChangeNotifier,
    })
}

impl TransportIngressSender {
    pub fn emit(
        &self,
        class: TransportIngressClass,
        event: TransportIngressEvent,
    ) -> Result<TransportIngressSendOutcome, ControlIngressOverflow> {
        let (result, waiter) = self.shared.with_state(|state| {
            if state.events.len() >= self.shared.capacity {
                if class == TransportIngressClass::Payload {
                    state.dropped_payload_count = state.dropped_payload_count.saturating_add(1);
                    SharedMailbox::bump_generation(state);
                    let waiter = SharedMailbox::take_waiter(state);
                    return (Ok(TransportIngressSendOutcome::DroppedPayload), waiter);
                }
                return (Err(ControlIngressOverflow), None);
            }

            state.events.push_back(event);
            SharedMailbox::bump_generation(state);
            let waiter = SharedMailbox::take_waiter(state);
            (Ok(TransportIngressSendOutcome::Enqueued), waiter)
        });
        if waiter.is_some() || result.is_ok() {
            self.shared.notify_changed();
        }
        SharedMailbox::wake_waiter(waiter);
        result
    }
}

impl TransportIngressReceiver {
    #[must_use]
    pub fn drain(&mut self) -> TransportIngressDrain {
        self.shared.with_state(|state| TransportIngressDrain {
            events: state.events.drain(..).collect(),
            dropped_payload_count: mem::take(&mut state.dropped_payload_count),
        })
    }
}

impl TransportIngressNotifier {
    #[must_use]
    pub fn snapshot(&self) -> u64 {
        self.shared.with_state(|state| state.generation)
    }

    #[must_use]
    pub fn has_changed_since(&self, snapshot: u64) -> bool {
        self.snapshot() != snapshot
    }

    #[cfg(all(feature = "std", not(target_arch = "wasm32")))]
    pub fn wait_for_change(&self, snapshot: u64) {
        let mut guard = self.shared.lock_state();
        while guard.generation == snapshot {
            guard = self
                .shared
                .notifier
                .changed
                .wait(guard)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    #[cfg(all(feature = "std", not(target_arch = "wasm32")))]
    #[must_use]
    pub fn wait_for_change_within_ms(&self, snapshot: u64, wait_ms: DurationMs) -> bool {
        let guard = self.shared.lock_state();
        let std_wait = SharedMailbox::std_duration(wait_ms);
        let (guard, _) = self
            .shared
            .notifier
            .changed
            .wait_timeout_while(guard, std_wait, |state| state.generation == snapshot)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.generation != snapshot
    }

    #[must_use]
    pub fn changed(&self, snapshot: u64) -> TransportIngressChanged<'_> {
        TransportIngressChanged {
            notifier: self,
            snapshot,
        }
    }
}

impl Future for TransportIngressChanged<'_> {
    type Output = u64;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.notifier.shared.with_state(|state| {
            if state.generation != self.snapshot {
                return Poll::Ready(state.generation);
            }

            match &state.waiter {
                Some(waiter) if waiter.will_wake(cx.waker()) => {}
                _ => {
                    state.waiter = Some(cx.waker().clone());
                }
            }
            Poll::Pending
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Barrier,
        },
        task::{Context, Poll, Wake, Waker},
    };

    #[cfg(not(target_arch = "wasm32"))]
    use std::thread;

    use jacquard_core::{ByteCount, EndpointLocator, NodeId, TransportKind};

    #[cfg(not(target_arch = "wasm32"))]
    use jacquard_core::DurationMs;

    use super::{transport_ingress_mailbox, TransportIngressClass, TransportIngressSendOutcome};

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

    #[cfg(not(target_arch = "wasm32"))]
    #[expect(
        clippy::disallowed_types,
        reason = "std thread sleep and park APIs require std::time::Duration in tests"
    )]
    fn std_duration(duration_ms: DurationMs) -> std::time::Duration {
        std::time::Duration::from_millis(u64::from(duration_ms.0))
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
    #[cfg(not(target_arch = "wasm32"))]
    fn notifier_timeout_reports_when_no_change_arrives() {
        let (_, _, notifier) = transport_ingress_mailbox(1);
        let snapshot = notifier.snapshot();

        assert!(!notifier.wait_for_change_within_ms(snapshot, DurationMs(5)));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn notifier_timeout_reports_when_change_arrives() {
        let (sender, _, notifier) = transport_ingress_mailbox(1);
        let snapshot = notifier.snapshot();

        thread::spawn(move || {
            thread::sleep(std_duration(DurationMs(5)));
            sender
                .emit(TransportIngressClass::Payload, payload(7))
                .expect("enqueue payload");
        });

        assert!(notifier.wait_for_change_within_ms(snapshot, DurationMs(50)));
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
    #[cfg(not(target_arch = "wasm32"))]
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

    #[test]
    fn changed_future_wakes_after_ingress_change() {
        #[derive(Debug)]
        struct FlagWaker {
            woke: Arc<AtomicBool>,
            thread: thread::Thread,
        }

        impl Wake for FlagWaker {
            fn wake(self: Arc<Self>) {
                self.woke.store(true, Ordering::SeqCst);
                self.thread.unpark();
            }

            fn wake_by_ref(self: &Arc<Self>) {
                self.woke.store(true, Ordering::SeqCst);
                self.thread.unpark();
            }
        }

        let (sender, _, notifier) = transport_ingress_mailbox(1);
        let snapshot = notifier.snapshot();
        let woke = Arc::new(AtomicBool::new(false));
        let waker = Waker::from(Arc::new(FlagWaker {
            woke: Arc::clone(&woke),
            thread: thread::current(),
        }));
        let mut context = Context::from_waker(&waker);
        let mut changed = Box::pin(notifier.changed(snapshot));

        assert!(matches!(changed.as_mut().poll(&mut context), Poll::Pending));

        thread::spawn(move || {
            thread::sleep(std_duration(DurationMs(5)));
            sender
                .emit(TransportIngressClass::Payload, payload(8))
                .expect("enqueue payload");
        });

        while !woke.load(Ordering::SeqCst) {
            thread::park_timeout(std_duration(DurationMs(50)));
        }

        assert!(matches!(
            changed.as_mut().poll(&mut context),
            Poll::Ready(_)
        ));
    }

    #[test]
    fn changed_future_keeps_single_waiter_slot() {
        #[derive(Debug)]
        struct NoopWaker;

        impl Wake for NoopWaker {
            fn wake(self: Arc<Self>) {}
        }

        let (_, _, notifier) = transport_ingress_mailbox(1);
        let snapshot = notifier.snapshot();
        let first_waker = Waker::from(Arc::new(NoopWaker));
        let second_waker = Waker::from(Arc::new(NoopWaker));
        let mut first_context = Context::from_waker(&first_waker);
        let mut second_context = Context::from_waker(&second_waker);
        let mut first = Box::pin(notifier.changed(snapshot));
        let mut second = Box::pin(notifier.changed(snapshot));

        assert!(matches!(
            first.as_mut().poll(&mut first_context),
            Poll::Pending
        ));
        assert!(matches!(
            second.as_mut().poll(&mut second_context),
            Poll::Pending
        ));

        notifier.shared.with_state(|state| {
            assert!(state.waiter.is_some());
        });
    }
}
