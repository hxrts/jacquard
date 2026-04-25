//! Public simulator facade for downstream experiment crates.
// proc-macro-scope: simulator facade types are external API schema and intentionally stay outside shared model macros.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use jacquard_babel::BABEL_ENGINE_ID;
use jacquard_batman_bellman::BATMAN_BELLMAN_ENGINE_ID;
use jacquard_batman_classic::BATMAN_CLASSIC_ENGINE_ID;
use jacquard_core::{RoutingEngineId, SimulationSeed};
use jacquard_field::FIELD_ENGINE_ID;
use jacquard_mercator::MERCATOR_ENGINE_ID;
use jacquard_olsrv2::OLSRV2_ENGINE_ID;
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_scatter::SCATTER_ENGINE_ID;
use jacquard_traits::RoutingScenario;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    diffusion::{self, DiffusionArtifacts, DiffusionSuite},
    environment::ScriptedEnvironmentModel,
    experiments::{self, ExperimentArtifacts, ExperimentSuite},
    harness::{JacquardHostAdapter, JacquardSimulator, ReferenceClientAdapter, SimulationError},
    scenario::{EngineLane, JacquardScenario},
};

pub const EXTERNAL_ROUTE_ARTIFACT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimulatorConfig {
    pub capture_level: crate::SimulationCaptureLevel,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            capture_level: crate::SimulationCaptureLevel::ReducedReplay,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtifactSink {
    Directory(PathBuf),
    Disabled,
}

impl ArtifactSink {
    #[must_use]
    pub fn directory(path: impl Into<PathBuf>) -> Self {
        Self::Directory(path.into())
    }

    #[must_use]
    pub const fn disabled() -> Self {
        Self::Disabled
    }

    fn output_dir(&self) -> Option<&Path> {
        match self {
            Self::Directory(path) => Some(path.as_path()),
            Self::Disabled => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EngineRouteShape {
    RouteVisible,
    Diffusion,
    ModelOnly,
    EnginePrivateDiagnostics,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EngineRegistryEntry {
    pub engine_id: String,
    pub route_shape: EngineRouteShape,
    pub report_label: String,
    pub enabled_by_default: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EngineRegistry {
    entries: BTreeMap<String, EngineRegistryEntry>,
}

impl EngineRegistry {
    #[must_use]
    pub fn route_visible_defaults() -> Self {
        let mut registry = Self::default();
        for (engine_id, label) in [
            ("batman-classic", "BATMAN Classic"),
            ("batman-bellman", "BATMAN Bellman"),
            ("babel", "Babel"),
            ("olsrv2", "OLSRv2"),
            ("scatter", "Scatter"),
            ("mercator", "Mercator"),
            ("pathway", "Pathway"),
            ("pathway-batman-bellman", "Pathway + BATMAN Bellman"),
        ] {
            registry.insert(EngineRegistryEntry {
                engine_id: engine_id.to_string(),
                route_shape: EngineRouteShape::RouteVisible,
                report_label: label.to_string(),
                enabled_by_default: true,
            });
        }
        registry
    }

    pub fn insert(&mut self, entry: EngineRegistryEntry) {
        self.entries.insert(entry.engine_id.clone(), entry);
    }

    #[must_use]
    pub fn get(&self, engine_id: &str) -> Option<&EngineRegistryEntry> {
        self.entries.get(engine_id)
    }

    pub fn engine_ids(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    fn require_route_visible(
        &self,
        suite_id: &str,
        run: &RouteVisibleRunSpec,
    ) -> Result<(), ExternalExperimentError> {
        let Some(entry) = self.get(&run.engine_family) else {
            return Err(ExternalExperimentError::UnknownEngine {
                suite_id: suite_id.to_string(),
                family_id: run.family_id.clone(),
                config_id: run.run_id.clone(),
                engine_id: run.engine_family.clone(),
            });
        };
        if entry.route_shape != EngineRouteShape::RouteVisible {
            return Err(ExternalExperimentError::UnsupportedEngineShape {
                suite_id: suite_id.to_string(),
                family_id: run.family_id.clone(),
                config_id: run.run_id.clone(),
                engine_id: run.engine_family.clone(),
                route_shape: entry.route_shape,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct RouteVisibleRunSpec {
    pub run_id: String,
    pub family_id: String,
    pub engine_family: String,
    pub seed: SimulationSeed,
    pub scenario: JacquardScenario,
    pub environment: ScriptedEnvironmentModel,
}

impl RouteVisibleRunSpec {
    #[must_use]
    pub fn new(
        run_id: impl Into<String>,
        family_id: impl Into<String>,
        engine_family: impl Into<String>,
        seed: SimulationSeed,
        scenario: JacquardScenario,
        environment: ScriptedEnvironmentModel,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            family_id: family_id.into(),
            engine_family: engine_family.into(),
            seed,
            scenario,
            environment,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ExperimentSuiteSpec {
    pub suite_id: String,
    pub route_visible_runs: Vec<RouteVisibleRunSpec>,
}

impl ExperimentSuiteSpec {
    #[must_use]
    pub fn route_visible(
        suite_id: impl Into<String>,
        route_visible_runs: Vec<RouteVisibleRunSpec>,
    ) -> Self {
        Self {
            suite_id: suite_id.into(),
            route_visible_runs,
        }
    }

    #[must_use]
    pub fn run_count(&self) -> usize {
        self.route_visible_runs.len()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalExperimentManifest {
    pub schema_version: u32,
    pub suite_id: String,
    pub artifact_kind: String,
    pub generated_at_unix_seconds: u64,
    pub run_count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RouteVisibleRunSummary {
    pub schema_version: u32,
    pub suite_id: String,
    pub run_id: String,
    pub family_id: String,
    pub scenario_name: String,
    pub engine_family: String,
    pub seed: u64,
    pub round_count: u32,
    pub distinct_engine_ids: Vec<String>,
    pub active_route_round_count: u32,
    pub driver_status_event_count: u32,
    pub failure_summary_count: u32,
}

#[derive(Clone, Debug)]
pub struct RouteVisibleArtifacts {
    pub output_dir: Option<PathBuf>,
    pub manifest: ExternalExperimentManifest,
    pub runs: Vec<RouteVisibleRunSummary>,
}

#[derive(Clone, Debug)]
pub struct ExperimentRunner {
    config: SimulatorConfig,
    engine_registry: EngineRegistry,
}

impl Default for ExperimentRunner {
    fn default() -> Self {
        Self::new(SimulatorConfig::default())
    }
}

impl ExperimentRunner {
    #[must_use]
    pub fn new(config: SimulatorConfig) -> Self {
        Self {
            config,
            engine_registry: EngineRegistry::route_visible_defaults(),
        }
    }

    #[must_use]
    pub fn with_engine_registry(mut self, engine_registry: EngineRegistry) -> Self {
        self.engine_registry = engine_registry;
        self
    }

    #[must_use]
    pub const fn config(&self) -> &SimulatorConfig {
        &self.config
    }

    #[must_use]
    pub const fn engine_registry(&self) -> &EngineRegistry {
        &self.engine_registry
    }

    pub fn validate_route_visible_suite(
        &self,
        suite: &ExperimentSuiteSpec,
    ) -> Result<(), ExternalExperimentError> {
        validate_id("suite", &suite.suite_id)?;
        if suite.route_visible_runs.is_empty() {
            return Err(ExternalExperimentError::EmptySuite {
                suite_id: suite.suite_id.clone(),
            });
        }
        let mut run_ids = BTreeSet::<String>::new();
        let mut tuples = BTreeSet::<(String, String, u64)>::new();
        for run in &suite.route_visible_runs {
            validate_id("run", &run.run_id)?;
            validate_id("family", &run.family_id)?;
            validate_id("engine", &run.engine_family)?;
            if !run_ids.insert(run.run_id.clone()) {
                return Err(ExternalExperimentError::DuplicateRunId {
                    suite_id: suite.suite_id.clone(),
                    run_id: run.run_id.clone(),
                });
            }
            let tuple = (run.family_id.clone(), run.engine_family.clone(), run.seed.0);
            if !tuples.insert(tuple.clone()) {
                return Err(ExternalExperimentError::DuplicateRunTuple {
                    suite_id: suite.suite_id.clone(),
                    family_id: tuple.0,
                    config_id: tuple.1,
                    seed: tuple.2,
                });
            }
            self.engine_registry
                .require_route_visible(&suite.suite_id, run)?;
            validate_host_lanes(&suite.suite_id, run, &self.engine_registry)?;
        }
        Ok(())
    }

    pub fn run_route_visible_suite(
        &self,
        suite: &ExperimentSuiteSpec,
        sink: &ArtifactSink,
    ) -> Result<RouteVisibleArtifacts, ExternalExperimentError> {
        self.run_route_visible_suite_with_adapter(&ReferenceClientAdapter, suite, sink)
    }

    // long-block-exception: custom-suite execution keeps validation, adapter setup, and artifact writing together as the facade boundary.
    pub fn run_route_visible_suite_with_adapter<A>(
        &self,
        adapter: &A,
        suite: &ExperimentSuiteSpec,
        sink: &ArtifactSink,
    ) -> Result<RouteVisibleArtifacts, ExternalExperimentError>
    where
        A: JacquardHostAdapter + Clone,
    {
        self.validate_route_visible_suite(suite)?;
        let mut runs = Vec::with_capacity(suite.route_visible_runs.len());
        for run in &suite.route_visible_runs {
            let simulator = JacquardSimulator::new((*adapter).clone());
            let (reduced, _) = simulator
                .run_scenario_reduced(&run.scenario, &run.environment)
                .map_err(|source| ExternalExperimentError::SimulationRun {
                    run_id: run.run_id.clone(),
                    source,
                })?;
            runs.push(RouteVisibleRunSummary {
                schema_version: EXTERNAL_ROUTE_ARTIFACT_SCHEMA_VERSION,
                suite_id: suite.suite_id.clone(),
                run_id: run.run_id.clone(),
                family_id: run.family_id.clone(),
                scenario_name: run.scenario.name().to_string(),
                engine_family: run.engine_family.clone(),
                seed: run.seed.0,
                round_count: reduced.round_count,
                active_route_round_count: u32::try_from(
                    reduced
                        .rounds
                        .iter()
                        .filter(|round| !round.active_routes.is_empty())
                        .count(),
                )
                .unwrap_or(u32::MAX),
                distinct_engine_ids: reduced
                    .distinct_engine_ids
                    .iter()
                    .map(engine_id_label)
                    .collect(),
                driver_status_event_count: u32::try_from(reduced.driver_status_events.len())
                    .unwrap_or(u32::MAX),
                failure_summary_count: u32::try_from(reduced.failure_summaries.len())
                    .unwrap_or(u32::MAX),
            });
        }
        let manifest = ExternalExperimentManifest {
            schema_version: EXTERNAL_ROUTE_ARTIFACT_SCHEMA_VERSION,
            suite_id: suite.suite_id.clone(),
            artifact_kind: "route-visible".to_string(),
            generated_at_unix_seconds: 0,
            run_count: u32::try_from(runs.len()).unwrap_or(u32::MAX),
        };
        let output_dir = sink.output_dir().map(Path::to_path_buf);
        if let Some(output_dir) = sink.output_dir() {
            write_route_visible_artifacts(output_dir, &manifest, &runs)?;
        }
        Ok(RouteVisibleArtifacts {
            output_dir,
            manifest,
            runs,
        })
    }

    pub fn run_tuning_suite<A>(
        &self,
        simulator: &mut JacquardSimulator<A>,
        suite: &ExperimentSuite,
        sink: &ArtifactSink,
    ) -> Result<ExperimentArtifacts, ExternalExperimentError>
    where
        A: JacquardHostAdapter + Clone + Send + Sync,
    {
        let output_dir =
            sink.output_dir()
                .ok_or(ExternalExperimentError::ArtifactSinkDisabled {
                    suite_id: suite.suite_id().to_string(),
                    artifact_kind: "tuning".to_string(),
                })?;
        experiments::run_suite(simulator, suite, output_dir).map_err(ExternalExperimentError::from)
    }

    pub fn run_diffusion_suite(
        &self,
        suite: &DiffusionSuite,
        sink: &ArtifactSink,
    ) -> Result<DiffusionArtifacts, ExternalExperimentError> {
        let output_dir =
            sink.output_dir()
                .ok_or(ExternalExperimentError::ArtifactSinkDisabled {
                    suite_id: suite.suite_id().to_string(),
                    artifact_kind: "diffusion".to_string(),
                })?;
        diffusion::run_diffusion_suite(suite, output_dir).map_err(ExternalExperimentError::from)
    }
}

#[derive(Debug, Error)]
pub enum ExternalExperimentError {
    #[error("{kind} id must not be empty")]
    EmptyId { kind: &'static str },
    #[error("{kind} id '{id}' contains unsupported characters")]
    InvalidId { kind: &'static str, id: String },
    #[error("suite '{suite_id}' has no route-visible runs")]
    EmptySuite { suite_id: String },
    #[error("duplicate run id '{run_id}' in suite '{suite_id}'")]
    DuplicateRunId { suite_id: String, run_id: String },
    #[error(
        "duplicate run tuple in suite '{suite_id}' for family '{family_id}', config '{config_id}', seed {seed}"
    )]
    DuplicateRunTuple {
        suite_id: String,
        family_id: String,
        config_id: String,
        seed: u64,
    },
    #[error(
        "unknown engine '{engine_id}' in suite '{suite_id}', family '{family_id}', config '{config_id}'"
    )]
    UnknownEngine {
        suite_id: String,
        family_id: String,
        config_id: String,
        engine_id: String,
    },
    #[error(
        "engine '{engine_id}' in suite '{suite_id}', family '{family_id}', config '{config_id}' has route shape {route_shape:?}"
    )]
    UnsupportedEngineShape {
        suite_id: String,
        family_id: String,
        config_id: String,
        engine_id: String,
        route_shape: EngineRouteShape,
    },
    #[error(
        "scenario host lane {lane:?} in suite '{suite_id}', run '{run_id}' requires missing registry engine '{engine_id}'"
    )]
    MissingHostLaneEngine {
        suite_id: String,
        run_id: String,
        lane: EngineLane,
        engine_id: String,
    },
    #[error("simulation failed for {run_id}: {source}")]
    SimulationRun {
        run_id: String,
        #[source]
        source: SimulationError,
    },
    #[error("artifact sink is disabled for {artifact_kind} suite '{suite_id}'")]
    ArtifactSinkDisabled {
        suite_id: String,
        artifact_kind: String,
    },
    #[error("io failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("json failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("experiment failed: {0}")]
    Experiment(#[from] experiments::ExperimentError),
}

fn write_route_visible_artifacts(
    output_dir: &Path,
    manifest: &ExternalExperimentManifest,
    runs: &[RouteVisibleRunSummary],
) -> Result<(), ExternalExperimentError> {
    fs::create_dir_all(output_dir)?;
    serde_json::to_writer_pretty(
        File::create(output_dir.join("external_manifest.json"))?,
        manifest,
    )?;
    let mut writer = BufWriter::new(File::create(output_dir.join("external_runs.jsonl"))?);
    for run in runs {
        serde_json::to_writer(&mut writer, run)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

fn validate_id(kind: &'static str, id: &str) -> Result<(), ExternalExperimentError> {
    if id.is_empty() {
        return Err(ExternalExperimentError::EmptyId { kind });
    }
    if id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Ok(());
    }
    Err(ExternalExperimentError::InvalidId {
        kind,
        id: id.to_string(),
    })
}

fn validate_host_lanes(
    suite_id: &str,
    run: &RouteVisibleRunSpec,
    registry: &EngineRegistry,
) -> Result<(), ExternalExperimentError> {
    for host in run.scenario.hosts() {
        for engine_id in engine_ids_for_lane(&host.lane) {
            if registry.get(engine_id).is_none() {
                return Err(ExternalExperimentError::MissingHostLaneEngine {
                    suite_id: suite_id.to_string(),
                    run_id: run.run_id.clone(),
                    lane: host.lane.clone(),
                    engine_id: engine_id.to_string(),
                });
            }
        }
    }
    Ok(())
}

fn engine_ids_for_lane(lane: &EngineLane) -> &'static [&'static str] {
    match lane {
        EngineLane::Pathway => &["pathway"],
        EngineLane::BatmanBellman => &["batman-bellman"],
        EngineLane::BatmanClassic => &["batman-classic"],
        EngineLane::Babel => &["babel"],
        EngineLane::OlsrV2 => &["olsrv2"],
        EngineLane::Scatter => &["scatter"],
        EngineLane::Mercator => &["mercator"],
        EngineLane::PathwayAndBatmanBellman => &["pathway", "batman-bellman"],
        EngineLane::PathwayAndBabel => &["pathway", "babel"],
        EngineLane::PathwayAndOlsrV2 => &["pathway", "olsrv2"],
        EngineLane::BabelAndBatmanBellman => &["babel", "batman-bellman"],
        EngineLane::OlsrV2AndBatmanBellman => &["olsrv2", "batman-bellman"],
        EngineLane::RouteVisibleEngines => &[
            "batman-classic",
            "batman-bellman",
            "babel",
            "olsrv2",
            "scatter",
            "mercator",
            "pathway",
        ],
        EngineLane::AllEngines
        | EngineLane::Field
        | EngineLane::PathwayAndField
        | EngineLane::FieldAndBatmanBellman => &["field"],
    }
}

fn engine_id_label(engine_id: &RoutingEngineId) -> String {
    if *engine_id == BATMAN_CLASSIC_ENGINE_ID {
        "batman-classic"
    } else if *engine_id == BATMAN_BELLMAN_ENGINE_ID {
        "batman-bellman"
    } else if *engine_id == BABEL_ENGINE_ID {
        "babel"
    } else if *engine_id == OLSRV2_ENGINE_ID {
        "olsrv2"
    } else if *engine_id == SCATTER_ENGINE_ID {
        "scatter"
    } else if *engine_id == MERCATOR_ENGINE_ID {
        "mercator"
    } else if *engine_id == PATHWAY_ENGINE_ID {
        "pathway"
    } else if *engine_id == FIELD_ENGINE_ID {
        "field"
    } else {
        return format!("{:?}", engine_id.contract_id);
    }
    .to_string()
}
