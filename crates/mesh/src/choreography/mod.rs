//! Mesh-local Telltale choreography surface.
//!
//! This module is the internal boundary between Jacquard mesh planning/runtime
//! code and Telltale's generated protocol surfaces. Mesh protocols are defined
//! inline with `tell!` so the generated session/effect code lives next to the
//! Rust host logic that enters those protocols.

mod activation;
mod artifacts;
mod effects;
mod forwarding;
mod handoff;
mod hold_replay;
mod repair;
mod runtime;

pub(crate) use effects::MeshProtocolRuntimeAdapter;
pub(crate) use runtime::MeshGuestRuntime;
