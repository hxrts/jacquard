//! Scenario presets for simulator smoke tests and examples.
//!
//! The public preset surface is grouped into:
//! - `basics`: line/ring/mixed baseline scenarios
//! - `regressions`: targeted scenario regressions
//! - `composition`: engine-composition scenarios
//! - `tuning`: small preset matrices used by tuning-oriented tests

#![allow(clippy::wildcard_imports)]

use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, ConnectivityPosture, DestinationId, DurationMs, Environment, FactSourceClass,
    Observation, OriginAuthenticationClass, PriorityPoints, RatioPermille, RouteEpoch,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
    RoutingEvidenceClass, RoutingObjective, ServiceId, Tick,
};
use jacquard_pathway::PathwaySearchConfig;

use crate::{
    environment::{EnvironmentHook, ScheduledEnvironmentHook, ScriptedEnvironmentModel},
    harness::default_objective,
    scenario::{BoundObjective, FieldBootstrapSummary, HostSpec, JacquardScenario},
    topology,
};

const NODE_A: jacquard_core::NodeId = jacquard_core::NodeId([1; 32]);
const NODE_B: jacquard_core::NodeId = jacquard_core::NodeId([2; 32]);
const NODE_C: jacquard_core::NodeId = jacquard_core::NodeId([3; 32]);
const NODE_D: jacquard_core::NodeId = jacquard_core::NodeId([4; 32]);

mod basics;
mod common;
mod composition;
mod regressions;
mod tuning;

use common::*;

pub use basics::{
    all_engines_line, all_engines_ring, babel_line, batman_classic_line, batman_line,
    field_bootstrap_multihop, field_line, mixed_line, olsrv2_line, pathway_line,
};
pub use composition::{
    composition_cascade_partition_eliminates_route, composition_concurrent_objectives,
    composition_corridor_preferred, composition_explicit_path_preferred,
    composition_next_hop_only_viable,
};
pub use regressions::{
    adversarial_relay_regression, churn_regression, deferred_delivery_regression,
    dense_saturation_regression, partition_regression,
};
pub use tuning::{
    batman_decay_tuning, olsrv2_decay_tuning, pathway_search_budget_tuning,
    profile_driven_engine_selection,
};
