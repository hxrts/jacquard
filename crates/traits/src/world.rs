//! World extension traits for typed topology observations.
//!
//! These traits are the extension boundary for contributors who need to add
//! hardware-specific, runtime-specific, or transport-adjacent observation logic
//! without taking ownership of canonical route state or forking the shared
//! world schema defined in `jacquard-core`.
//!
//! Key traits exported from this module:
//! - [`WorldExtensionDescriptor`] — pure metadata: identity and transport
//!   reach.
//! - [`WorldExtension<O>`] — generic effectful poll surface for any observation
//!   value type `O`; each typed facet trait blanket-implements this.
//! - [`NodeWorldExtension`] — contribute observed `Node` instances.
//! - [`LinkWorldExtension`] — contribute observed `Link` instances.
//! - [`EnvironmentWorldExtension`] — contribute local environment observations.
//! - [`ServiceWorldExtension`] — contribute shared `ServiceDescriptor` facts.
//! - [`TransportWorldExtension`] — contribute raw `TransportObservation`
//!   events.
//!
//! Extensions must not publish canonical route truth. They provide raw
//! observational input; the router and engines decide what to do with it.

use jacquard_core::{
    Environment, EnvironmentObservation, Link, LinkObservation, Node, NodeObservation, Observation,
    ServiceDescriptor, ServiceObservation, TransportKind, TransportObservation, WorldError,
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
    fn supported_transports(&self) -> Vec<TransportKind>;
}

#[purity(effectful)]
/// Generic effectful boundary for extensions that contribute typed
/// observations.
///
/// The type parameter `O` is the observation value type (e.g. `Node`, `Link`,
/// `ObservedValue`). Each of the five specific world-extension traits
/// (`NodeWorldExtension`, `LinkWorldExtension`, etc.) blanket-implements this
/// trait for their respective observation type, so any implementor of a
/// specific trait automatically satisfies `WorldExtension<O>` for the matching
/// `O`.
pub trait WorldExtension<O>: WorldExtensionDescriptor {
    must_use_evidence!("poll_observations", "observations";
        fn poll_observations(&mut self) -> Result<Vec<Observation<O>>, WorldError>;
    );
}

#[purity(effectful)]
/// Effectful runtime boundary for extensions that contribute observed nodes.
///
/// The shared `Node` schema remains fixed in `jacquard-core`. This trait lets
/// an extension add more observed node instances without redefining what a node
/// is.
pub trait NodeWorldExtension: WorldExtensionDescriptor {
    must_use_evidence!("poll_node_observations", "node observations";
        fn poll_node_observations(&mut self) -> Result<Vec<NodeObservation>, WorldError>;
    );
}

impl<T: NodeWorldExtension> WorldExtension<Node> for T {
    fn poll_observations(&mut self) -> Result<Vec<NodeObservation>, WorldError> {
        self.poll_node_observations()
    }
}

#[purity(effectful)]
/// Effectful runtime boundary for extensions that contribute observed links.
///
/// The shared `Link` schema remains fixed in `jacquard-core`. This trait lets
/// an extension add more observed link instances without redefining what a link
/// is.
pub trait LinkWorldExtension: WorldExtensionDescriptor {
    must_use_evidence!("poll_link_observations", "link observations";
        fn poll_link_observations(&mut self) -> Result<Vec<LinkObservation>, WorldError>;
    );
}

impl<T: LinkWorldExtension> WorldExtension<Link> for T {
    fn poll_observations(&mut self) -> Result<Vec<Observation<Link>>, WorldError> {
        self.poll_link_observations()
    }
}

#[purity(effectful)]
/// Effectful runtime boundary for extensions that contribute observed local or
/// neighborhood environment state.
pub trait EnvironmentWorldExtension: WorldExtensionDescriptor {
    must_use_evidence!("poll_environment_observations", "environment observations";
        fn poll_environment_observations(
            &mut self,
        ) -> Result<Vec<EnvironmentObservation>, WorldError>;
    );
}

impl<T: EnvironmentWorldExtension> WorldExtension<Environment> for T {
    fn poll_observations(&mut self) -> Result<Vec<Observation<Environment>>, WorldError> {
        self.poll_environment_observations()
    }
}

#[purity(effectful)]
/// Effectful runtime boundary for extensions that contribute observed shared
/// service descriptors.
pub trait ServiceWorldExtension: WorldExtensionDescriptor {
    must_use_evidence!("poll_service_observations", "service observations";
        fn poll_service_observations(
            &mut self,
        ) -> Result<Vec<ServiceObservation>, WorldError>;
    );
}

impl<T: ServiceWorldExtension> WorldExtension<ServiceDescriptor> for T {
    fn poll_observations(&mut self) -> Result<Vec<Observation<ServiceDescriptor>>, WorldError> {
        self.poll_service_observations()
    }
}

#[purity(effectful)]
/// Effectful runtime boundary for extensions that contribute observed transport
/// activity through the shared transport-observation vocabulary.
///
/// Connectivity surface: emits `TransportObservation` values that describe
/// raw link-level events, not typed routing semantics.
pub trait TransportWorldExtension: WorldExtensionDescriptor {
    must_use_evidence!("poll_transport_observations", "transport observations";
        fn poll_transport_observations(
            &mut self,
        ) -> Result<Vec<Observation<TransportObservation>>, WorldError>;
    );
}

impl<T: TransportWorldExtension> WorldExtension<TransportObservation> for T {
    fn poll_observations(&mut self) -> Result<Vec<Observation<TransportObservation>>, WorldError> {
        self.poll_transport_observations()
    }
}
