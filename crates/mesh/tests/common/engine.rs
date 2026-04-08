//! Engine builders and high-level helpers for the mesh integration tests.
//!
//! `TestEngine` is the concrete `MeshEngine` instantiation used by every
//! integration test, with the test runtime effects, transport, and
//! retention store from `super::effects`. The functions in this module
//! either build a fresh engine, build a routing objective or profile,
//! or compose admission and materialization into a single high-level
//! step that returns the materialized identity and runtime ready for
//! maintenance probes.

use std::ops::{Deref, DerefMut};

use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine};
use jacquard_traits::{
    jacquard_core::{
        Configuration, ConnectivityPosture, DestinationId, DiversityFloor, DurationMs,
        HoldFallbackPolicy, Limit, MaterializedRouteIdentity, NodeId, Observation,
        OperatingMode, PriorityPoints, PublicationId, RouteAdmission, RouteCandidate,
        RouteHandle, RouteIdentityStamp, RouteLease, RouteMaterializationInput,
        RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
        RouteReplacementPolicy, RouteRuntimeState, RouteServiceKind,
        RoutingEngineFallbackPolicy, RoutingObjective, RoutingTickContext,
        SelectedRoutingParameters, Tick, TimeWindow,
    },
    Blake3Hashing, RoutingEngine, RoutingEnginePlanner,
};

use super::{
    effects::{TestRetentionStore, TestRuntimeEffects, TestTransport},
    fixtures::sample_configuration,
};

type RawTestEngine = MeshEngine<
    DeterministicMeshTopologyModel,
    TestTransport,
    TestRetentionStore,
    TestRuntimeEffects,
    Blake3Hashing,
>;

/// Concrete mesh test harness used by every integration test.
pub struct TestEngine {
    engine: RawTestEngine,
    pub transport: TestTransport,
    pub retention: TestRetentionStore,
    pub effects: TestRuntimeEffects,
}

impl Deref for TestEngine {
    type Target = RawTestEngine;

    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}

impl DerefMut for TestEngine {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.engine
    }
}

/// Local node id used by every test engine. Centralised so the lease
/// owner and the engine identity stay in sync.
pub const LOCAL_NODE_ID: NodeId = NodeId([1; 32]);

pub fn build_engine() -> TestEngine {
    build_engine_at_tick(Tick(2))
}

pub fn build_engine_at_tick(now: Tick) -> TestEngine {
    build_engine_for_node_at_tick(LOCAL_NODE_ID, now)
}

pub fn build_engine_for_node_at_tick(local_node_id: NodeId, now: Tick) -> TestEngine {
    let transport = TestTransport::default();
    let retention = TestRetentionStore::default();
    let effects = TestRuntimeEffects::default();
    effects.set_now(now);
    let engine = MeshEngine::without_committee_selector(
        local_node_id,
        DeterministicMeshTopologyModel::new(),
        transport.clone(),
        retention.clone(),
        effects.clone(),
        Blake3Hashing,
    );
    TestEngine { engine, transport, retention, effects }
}

pub fn mesh_connectivity(partition: RoutePartitionClass) -> ConnectivityPosture {
    ConnectivityPosture {
        repair: RouteRepairClass::Repairable,
        partition,
    }
}

pub fn objective(destination: DestinationId) -> RoutingObjective {
    objective_with_floor(
        destination,
        RouteProtectionClass::LinkProtected,
        RouteProtectionClass::LinkProtected,
    )
}

pub fn objective_with_floor(
    destination: DestinationId,
    target: RouteProtectionClass,
    floor: RouteProtectionClass,
) -> RoutingObjective {
    RoutingObjective {
        destination,
        service_kind: RouteServiceKind::Move,
        target_protection: target,
        protection_floor: floor,
        target_connectivity: mesh_connectivity(RoutePartitionClass::PartitionTolerant),
        hold_fallback_policy: HoldFallbackPolicy::Allowed,
        latency_budget_ms: Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

pub fn profile() -> SelectedRoutingParameters {
    profile_with_connectivity(
        RouteRepairClass::Repairable,
        RoutePartitionClass::PartitionTolerant,
    )
}

pub fn profile_with_connectivity(
    repair: RouteRepairClass,
    partition: RoutePartitionClass,
) -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture { repair, partition },
        deployment_profile: OperatingMode::FieldPartitionTolerant,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

/// Build a route lease owned by the test local node, valid over the
/// supplied tick window. The lease epoch is set to the topology epoch
/// of the standard sample configuration.
pub fn lease(start: Tick, end: Tick) -> RouteLease {
    RouteLease {
        owner_node_id: LOCAL_NODE_ID,
        lease_epoch: sample_configuration().value.epoch,
        valid_for: TimeWindow::new(start, end).expect("valid lease window"),
    }
}

pub fn tick_context(topology: &Observation<Configuration>) -> RoutingTickContext {
    RoutingTickContext::new(topology.clone())
}

/// Assemble a `RouteMaterializationInput` from an admission and a lease,
/// using a deterministic publication id and the lease start tick as the
/// materialization tick.
pub fn materialization_input(
    admission: jacquard_traits::jacquard_core::RouteAdmission,
    lease_value: RouteLease,
) -> RouteMaterializationInput {
    let materialized_at_tick = lease_value.valid_for.start_tick();
    RouteMaterializationInput {
        handle: RouteHandle {
            stamp: RouteIdentityStamp {
                route_id: admission.route_id,
                topology_epoch: lease_value.lease_epoch,
                materialized_at_tick,
                publication_id: PublicationId([7; 16]),
            },
        },
        admission,
        lease: lease_value,
    }
}

/// Step 1 of the activate pipeline: tick the engine and collect candidates
/// for the given objective/profile against the supplied topology.
pub fn tick_and_get_candidates(
    engine: &mut TestEngine,
    topology: &Observation<Configuration>,
    goal: &RoutingObjective,
    policy: &SelectedRoutingParameters,
) -> Vec<RouteCandidate> {
    engine.engine_tick(&tick_context(topology)).expect("engine tick");
    engine.candidate_routes(goal, policy, topology)
}

/// Step 2 of the activate pipeline: admit the first candidate from the
/// supplied list. Panics if the list is empty or admission fails.
pub fn admit_first_candidate(
    engine: &mut TestEngine,
    topology: &Observation<Configuration>,
    goal: &RoutingObjective,
    policy: &SelectedRoutingParameters,
    candidates: Vec<RouteCandidate>,
) -> RouteAdmission {
    let candidate = candidates
        .into_iter()
        .next()
        .expect("admit_first_candidate requires at least one candidate");
    engine
        .admit_route(goal, policy, candidate, topology)
        .expect("admit_first_candidate admission")
}

/// Step 3 of the activate pipeline: materialize an admitted route and
/// assemble the canonical `(MaterializedRouteIdentity, RouteRuntimeState)`
/// pair that the engine expects on `maintain_route` calls.
pub fn materialize_admitted(
    engine: &mut TestEngine,
    admission: RouteAdmission,
    lease_value: RouteLease,
) -> (MaterializedRouteIdentity, RouteRuntimeState) {
    let materialization_tick = lease_value.valid_for.start_tick();
    let input = materialization_input(admission, lease_value);
    let installation = engine
        .materialize_route(input.clone())
        .expect("materialize_admitted materialization");

    let runtime = RouteRuntimeState {
        last_lifecycle_event: installation.last_lifecycle_event,
        health: installation.health,
        progress: installation.progress,
    };
    let identity = MaterializedRouteIdentity {
        stamp: input.handle.stamp.clone(),
        proof: installation.materialization_proof,
        admission: input.admission,
        lease: input.lease,
    };
    debug_assert_eq!(
        materialization_tick, identity.stamp.materialized_at_tick,
        "materialization_input should use the lease start tick",
    );
    (identity, runtime)
}

/// Drive a route from candidate production to a fully materialized
/// runtime. Returns the canonical identity and runtime state the engine
/// expects on `maintain_route` calls. The objective always uses the
/// supplied destination with the standard mesh objective and profile.
///
/// This is the canonical "set up an active route to probe" recipe and
/// every test that needs an active route should use it instead of
/// duplicating the candidate -> admit -> materialize -> assemble chain.
pub fn activate_route(
    engine: &mut TestEngine,
    topology: &Observation<Configuration>,
    destination: NodeId,
    lease_value: RouteLease,
) -> (MaterializedRouteIdentity, RouteRuntimeState) {
    let goal = objective(DestinationId::Node(destination));
    let policy = profile();
    activate_route_with_profile(engine, topology, &goal, &policy, lease_value)
}

pub fn activate_route_with_profile(
    engine: &mut TestEngine,
    topology: &Observation<Configuration>,
    goal: &RoutingObjective,
    policy: &SelectedRoutingParameters,
    lease_value: RouteLease,
) -> (MaterializedRouteIdentity, RouteRuntimeState) {
    let candidates = tick_and_get_candidates(engine, topology, goal, policy);
    let admission = admit_first_candidate(engine, topology, goal, policy, candidates);
    materialize_admitted(engine, admission, lease_value)
}
