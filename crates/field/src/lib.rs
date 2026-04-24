//! First-party deterministic field-style routing engine for Jacquard.
//!
//! `FieldEngine` implements the shared planner/runtime contracts while keeping
//! its corridor belief state private. The engine publishes
//! `RouteShapeVisibility::CorridorEnvelope`: it can make conservative
//! end-to-end claims without claiming an explicit hop-by-hop path.
//!
//! The corridor planner is now baseline-only for the coded-diffusion research
//! path. New research code belongs under the `research` boundary, whose
//! vocabulary is message fragments, coding rank, fragment custody,
//! innovative/duplicate arrivals, diffusion pressure, and reconstruction
//! quorum.
//!
//! The implementation is intentionally split into thin modules so the
//! continuously updated field model, Telltale-backed search substrate, and
//! private protocol runtime can evolve without changing the shared engine
//! surface:
//! - `engine` defines the engine type, identity, and baseline capabilities.
//! - `research` defines the coded-diffusion research-path vocabulary.
//! - `planner` implements the shared planning surface.
//! - `runtime` implements materialization, maintenance, and forwarding hooks.
//! - `search` freezes field snapshots and runs exact Telltale search while
//!   keeping the public result shape as a corridor-envelope claim.
//! - `summary`, `observer`, `control`, and `choreography` own the
//!   continuously refreshed evidence path behind that planner/runtime surface.
//!
//! Verification notes for the first formal model live under
//! `verification/Field/Docs/`:
//! - `verification/Field/Docs/Model.md`
//! - `verification/Field/Docs/Protocol.md`
//! - `verification/Field/Docs/Adequacy.md`
//! - `verification/Field/Docs/Guide.md`
//! - `verification/Field/Docs/Parity.md`
//!
//! The current proof boundary is intentionally narrow and explicit:
//! - Lean covers a bounded local observer-controller model
//! - Lean covers a reduced private summary-exchange protocol boundary
//! - Lean covers a reduced runtime-search adequacy bridge
//! - Lean does not own canonical route publication or router lifecycle truth
//! - Lean does not own richer choreography/runtime internals outside the
//!   reduced proof-facing surfaces

#![forbid(unsafe_code)]

mod attractor;
mod choreography;
mod control;
mod engine;
mod observer;
mod operational;
mod planner;
mod planner_model;
mod policy;
mod recovery;
mod research;
mod route;
mod runtime;
mod search;
mod state;
mod summary;
#[cfg(test)]
mod validation;

pub use choreography::{
    BlockedReceiveMarker, FieldChoreographyRoundResult, FieldExecutionPolicyClass,
    FieldHostWaitStatus, FieldProtocolArtifact, FieldProtocolArtifactDetail, FieldProtocolKind,
    FieldProtocolReconfiguration, FieldProtocolReconfigurationCause, FieldProtocolSessionKey,
    FieldRoundDisposition,
};
pub use engine::{
    FieldCommitmentReplayEntry, FieldCommitmentReplaySurface, FieldEngine,
    FieldExportedPolicyEvent, FieldExportedProtocolArtifact, FieldExportedProtocolReconfiguration,
    FieldExportedProtocolReplay, FieldExportedRecoveryEntry, FieldExportedRecoveryReplay,
    FieldExportedReplayBundle, FieldExportedRuntimeRoundArtifact,
    FieldExportedRuntimeRouteArtifact, FieldExportedRuntimeSearchReplay, FieldExportedSearchEpoch,
    FieldExportedSearchExecutionPolicy, FieldExportedSearchProjection, FieldExportedSearchQuery,
    FieldExportedSearchReconfiguration, FieldExportedSelectedResult,
    FieldForwardSummaryObservation, FieldLeanProtocolFixture, FieldLeanRecoveryFixture,
    FieldLeanReplayFixture, FieldLeanRuntimeLinkageFixture, FieldLeanSearchFixture,
    FieldPolicyEvent, FieldPolicyGate, FieldPolicyReason, FieldProtocolReplaySurface,
    FieldRecoveryReplayEntry, FieldRecoveryReplaySurface, FieldReducedObjectiveClass,
    FieldReducedProtocolArtifact, FieldReducedProtocolReconfiguration, FieldReducedProtocolReplay,
    FieldReducedProtocolSession, FieldReducedQueryKind, FieldReducedRuntimeSearchReplay,
    FieldReducedSearchExecutionPolicy, FieldReducedSearchProjection, FieldReducedSearchQuery,
    FieldReducedSelectedResult, FieldReplaySnapshot, FieldReplaySurfaceClass,
    FieldRouterAnalysisRouteSummary, FieldRouterAnalysisSnapshot, FieldRuntimeReplaySurface,
    FieldRuntimeRoundArtifact, FieldRuntimeRouteArtifact, FieldSearchReplaySurface,
    FIELD_CAPABILITIES, FIELD_ENGINE_ID, FIELD_POLICY_EVENT_RETENTION_MAX,
    FIELD_REPLAY_SURFACE_VERSION, FIELD_RUNTIME_ROUND_ARTIFACT_RETENTION_MAX,
};
pub use planner_model::{
    selected_neighbor_from_backend_route_id, FieldPlannerModel, FieldPlannerSeed,
};
pub use recovery::{
    FieldBootstrapTransition, FieldPromotionBlocker, FieldPromotionDecision,
    FieldRouteRecoveryOutcome, FieldRouteRecoveryState, FieldRouteRecoveryTrigger,
};
pub use research::{
    CodingWindow, DiffusionFragmentId, DiffusionMessageId, DiffusionPressure, FragmentArrivalClass,
    FragmentCustody, ReceiverRankState, ReconstructionQuorum,
};
pub use route::FieldBootstrapClass;
pub use search::{
    FieldPlannerSearchRecord, FieldSearchConfig, FieldSearchConfigError, FieldSearchEdgeMeta,
    FieldSearchEpoch, FieldSearchHeuristicMode, FieldSearchPlanningFailure,
    FieldSearchReconfiguration, FieldSearchRun, FieldSearchSnapshotId, FieldSearchTransitionClass,
    FieldSelectedContinuation,
};
