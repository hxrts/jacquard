//! Mesh-local Telltale choreography surface.
//!
//! This module is the internal boundary between Jacquard mesh planning/runtime
//! code and Telltale's choreography compiler/runtime surfaces. Larger mesh
//! protocols live as `.tell` sources compiled through the normal Telltale
//! pipeline, while very small protocols can stay inline next to Rust glue.

mod artifacts;
mod effects;
mod runtime;

pub(crate) use artifacts::MeshProtocolKind;
pub(crate) use effects::MeshProtocolRuntimeAdapter;
pub(crate) use runtime::MeshGuestRuntime;
