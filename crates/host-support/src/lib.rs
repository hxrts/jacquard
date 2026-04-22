//! Reusable host-support primitives for Jacquard transport/profile
//! implementers.
//!
//! `jacquard-host-support` exists to hold generic ownership, mailbox, bookkeeping,
//! and host-side observational helpers that concrete transports, profiles, and
//! hosts can reuse without pushing runtime-support infrastructure into
//! `jacquard-core` or `jacquard-traits`.
//!
//! ## Host Support Surface
//!
//! This crate owns transport-neutral host-side helpers such as bounded raw
//! ingress mailboxes, unresolved/resolved peer bookkeeping, in-flight claim
//! guards, transport-neutral endpoint conveniences, and host-side topology
//! read-model projectors. These helpers are reusable support primitives and
//! observational utilities, not shared world-model vocabulary and not
//! behavioral trait boundaries.
//!
//! ## Ownership
//!
//! `jacquard-host-support` may own generic support primitives for host tasks and
//! observational read models, but it must not:
//! - redefine shared world-model types that belong in `jacquard-core`
//! - publish capability or driver traits that belong in `jacquard-traits`
//! - encode transport-specific semantics that belong in concrete transport crates
//! - implement router logic, engine logic, or canonical route publication
//! - stamp Jacquard `Tick` or `OrderStamp` internally
//!
//! Concrete host and transport integrations may depend on this crate for ownership scaffolding, but
//! canonical route truth, router progression, and transport-specific protocol
//! behavior all stay outside this crate.

#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod claims;
mod decay_window;
mod dispatch;
mod endpoint;
mod mailbox;
mod ogm_receive_window;
mod peers;
mod topology;

pub use claims::*;
pub use decay_window::*;
pub use dispatch::*;
pub use endpoint::*;
pub use mailbox::*;
pub use ogm_receive_window::*;
pub use peers::*;
pub use topology::*;
