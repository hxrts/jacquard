//! Deterministic tuning experiment matrix for BATMAN, Pathway, and Field.
//!
//! The experiment subsystem is grouped by responsibility:
//! - `types` defines the run/config/result schema
//! - `suite` assembles run matrices
//! - `runner` executes suites and writes artifacts
//! - `summary` reduces replays into stable JSON/report surfaces
//! - `common` owns shared topology/objective/environment helpers
//! - engine-family builder modules own concrete scenario families

#![allow(clippy::wildcard_imports)]

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use jacquard_babel::BABEL_ENGINE_ID;
use jacquard_batman_bellman::{DecayWindow, BATMAN_BELLMAN_ENGINE_ID};
use jacquard_batman_classic::BATMAN_CLASSIC_ENGINE_ID;
use jacquard_core::{
    Belief, Configuration, ConnectivityPosture, DestinationId, DurationMs, Environment,
    FactSourceClass, Node, NodeId, Observation, OriginAuthenticationClass, PriorityPoints,
    RatioPermille, RouteEpoch, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
    RouteServiceKind, RoutingEvidenceClass, RoutingObjective, SelectedRoutingParameters,
    SimulationSeed, Tick,
};
use jacquard_field::{
    FieldForwardSummaryObservation, FieldSearchConfig, FieldSearchHeuristicMode, FIELD_ENGINE_ID,
};
#[cfg(test)]
use jacquard_mercator::MERCATOR_ENGINE_ID;
use jacquard_olsrv2::{DecayWindow as OlsrV2DecayWindow, OLSRV2_ENGINE_ID};
use jacquard_pathway::{PathwaySearchConfig, PathwaySearchHeuristicMode, PATHWAY_ENGINE_ID};
use jacquard_scatter::SCATTER_ENGINE_ID;
use jacquard_traits::RoutingScenario;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    environment::{EnvironmentHook, ScheduledEnvironmentHook, ScriptedEnvironmentModel},
    harness::{JacquardHostAdapter, JacquardSimulator, SimulationError},
    scenario::{
        BoundObjective, FieldBootstrapSummary, HostSpec, HostTopologyLag, JacquardScenario,
    },
    topology, ReducedReplayView,
};

mod batman;
mod catalog;
mod common;
mod comparison;
mod model_support;
mod pathway_field;
mod runner;
mod suite;
mod summary;
mod templates;
mod types;

use batman::*;
use common::*;
use comparison::*;
use pathway_field::*;
use summary::*;
use templates::*;
use types::*;

pub use runner::run_suite;
pub use suite::{
    babel_equivalence_smoke_suite, babel_model_smoke_suite, batman_bellman_model_smoke_suite,
    batman_classic_model_smoke_suite, field_model_smoke_suite, local_stage_suite,
    local_stage_suite_with_seeds, local_stage_suite_with_seeds_and_config, local_suite,
    olsrv2_model_smoke_suite, pathway_model_smoke_suite, scatter_model_smoke_suite, smoke_suite,
};
pub use summary::{aggregate_runs, summarize_breakdowns};
pub use types::{
    ExperimentAggregateSummary, ExperimentArtifacts, ExperimentBreakdownSummary, ExperimentError,
    ExperimentManifest, ExperimentModelArtifact, ExperimentParameterSet, ExperimentRunSummary,
    ExperimentSuite, RegimeDescriptor, ROUTE_VISIBLE_ARTIFACT_SCHEMA_VERSION,
};
