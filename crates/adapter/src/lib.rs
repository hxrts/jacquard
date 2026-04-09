//! Reusable adapter-support primitives for Jacquard transport/profile
//! implementers.
//!
//! `jacquard-adapter` exists to hold generic ownership, mailbox, and
//! bookkeeping helpers that concrete transport/profile adapters can reuse
//! without pushing runtime-support infrastructure into `jacquard-core` or
//! `jacquard-traits`.
//!
//! ## Adapter Support Surface
//!
//! This crate owns transport-neutral adapter-side helpers such as bounded raw
//! ingress mailboxes, unresolved/resolved peer bookkeeping, and in-flight claim
//! guards. These helpers are reusable support primitives, not shared world
//! model vocabulary and not behavioral trait boundaries.
//!
//! ## Ownership
//!
//! `jacquard-adapter` may own generic support primitives for adapter tasks, but
//! it must not:
//! - redefine shared world-model types that belong in `jacquard-core`
//! - publish capability or driver traits that belong in `jacquard-traits`
//! - encode transport-specific semantics that belong in concrete adapter crates
//! - stamp Jacquard `Tick` or `OrderStamp` internally
//!
//! Concrete adapters may depend on this crate for ownership scaffolding, but
//! canonical route truth, router progression, and transport-specific protocol
//! behavior all stay outside this crate.

#![forbid(unsafe_code)]

mod claims;
mod dispatch;
mod endpoint;
mod mailbox;
mod peers;

pub use claims::*;
pub use dispatch::*;
pub use endpoint::*;
pub use mailbox::*;
pub use peers::*;
