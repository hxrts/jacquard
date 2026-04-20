//! Reusable adapter-support primitives for Jacquard transport/profile
//! implementers.
//!
//! `jacquard-adapter` exists to hold generic ownership, mailbox, bookkeeping,
//! and host-side observational helpers that concrete transports, profiles, and
//! hosts can reuse without pushing runtime-support infrastructure into
//! `jacquard-core` or `jacquard-traits`.
//!
//! ## Adapter Support Surface
//!
//! This crate owns transport-neutral adapter-side helpers such as bounded raw
//! ingress mailboxes, unresolved/resolved peer bookkeeping, in-flight claim
//! guards, transport-neutral endpoint conveniences, and host-side topology
//! read-model projectors. These helpers are reusable support primitives and
//! observational utilities, not shared world-model vocabulary and not
//! behavioral trait boundaries.
//!
//! ## Ownership
//!
//! `jacquard-adapter` may own generic support primitives for adapter tasks and
//! observational read models, but it must not:
//! - redefine shared world-model types that belong in `jacquard-core`
//! - publish capability or driver traits that belong in `jacquard-traits`
//! - encode transport-specific semantics that belong in concrete adapter crates
//! - implement router logic, engine logic, or canonical route publication
//! - stamp Jacquard `Tick` or `OrderStamp` internally
//!
//! Concrete adapters may depend on this crate for ownership scaffolding, but
//! canonical route truth, router progression, and transport-specific protocol
//! behavior all stay outside this crate.

#![forbid(unsafe_code)]

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
