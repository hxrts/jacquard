//! Host-owned async-driver contracts.
//!
//! These traits describe supervision surfaces that belong to the host/runtime,
//! not to the deterministic routing-effect vocabulary. Drivers may own
//! streams, reconnect loops, bounded ingress queues, and shutdown semantics,
//! but they must not mutate router or engine state directly.

use jacquard_core::{TransportError, TransportIngressEvent};
use jacquard_macros::purity;

#[purity(effectful)]
/// Host-owned connectivity surface for raw transport ingress events.
///
/// This surface deliberately sits outside `#[effect_trait]` because it is not a
/// routing capability invoked during a synchronous round. It is a supervision
/// boundary owned by the host or bridge layer, and it governs raw transport
/// ingress ownership rather than deterministic router progression.
pub trait TransportDriver {
    must_use_evidence!("drain_transport_ingress", "transport ingress";
        fn drain_transport_ingress(
            &mut self,
        ) -> Result<Vec<TransportIngressEvent>, TransportError>;
    );

    #[must_use = "unchecked shutdown_transport_driver result silently discards shutdown failures"]
    fn shutdown_transport_driver(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
}
