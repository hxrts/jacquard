//! World extension traits.
//!
//! These traits are the boundary for teams contributing hardware-specific,
//! runtime-specific, or transport-adjacent observation logic without taking
//! ownership of canonical route state or forking the shared world schema.

use jacquard_core::{
    EnvironmentObservation, LinkObservation, NodeObservation, Observation,
    ServiceObservation, TransportObservation, TransportProtocol, WorldError,
    WorldObservation,
};
use jacquard_macros::purity;

#[purity(pure)]
/// Pure metadata for one world extension.
///
/// This surface advertises identity and transport reach without embedding
/// routing-engine policy or batching semantics.
pub trait WorldExtensionDescriptor {
    #[must_use]
    fn extension_id(&self) -> &str;

    #[must_use]
    fn supported_transports(&self) -> Vec<TransportProtocol>;
}

#[purity(effectful)]
/// Effectful runtime boundary for extensions that contribute observed nodes.
///
/// The shared `Node` schema remains fixed in `jacquard-core`. This trait lets
/// an extension add more observed node instances without redefining what a node
/// is.
pub trait NodeWorldExtension: WorldExtensionDescriptor {
    fn poll_node_observations(&mut self) -> Result<Vec<NodeObservation>, WorldError>;
}

#[purity(effectful)]
/// Effectful runtime boundary for extensions that contribute observed links.
///
/// The shared `Link` schema remains fixed in `jacquard-core`. This trait lets
/// an extension add more observed link instances without redefining what a link
/// is.
pub trait LinkWorldExtension: WorldExtensionDescriptor {
    fn poll_link_observations(&mut self) -> Result<Vec<LinkObservation>, WorldError>;
}

#[purity(effectful)]
/// Effectful runtime boundary for extensions that contribute observed local or
/// neighborhood environment state.
pub trait EnvironmentWorldExtension: WorldExtensionDescriptor {
    fn poll_environment_observations(
        &mut self,
    ) -> Result<Vec<EnvironmentObservation>, WorldError>;
}

#[purity(effectful)]
/// Effectful runtime boundary for extensions that contribute observed shared
/// service descriptors.
pub trait ServiceWorldExtension: WorldExtensionDescriptor {
    fn poll_service_observations(
        &mut self,
    ) -> Result<Vec<ServiceObservation>, WorldError>;
}

#[purity(effectful)]
/// Effectful runtime boundary for extensions that contribute observed transport
/// activity through the shared transport-observation vocabulary.
pub trait TransportWorldExtension: WorldExtensionDescriptor {
    fn poll_transport_observations(
        &mut self,
    ) -> Result<Vec<Observation<TransportObservation>>, WorldError>;
}

#[purity(effectful)]
/// Effectful runtime boundary for one world extension.
///
/// The extension adds plain self-describing observations to the shared world.
/// Higher-level host logic may later batch, diff, merge, checkpoint, or
/// prioritize them, but the extension boundary itself stays focused on what was
/// observed. These surfaces report `WorldError` because they contribute world
/// input rather than owning routing semantics.
pub trait WorldExtension: WorldExtensionDescriptor {
    fn poll_observations(&mut self) -> Result<Vec<WorldObservation>, WorldError>;
}
