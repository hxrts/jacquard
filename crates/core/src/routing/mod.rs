//! Route lifecycle, admission, commitments, and runtime-facing routing objects.
//!
//! The `routing` module is the shared boundary between the routing engine
//! implementations and the router control plane. It defines all the types that
//! cross that boundary: engine capabilities and admission artifacts, committee
//! selection results, replay-visible route events, substrate layering objects,
//! and the full runtime lifecycle vocabulary (materialization, leases, handles,
//! commitments, maintenance, and handoffs).
//!
//! Submodules:
//! - `admission` — engine capabilities, candidates, admission checks,
//!   witnesses.
//! - `committee` — committee selection and membership types.
//! - `events` — stamped route events visible to the replay and event log.
//! - `layering` — substrate requirements, capabilities, leases, and layer
//!   hints.
//! - `runtime` — route identity stamps, handles, leases, materialization,
//!   installation, maintenance, commitments, and router round outcomes.

mod admission;
mod committee;
mod events;
mod layering;
mod runtime;

pub use admission::*;
pub use committee::*;
pub use events::*;
pub use layering::*;
pub use runtime::*;
