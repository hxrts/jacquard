//! Observation extension traits.
//!
//! These traits are the boundary for teams contributing hardware-specific,
//! runtime-specific, or transport-adjacent observation logic without taking
//! ownership of canonical route state.

use jacquard_core::{RouteError, SharedObservation, TransportProtocol};
use jacquard_macros::purity;

#[purity(pure)]
/// Pure metadata for one observation extension.
///
/// This surface advertises identity and transport reach without embedding
/// routing-engine policy or batching semantics.
pub trait ObservationExtensionDescriptor {
    #[must_use]
    fn extension_id(&self) -> &str;

    #[must_use]
    fn supported_transports(&self) -> Vec<TransportProtocol>;
}

#[purity(effectful)]
/// Effectful runtime boundary for one observation extension.
///
/// The extension emits plain shared observations. Higher-level host logic may
/// later batch, diff, merge, checkpoint, or prioritize them, but the
/// extension boundary itself stays focused on what was observed.
pub trait ObservationExtension: ObservationExtensionDescriptor {
    fn poll_observations(&mut self) -> Result<Vec<SharedObservation>, RouteError>;
}
