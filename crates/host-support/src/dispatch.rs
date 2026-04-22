//! Bounded fail-closed dispatch mailbox for host-owned outbound work.
//!
//! This helper complements the raw ingress mailbox: hosts or bridges that
//! need to enqueue outbound commands from synchronous capability handlers can
//! use `dispatch_mailbox(capacity)` to obtain:
//! - `DispatchSender<T>` — cloneable bounded enqueue handle
//! - `DispatchReceiver<T>` — single-owner drain/inspection handle
//!
//! The mailbox is generic over `T` and stays transport-neutral. It does not
//! assign Jacquard time or ordering and it does not interpret the queued work.

use alloc::{collections::VecDeque, vec::Vec};
use core::fmt;

#[cfg(not(feature = "std"))]
use alloc::rc::Rc;
#[cfg(not(feature = "std"))]
use core::cell::RefCell;
#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatchSendOutcome {
    Enqueued,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispatchOverflow;

impl fmt::Display for DispatchOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("dispatch queue is full")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DispatchOverflow {}

struct SharedDispatch<T> {
    queue: DispatchQueue<T>,
    capacity: usize,
}

#[cfg(feature = "std")]
type SharedDispatchHandle<T> = Arc<SharedDispatch<T>>;

#[cfg(not(feature = "std"))]
type SharedDispatchHandle<T> = Rc<SharedDispatch<T>>;

#[cfg(feature = "std")]
type DispatchQueue<T> = Mutex<VecDeque<T>>;

#[cfg(not(feature = "std"))]
type DispatchQueue<T> = RefCell<VecDeque<T>>;

#[derive(Clone)]
pub struct DispatchSender<T> {
    shared: SharedDispatchHandle<T>,
}

pub struct DispatchReceiver<T> {
    shared: SharedDispatchHandle<T>,
}

#[cfg(feature = "std")]
fn new_queue<T>() -> DispatchQueue<T> {
    Mutex::new(VecDeque::new())
}

#[cfg(not(feature = "std"))]
fn new_queue<T>() -> DispatchQueue<T> {
    RefCell::new(VecDeque::new())
}

#[cfg(feature = "std")]
fn with_queue<T, Output>(
    queue: &DispatchQueue<T>,
    operation: impl FnOnce(&mut VecDeque<T>) -> Output,
) -> Output {
    let mut guard = queue
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation(&mut guard)
}

#[cfg(not(feature = "std"))]
fn with_queue<T, Output>(
    queue: &DispatchQueue<T>,
    operation: impl FnOnce(&mut VecDeque<T>) -> Output,
) -> Output {
    let mut guard = queue.borrow_mut();
    operation(&mut guard)
}

#[must_use]
pub fn dispatch_mailbox<T>(capacity: usize) -> (DispatchSender<T>, DispatchReceiver<T>) {
    assert!(capacity > 0, "dispatch mailbox capacity must be non-zero");
    let shared = SharedDispatchHandle::new(SharedDispatch {
        queue: new_queue(),
        capacity,
    });
    (
        DispatchSender {
            shared: shared.clone(),
        },
        DispatchReceiver { shared },
    )
}

impl<T> DispatchSender<T> {
    pub fn send(&self, item: T) -> Result<DispatchSendOutcome, DispatchOverflow> {
        with_queue(&self.shared.queue, |queue| {
            if queue.len() >= self.shared.capacity {
                return Err(DispatchOverflow);
            }
            queue.push_back(item);
            Ok(DispatchSendOutcome::Enqueued)
        })
    }
}

impl<T> DispatchReceiver<T> {
    #[must_use]
    pub fn drain(&mut self) -> Vec<T> {
        with_queue(&self.shared.queue, |queue| queue.drain(..).collect())
    }

    #[must_use]
    pub fn pending_len(&self) -> usize {
        with_queue(&self.shared.queue, |queue| queue.len())
    }
}

#[cfg(test)]
mod tests {
    use super::{dispatch_mailbox, DispatchSendOutcome};

    #[test]
    fn dispatch_mailbox_fails_closed_when_full() {
        let (sender, _) = dispatch_mailbox(1);

        assert_eq!(
            sender.send(1).expect("enqueue command"),
            DispatchSendOutcome::Enqueued
        );
        let error = sender
            .send(2)
            .expect_err("queue should fail closed when full");

        assert_eq!(error.to_string(), "dispatch queue is full");
    }

    #[test]
    fn dispatch_mailbox_drains_in_fifo_order() {
        let (sender, mut receiver) = dispatch_mailbox(4);
        sender.send(1).expect("enqueue first");
        sender.send(2).expect("enqueue second");

        let drained = receiver.drain();

        assert_eq!(drained, vec![1, 2]);
        assert_eq!(receiver.pending_len(), 0);
    }
}
