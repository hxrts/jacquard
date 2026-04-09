//! In-memory link authoring, transport, retention, and effect adapters.
//!
//! Control flow: callers define shared endpoints, choose preset-oriented links,
//! and attach transports to an in-memory network. The crate also exposes
//! deterministic retention and runtime-effect adapters for tests. It does not
//! plan routes or interpret mesh policy. It only provides reusable in-memory
//! infrastructure.
//!
//! Most callers should start with the [`authoring`] module, especially
//! [`ReferenceLink`]. [`SimulatedLinkProfile`] remains available as the
//! lower-level escape hatch when tests need exact control over `LinkProfile`
//! and `LinkState`. Callers construct shared `LinkEndpoint` values directly via
//! `jacquard-core`.
//!
//! Module map:
//! - [`authoring`]: human-facing link authoring presets
//! - [`state`]: low-level link profile/state builder
//! - `transport`: in-memory `TransportEffects` implementation
//! - `network`: shared in-memory carrier fabric
//! - `retention`: in-memory retention-store implementation
//! - `effect`: in-memory runtime-effect implementations
//!
//! ```rust
//! use jacquard_core::{
//!     ByteCount, EndpointLocator, LinkEndpoint, Tick, TransportKind,
//! };
//! use jacquard_mem_link_profile::ReferenceLink;
//!
//! let active = ReferenceLink::active(
//!     LinkEndpoint::new(
//!         TransportKind::WifiAware,
//!         EndpointLocator::Opaque(vec![7]),
//!         ByteCount(128),
//!     ),
//!     Tick(1),
//! )
//! .build();
//! let lossy = ReferenceLink::lossy(
//!     LinkEndpoint::new(
//!         TransportKind::WifiAware,
//!         EndpointLocator::Opaque(vec![8]),
//!         ByteCount(128),
//!     ),
//!     jacquard_core::RatioPermille(650),
//!     Tick(1),
//! )
//! .build();
//!
//! assert_eq!(active.state.state, jacquard_core::LinkRuntimeState::Active);
//! assert_eq!(
//!     lossy
//!         .state
//!         .delivery_confidence_permille
//!         .value_or(jacquard_core::RatioPermille(0)),
//!     jacquard_core::RatioPermille(650)
//! );
//! ```
//!
//! Ownership:
//! - `Observed`: link capability and transport observation surface only
//! - never mints canonical route truth or performs routing decisions

#![forbid(unsafe_code)]

pub mod authoring;
mod effect;
mod network;
mod retention;
mod state;
mod transport;

pub use authoring::{ReferenceLink, DEFAULT_REFERENCE_TRANSFER_RATE_BYTES_PER_SEC};
pub use effect::InMemoryRuntimeEffects;
pub use network::SharedInMemoryNetwork;
pub use retention::InMemoryRetentionStore;
pub use state::{
    SimulatedLinkProfile, DEFAULT_STABILITY_HORIZON_MS, REFERENCE_LATENCY_FLOOR_MS,
    REFERENCE_TYPICAL_RTT_MS,
};
pub use transport::InMemoryTransport;
