//! Builders for pre-wired `MultiEngineRouter` instances used across router
//! integration tests.

use std::sync::{Arc, Mutex};

use jacquard_core::Tick;
use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport,
};
use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::Blake3Hashing;

use super::{
    committee_selector::AdvisoryCommitteeSelector,
    fixtures::{profile, sample_configuration, sample_policy_inputs, LOCAL_NODE_ID},
    proactive_engine::ProactiveTableTestEngine,
    recoverable_engine::RecoverableTestEngine,
};

pub(crate) type TestMeshEngine = MeshEngine<
    DeterministicMeshTopologyModel,
    InMemoryTransport,
    InMemoryRetentionStore,
    InMemoryRuntimeEffects,
    Blake3Hashing,
>;

pub(crate) type CommitteeMeshEngine = MeshEngine<
    DeterministicMeshTopologyModel,
    InMemoryTransport,
    InMemoryRetentionStore,
    InMemoryRuntimeEffects,
    Blake3Hashing,
    AdvisoryCommitteeSelector,
>;

pub(crate) fn build_router(
    now: Tick,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    build_router_with_effects(now, InMemoryRuntimeEffects { now, ..Default::default() })
}

pub(crate) fn build_router_with_selector(
    now: Tick,
    selector: AdvisoryCommitteeSelector,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let engine: CommitteeMeshEngine = MeshEngine::with_committee_selector(
        LOCAL_NODE_ID,
        DeterministicMeshTopologyModel::new(),
        InMemoryTransport::new(),
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects { now, ..Default::default() },
        Blake3Hashing,
        selector,
    );
    let policy_engine = FixedPolicyEngine::new(profile());
    let router_effects = InMemoryRuntimeEffects { now, ..Default::default() };

    let mut router = MultiEngineRouter::new(
        LOCAL_NODE_ID,
        policy_engine,
        router_effects,
        topology,
        policy_inputs,
    );
    router
        .register_engine(Box::new(engine))
        .expect("register committee mesh engine");
    router
}

pub(crate) fn build_router_with_effects(
    now: Tick,
    router_effects: InMemoryRuntimeEffects,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    build_router_with_runtime_pair(
        now,
        router_effects,
        InMemoryRuntimeEffects { now, ..Default::default() },
    )
}

pub(crate) fn build_router_with_runtime_pair(
    _now: Tick,
    router_effects: InMemoryRuntimeEffects,
    engine_effects: InMemoryRuntimeEffects,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let engine: TestMeshEngine = MeshEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicMeshTopologyModel::new(),
        InMemoryTransport::new(),
        InMemoryRetentionStore::default(),
        engine_effects,
        Blake3Hashing,
    );
    let policy_engine = FixedPolicyEngine::new(profile());

    let mut router = MultiEngineRouter::new(
        LOCAL_NODE_ID,
        policy_engine,
        router_effects,
        topology,
        policy_inputs,
    );
    router
        .register_engine(Box::new(engine))
        .expect("register mesh engine");
    router
}

pub(crate) fn build_router_with_recoverable_engine(
    now: Tick,
    router_effects: InMemoryRuntimeEffects,
    shared_state: Arc<Mutex<std::collections::BTreeSet<jacquard_core::RouteId>>>,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let policy_engine = FixedPolicyEngine::new(profile());
    let mut router = MultiEngineRouter::new(
        LOCAL_NODE_ID,
        policy_engine,
        router_effects,
        topology,
        policy_inputs,
    );
    router
        .register_engine(Box::new(RecoverableTestEngine::new(
            LOCAL_NODE_ID,
            shared_state,
            now,
        )))
        .expect("register recoverable test engine");
    router
}

pub(crate) fn build_router_with_proactive_engine(
    now: Tick,
    engine_id: jacquard_core::RoutingEngineId,
    visibility: jacquard_core::RouteShapeVisibility,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let policy_engine = FixedPolicyEngine::new(profile());
    let mut router = MultiEngineRouter::new(
        LOCAL_NODE_ID,
        policy_engine,
        InMemoryRuntimeEffects { now, ..Default::default() },
        topology,
        policy_inputs,
    );
    router
        .register_engine(Box::new(ProactiveTableTestEngine::new(
            LOCAL_NODE_ID,
            engine_id,
            visibility,
            now,
        )))
        .expect("register proactive test engine");
    router
}
