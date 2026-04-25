//! Builders for pre-wired `MultiEngineRouter` instances used across router
//! integration tests.
//!
//! Each builder function composes a `MultiEngineRouter<FixedPolicyEngine,
//! InMemoryRuntimeEffects>` from the shared fixture topology, a policy engine,
//! and one registered routing engine appropriate for the test scenario:
//!
//! - `build_router`: standard pathway-only router at a given `Tick`.
//! - `build_router_with_effects`: same as above but with caller-supplied router
//!   runtime effects, used for fail-closed injection tests.
//! - `build_router_with_selector`: pathway engine wired with an
//!   `AdvisoryCommitteeSelector`, used for committee selection tests.
//! - `build_router_with_recoverable_engine`: registers a
//!   `RecoverableTestEngine` backed by a shared `BTreeSet`, used for recovery
//!   and checkpoint tests.
//! - `build_router_with_proactive_engine`: registers a
//!   `ProactiveTableTestEngine` with a caller-specified `RouteShapeVisibility`,
//!   used for proactive routing tests.
//! - `build_router_with_opaque_engine`: registers one opaque external-engine
//!   double used to prove the router can host a route source it cannot
//!   introspect.
//! - `build_router_with_pathway_and_batman`: registers the real in-tree
//!   `pathway` and `batman` engines in one router over a dual-engine topology.

use std::sync::{Arc, Mutex};

use jacquard_batman_bellman::BatmanBellmanEngine;
use jacquard_core::{RoutePartitionClass, Tick};
use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport, SharedInMemoryNetwork,
};
use jacquard_pathway::{DeterministicPathwayTopologyModel, PathwayEngine};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_testkit::topology;
use jacquard_traits::Blake3Hashing;

use super::{
    committee_selector::AdvisoryCommitteeSelector,
    fixtures::{profile, sample_configuration, sample_policy_inputs, LOCAL_NODE_ID},
    opaque_engine::OpaqueSummaryTestEngine,
    proactive_engine::ProactiveTableTestEngine,
    recoverable_engine::RecoverableTestEngine,
};

pub(crate) type TestPathwayEngine = PathwayEngine<
    DeterministicPathwayTopologyModel,
    InMemoryTransport,
    InMemoryRetentionStore,
    InMemoryRuntimeEffects,
    Blake3Hashing,
>;

pub(crate) type CommitteePathwayEngine = PathwayEngine<
    DeterministicPathwayTopologyModel,
    InMemoryTransport,
    InMemoryRetentionStore,
    InMemoryRuntimeEffects,
    Blake3Hashing,
    AdvisoryCommitteeSelector,
>;

pub(crate) fn build_router(
    now: Tick,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    build_router_with_effects(
        now,
        InMemoryRuntimeEffects {
            now,
            ..Default::default()
        },
    )
}

pub(crate) fn build_router_with_selector(
    now: Tick,
    selector: AdvisoryCommitteeSelector,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let engine: CommitteePathwayEngine = PathwayEngine::with_committee_selector(
        LOCAL_NODE_ID,
        DeterministicPathwayTopologyModel::new(),
        InMemoryTransport::new(),
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects {
            now,
            ..Default::default()
        },
        Blake3Hashing,
        selector,
    );
    let policy_engine = FixedPolicyEngine::new(profile());
    let router_effects = InMemoryRuntimeEffects {
        now,
        ..Default::default()
    };

    let mut router = MultiEngineRouter::new(
        LOCAL_NODE_ID,
        policy_engine,
        router_effects,
        topology,
        policy_inputs,
    );
    router
        .register_engine(Box::new(engine))
        .expect("register committee pathway engine");
    router
}

pub(crate) fn build_router_with_effects(
    now: Tick,
    router_effects: InMemoryRuntimeEffects,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    build_router_with_runtime_pair(
        now,
        router_effects,
        InMemoryRuntimeEffects {
            now,
            ..Default::default()
        },
    )
}

pub(crate) fn build_router_with_runtime_pair(
    _now: Tick,
    router_effects: InMemoryRuntimeEffects,
    engine_effects: InMemoryRuntimeEffects,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let engine: TestPathwayEngine = PathwayEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicPathwayTopologyModel::new(),
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
        .expect("register pathway engine");
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
        InMemoryRuntimeEffects {
            now,
            ..Default::default()
        },
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

pub(crate) fn build_router_with_opaque_engine(
    now: Tick,
    engine_id: jacquard_core::RoutingEngineId,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let policy_engine = FixedPolicyEngine::new(profile());
    let mut router = MultiEngineRouter::new(
        LOCAL_NODE_ID,
        policy_engine,
        InMemoryRuntimeEffects {
            now,
            ..Default::default()
        },
        topology,
        policy_inputs,
    );
    router
        .register_engine(Box::new(OpaqueSummaryTestEngine::new(
            LOCAL_NODE_ID,
            engine_id,
            now,
        )))
        .expect("register opaque external engine");
    router
}

// long-block-exception: this test builder keeps mixed-engine router setup in
// one place so integration fixtures stay explicit and deterministic.
pub(crate) fn build_router_with_pathway_and_batman(
    now: Tick,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = jacquard_core::Observation {
        value: jacquard_core::Configuration {
            epoch: jacquard_core::RouteEpoch(2),
            nodes: std::collections::BTreeMap::from([
                (
                    LOCAL_NODE_ID,
                    topology::node(1).pathway_and_batman_bellman().build(),
                ),
                (
                    super::fixtures::PEER_NODE_ID,
                    topology::node(2).pathway_and_batman_bellman().build(),
                ),
                (
                    super::fixtures::FAR_NODE_ID,
                    topology::node(3)
                        .for_engine(&jacquard_batman_bellman::BATMAN_BELLMAN_ENGINE_ID)
                        .build(),
                ),
                (
                    super::fixtures::BRIDGE_NODE_ID,
                    topology::node(4).pathway().build(),
                ),
            ]),
            links: std::collections::BTreeMap::from([
                (
                    (LOCAL_NODE_ID, super::fixtures::PEER_NODE_ID),
                    topology::link(2).build(),
                ),
                (
                    (super::fixtures::PEER_NODE_ID, LOCAL_NODE_ID),
                    topology::link(1).build(),
                ),
                (
                    (super::fixtures::PEER_NODE_ID, super::fixtures::FAR_NODE_ID),
                    topology::link(3).build(),
                ),
                (
                    (super::fixtures::FAR_NODE_ID, super::fixtures::PEER_NODE_ID),
                    topology::link(2).build(),
                ),
                (
                    (
                        super::fixtures::PEER_NODE_ID,
                        super::fixtures::BRIDGE_NODE_ID,
                    ),
                    topology::link(4).build(),
                ),
            ]),
            environment: jacquard_core::Environment {
                reachable_neighbor_count: 3,
                churn_permille: jacquard_core::RatioPermille(25),
                contention_permille: jacquard_core::RatioPermille(20),
            },
        },
        source_class: jacquard_core::FactSourceClass::Local,
        evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
        observed_at_tick: now,
    };
    let policy_inputs = sample_policy_inputs(&topology);
    let mut mixed_profile = profile();
    mixed_profile.selected_connectivity.partition = RoutePartitionClass::ConnectedOnly;
    let policy_engine = FixedPolicyEngine::new(mixed_profile);
    let network = SharedInMemoryNetwork::default();
    let endpoints = topology.value.nodes[&LOCAL_NODE_ID]
        .profile
        .endpoints
        .clone();
    let pathway_transport =
        InMemoryTransport::attach(LOCAL_NODE_ID, endpoints.clone(), network.clone());
    let batman_transport = InMemoryTransport::attach(LOCAL_NODE_ID, endpoints, network);
    let pathway_engine: TestPathwayEngine = PathwayEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicPathwayTopologyModel::new(),
        pathway_transport,
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects {
            now,
            ..Default::default()
        },
        Blake3Hashing,
    );
    let batman_engine = BatmanBellmanEngine::new(
        LOCAL_NODE_ID,
        batman_transport,
        InMemoryRuntimeEffects {
            now,
            ..Default::default()
        },
    );

    let mut router = MultiEngineRouter::new(
        LOCAL_NODE_ID,
        policy_engine,
        InMemoryRuntimeEffects {
            now,
            ..Default::default()
        },
        topology,
        policy_inputs,
    );
    router
        .register_engine(Box::new(pathway_engine))
        .expect("register pathway engine");
    router
        .register_engine(Box::new(batman_engine))
        .expect("register batman engine");
    router
}
