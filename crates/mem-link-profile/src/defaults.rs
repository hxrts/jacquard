//! Discoverable reference defaults for the in-memory link preset surface.
//!
//! These values seed the human-facing preset path. Callers that need tighter
//! or looser parameters may override them through [`crate::LinkPreset`] or by
//! dropping to [`crate::SimulatedLinkProfile`].

pub use crate::{
    authoring::DEFAULT_REFERENCE_TRANSFER_RATE_BYTES_PER_SEC,
    state::{
        DEFAULT_DELIVERY_CONFIDENCE_PERMILLE, DEFAULT_LOSS_PERMILLE,
        DEFAULT_STABILITY_HORIZON_MS, DEFAULT_SYMMETRY_PERMILLE,
        REFERENCE_LATENCY_FLOOR_MS, REFERENCE_TYPICAL_RTT_MS,
    },
};
