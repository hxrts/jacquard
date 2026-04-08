//! In-memory link authoring, transport, retention, and effect adapters.
//!
//! Control flow: callers define endpoints, choose preset-oriented links, and
//! attach transports to an in-memory network. The crate also exposes
//! deterministic retention and runtime-effect adapters for tests. It does not
//! plan routes or interpret mesh policy. It only provides reusable in-memory
//! infrastructure.
//!
//! Most callers should start with the [`authoring`] module, especially
//! [`ReferenceLink`]. [`SimulatedLinkProfile`] remains available as the
//! lower-level escape hatch when tests need exact control over `LinkProfile`
//! and `LinkState`.
//!
//! Module map:
//! - [`authoring`]: human-facing link authoring presets
//! - [`endpoint`]: reusable endpoint constructors
//! - [`state`]: low-level link profile/state builder
//! - `transport`: in-memory `TransportEffects` implementation
//! - `network`: shared in-memory carrier fabric
//! - `retention`: in-memory retention-store implementation
//! - `effect`: in-memory runtime-effect implementations
//!
//! ```rust
//! use jacquard_core::Tick;
//! use jacquard_mem_link_profile::ReferenceLink;
//!
//! let active = ReferenceLink::ble_active(7, Tick(1)).build();
//! let lossy =
//!     ReferenceLink::ble_lossy(8, jacquard_core::RatioPermille(650), Tick(1)).build();
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
mod endpoint;
mod network;
mod retention;
mod state;
mod transport;

pub use authoring::{ReferenceLink, DEFAULT_REFERENCE_TRANSFER_RATE_BYTES_PER_SEC};
pub use effect::InMemoryRuntimeEffects;
pub use endpoint::{ble_endpoint, opaque_endpoint, BLE_MTU_BYTES};
pub use network::SharedInMemoryNetwork;
pub use retention::InMemoryRetentionStore;
pub use state::{
    SimulatedLinkProfile, BLE_LATENCY_FLOOR_MS, BLE_TYPICAL_RTT_MS,
    DEFAULT_STABILITY_HORIZON_MS,
};
pub use transport::InMemoryTransport;
