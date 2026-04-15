//! Deterministic tuning experiment matrix for BATMAN, Pathway, and Field.
//!
//! The experiment subsystem is grouped by responsibility:
//! - `types` defines the run/config/result schema
//! - `suite` assembles run matrices and executes them
//! - `summary` reduces replays into stable JSON/report surfaces
//! - `common` owns shared topology/objective/environment helpers
//! - engine-family builder modules own concrete scenario families

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
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
use jacquard_olsrv2::{DecayWindow as OlsrV2DecayWindow, OLSRV2_ENGINE_ID};
use jacquard_pathway::{PathwaySearchConfig, PathwaySearchHeuristicMode, PATHWAY_ENGINE_ID};
use jacquard_reference_client::topology;
use jacquard_traits::{RoutingScenario, RoutingSimulator};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    environment::{EnvironmentHook, ScheduledEnvironmentHook, ScriptedEnvironmentModel},
    harness::{JacquardHostAdapter, JacquardSimulator, SimulationError},
    scenario::{BoundObjective, FieldBootstrapSummary, HostSpec, JacquardScenario},
    ReducedReplayView,
};

mod batman;
mod common;
mod comparison;
mod pathway_field;
mod suite;
mod summary;
mod types;

use batman::*;
use common::*;
use comparison::*;
use pathway_field::*;
use summary::*;
use types::*;

pub use suite::{local_suite, run_suite, smoke_suite};
pub use types::{
    ExperimentAggregateSummary, ExperimentArtifacts, ExperimentBreakdownSummary, ExperimentError,
    ExperimentManifest, ExperimentParameterSet, ExperimentRunSummary, ExperimentSuite,
    RegimeDescriptor,
};
