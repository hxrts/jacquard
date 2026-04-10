//! In-memory link authoring, transport, retention, and effect adapters.
//!
//! Most callers should start with [`LinkPreset`]. The low-level
//! `SimulatedLinkProfile` builder stays available when a test needs exact
//! control over `LinkProfile` and `LinkState`.
//!
//! Module map:
//! - [`authoring`]: human-facing link authoring presets
//! - [`state`]: low-level link profile/state builder
//! - `transport`: in-memory transport sender + driver implementation
//! - `network`: shared in-memory carrier fabric
//! - `retention`: in-memory retention-store implementation
//! - `effect`: in-memory runtime-effect implementations
//!
//! ```rust
//! use jacquard_adapter::opaque_endpoint;
//! use jacquard_core::{ByteCount, Tick, TransportKind};
//! use jacquard_mem_link_profile::{LinkPreset, LinkPresetOptions};
//!
//! let active = LinkPreset::active(LinkPresetOptions::new(
//!     opaque_endpoint(TransportKind::WifiAware, vec![7], ByteCount(128)),
//!     Tick(1),
//! ))
//! .build();
//! let lossy = LinkPreset::lossy(
//!     LinkPresetOptions::new(
//!         opaque_endpoint(TransportKind::WifiAware, vec![8], ByteCount(128)),
//!         Tick(1),
//!     )
//!     .with_confidence(jacquard_core::RatioPermille(650)),
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
//! Starter path:
//! 1. Construct an endpoint with `jacquard_adapter::opaque_endpoint`.
//! 2. Choose a `LinkPreset` constructor such as `active`, `lossy`, or
//!    `recoverable`.
//! 3. Use `LinkPresetOptions` for the common setup path.
//! 4. Drop to `SimulatedLinkProfile` only when the low-level profile/state
//!    split matters to the test.
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

pub use authoring::{LinkPreset, LinkPresetOptions, DEFAULT_REFERENCE_TRANSFER_RATE_BYTES_PER_SEC};
pub use effect::InMemoryRuntimeEffects;
pub use network::SharedInMemoryNetwork;
pub use retention::InMemoryRetentionStore;
pub use state::{
    SimulatedLinkProfile, DEFAULT_DELIVERY_CONFIDENCE_PERMILLE, DEFAULT_LOSS_PERMILLE,
    DEFAULT_STABILITY_HORIZON_MS, DEFAULT_SYMMETRY_PERMILLE, REFERENCE_LATENCY_FLOOR_MS,
    REFERENCE_TYPICAL_RTT_MS,
};
pub use transport::InMemoryTransport;
