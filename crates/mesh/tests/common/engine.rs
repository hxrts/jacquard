//! Engine builders and high-level helpers for the mesh integration tests.
//!
//! `TestEngine` is the concrete `MeshEngine` instantiation used by every
//! integration test, with the test runtime effects, transport, and
//! retention store from `super::effects`. The functions in this module
//! either build a fresh engine, build a routing objective or profile,
//! or compose admission and materialization into a single high-level
//! step that returns the materialized identity and runtime ready for
//! maintenance probes.

use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine};
use jacquard_traits::{
    jacquard_core::{
        AdaptiveRoutingProfile, Configuration, DeploymentProfile, DestinationId, DurationMs,
        HoldFallbackPolicy, Limit, MaterializedRouteIdentity, NodeId, Observation, PriorityPoints,
        PublicationId, RouteConnectivityProfile, RouteHandle, RouteLease,
        RouteMaterializationInput, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
        RouteReplacementPolicy, RouteRuntimeState, RouteServiceKind, RoutingEngineFallbackPolicy,
        RoutingObjective, Tick, TimeWindow,
    },
    Blake3Hashing, RoutingEngine, RoutingEnginePlanner,
};

use super::effects::{TestRetentionStore, TestRuntimeEffects, TestTransport};

/// Concrete `MeshEngine` instantiation used by every integration test.
pub type TestEngine = MeshEngine<
    DeterministicMeshTopologyModel,
    TestTransport,
    TestRetentionStore,
    TestRuntimeEffects,
    Blake3Hashing,
>;

/// Local node id used by every test engine. Centralised so the lease
/// owner and the engine identity stay in sync.
pub const LOCAL_NODE_ID: NodeId = NodeId([1; 32]);

pub fn build_engine() -> TestEngine {
    build_engine_at_tick(Tick(2))
}

pub fn build_engine_at_tick(now: Tick) -> TestEngine {
    MeshEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicMeshTopologyModel::new(),
        TestTransport::default(),
        TestRetentionStore::default(),
        TestRuntimeEffects {
            now,
            ..Default::default()
        },
        Blake3Hashing,
    )
}

pub fn mesh_connectivity(partition: RoutePartitionClass) -> RouteConnectivityProfile {
    RouteConnectivityProfile {
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

pub fn profile() -> AdaptiveRoutingProfile {
    profile_with_connectivity(
        RouteRepairClass::Repairable,
        RoutePartitionClass::PartitionTolerant,
    )
}

pub fn profile_with_connectivity(
    repair: RouteRepairClass,
    partition: RoutePartitionClass,
) -> AdaptiveRoutingProfile {
    AdaptiveRoutingProfile {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: RouteConnectivityProfile { repair, partition },
        deployment_profile: DeploymentProfile::FieldPartitionTolerant,
        diversity_floor: 1,
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
        lease_epoch: jacquard_traits::jacquard_core::RouteEpoch(2),
        valid_for: TimeWindow::new(start, end).expect("valid lease window"),
    }
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
            route_id: admission.route_id,
            topology_epoch: lease_value.lease_epoch,
            materialized_at_tick,
            publication_id: PublicationId([7; 16]),
        },
        admission,
        lease: lease_value,
    }
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

    engine.engine_tick(topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, topology)
        .into_iter()
        .next()
        .expect("activate_route requires at least one candidate");
    let admission = engine
        .admit_route(&goal, &policy, candidate, topology)
        .expect("activate_route admission");
    let input = materialization_input(admission, lease_value);
    let installation = engine
        .materialize_route(input.clone())
        .expect("activate_route materialization");

    let runtime = RouteRuntimeState {
        last_lifecycle_event: installation.last_lifecycle_event,
        health: installation.health,
        progress: installation.progress,
    };
    let identity = MaterializedRouteIdentity {
        handle: input.handle,
        materialization_proof: installation.materialization_proof,
        admission: input.admission,
        lease: input.lease,
    };
    (identity, runtime)
}
