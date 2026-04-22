//! Deterministic cast support helpers.
//!
//! This crate is the shared home for unicast, multicast, and broadcast cast
//! evidence shaping plus deterministic delivery support shaping. Transport-owned
//! profile crates record physical facts and pass bounded observations into these
//! helpers. The helpers sort, filter, and cap those observations into
//! deterministic evidence and route-neutral delivery support that profile and
//! host bridge code can translate into the normal input model.
//!
//! The crate intentionally depends only on `jacquard-core`. It does not own
//! host ingress plumbing, transport endpoint authoring, send drivers, router
//! state, or Mercator-specific routing behavior.

// proc-macro-scope: Cast support helpers use plain data shapes and no local proc macros.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

mod bounds;
mod broadcast;
mod common;
mod delivery;
mod multicast;
mod unicast;

pub use bounds::{
    CastEvidenceBounds, CAST_COPY_BUDGET_MAX, CAST_EVIDENCE_AGE_MS_MAX, CAST_FANOUT_COUNT_MAX,
    CAST_GROUP_COVERAGE_COUNT_MAX, CAST_RECEIVER_COUNT_MAX,
};
pub use broadcast::{
    shape_broadcast_evidence, BroadcastEvidence, BroadcastObservation,
    BroadcastReverseConfirmation, BroadcastSupportKind,
};
pub use common::{
    CastEvidenceError, CastEvidenceMeta, CastEvidencePolicy, CastEvidenceReport,
    ReceiverCoverageEvidence, ReceiverCoverageObservation,
};
pub use delivery::{
    shape_broadcast_delivery_support, shape_multicast_delivery_support,
    shape_unicast_delivery_support, BroadcastDeliverySupport, CastCoverageObjective,
    CastDeliveryMode, CastDeliveryObjective, CastDeliveryPolicy, CastDeliveryReport,
    CastDeliveryResourceUse, CastDeliverySupport, CastReceiverSet, MulticastDeliverySupport,
    UnicastDeliverySupport,
};
pub use multicast::{
    shape_multicast_evidence, CastGroupId, MulticastEvidence, MulticastObservation,
};
pub use unicast::{
    shape_unicast_evidence, UnicastEvidence, UnicastObservation, UnicastSupportKind,
};
