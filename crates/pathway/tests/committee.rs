//! Integration tests for the deterministic pathway committee selector.
//!
//! Unit tests in `committee.rs` cover the four return-`None` guard
//! branches in isolation. This file exercises the public selector
//! through `CommitteeSelector::select_committee` against a configured
//! topology and confirms the result is deterministic across repeated
//! calls and non-empty under the standard pathway fixture.

mod common;

use std::collections::BTreeMap;

use common::{
    effects::{TestRetentionStore, TestRuntimeEffects, TestTransport},
    engine::{
        lease, materialization_input, objective, profile, profile_with_connectivity,
        LOCAL_NODE_ID,
    },
    fixtures::{link, node, sample_configuration},
};
use jacquard_pathway::{
    DeterministicCommitteeSelector, DeterministicPathwayTopologyModel, PathwayEngine,
    PathwayTopologyModel,
};
use jacquard_traits::{
    jacquard_core::{
        AdmissionDecision, Configuration, ControllerId, DestinationId,
        DiscoveryScopeId, Environment, Node, NodeId, Observation, RatioPermille,
        RouteAdmissionRejection, RouteEpoch, RouteError, RoutePartitionClass,
        RouteRepairClass, RouteRuntimeError, RoutingTickContext, ServiceId,
        ServiceScope, Tick,
    },
    Blake3Hashing, CommitteeSelector, RoutingEngine, RoutingEnginePlanner,
};

#[derive(Clone)]
struct PreferredCommitteeTopologyModel {
    base: jacquard_pathway::DeterministicPathwayTopologyModel,
    preferred_peer: NodeId,
}

impl PreferredCommitteeTopologyModel {
    fn new(preferred_peer: NodeId) -> Self {
        Self {
            base: jacquard_pathway::DeterministicPathwayTopologyModel::new(),
            preferred_peer,
        }
    }
}

impl PathwayTopologyModel for PreferredCommitteeTopologyModel {
    type NeighborhoodEstimate = jacquard_pathway::PathwayNeighborhoodEstimate;
    type PeerEstimate = jacquard_pathway::PathwayPeerEstimate;

    fn local_node(
        &self,
        local_node_id: &NodeId,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Option<Node> {
        self.base.local_node(local_node_id, configuration)
    }

    fn neighboring_nodes(
        &self,
        local_node_id: &NodeId,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Vec<(NodeId, Node)> {
        self.base.neighboring_nodes(local_node_id, configuration)
    }

    fn reachable_endpoints(
        &self,
        local_node_id: &NodeId,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Vec<jacquard_traits::jacquard_core::LinkEndpoint> {
        self.base.reachable_endpoints(local_node_id, configuration)
    }

    fn adjacent_links(
        &self,
        local_node_id: &NodeId,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Vec<jacquard_traits::jacquard_core::Link> {
        self.base.adjacent_links(local_node_id, configuration)
    }

    fn peer_estimate(
        &self,
        local_node_id: &NodeId,
        peer_node_id: &NodeId,
        observed_at_tick: jacquard_traits::jacquard_core::Tick,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Option<Self::PeerEstimate> {
        let mut estimate = self.base.peer_estimate(
            local_node_id,
            peer_node_id,
            observed_at_tick,
            configuration,
        )?;
        if *peer_node_id == self.preferred_peer {
            estimate.relay_value_score =
                Some(jacquard_traits::jacquard_core::HealthScore(1000));
            estimate.retention_value_score =
                Some(jacquard_traits::jacquard_core::HealthScore(1000));
        } else {
            estimate.relay_value_score =
                Some(jacquard_traits::jacquard_core::HealthScore(0));
            estimate.retention_value_score =
                Some(jacquard_traits::jacquard_core::HealthScore(0));
        }
        Some(estimate)
    }

    fn neighborhood_estimate(
        &self,
        local_node_id: &NodeId,
        observed_at_tick: jacquard_traits::jacquard_core::Tick,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Option<Self::NeighborhoodEstimate> {
        self.base
            .neighborhood_estimate(local_node_id, observed_at_tick, configuration)
    }
}

// Two calls to the selector on the same inputs must return the same
// `Option<CommitteeSelection>`. The standard sample fixture should
// produce a `Some` result so the determinism check is meaningful.
#[test]
fn committee_selection_is_optional_and_deterministic() {
    let selector = DeterministicCommitteeSelector::new(NodeId([1; 32]));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Service(ServiceId(vec![9, 9])));
    let policy = profile();

    let first = selector
        .select_committee(&goal, &policy, &topology)
        .expect("selector result");
    let second = selector
        .select_committee(&goal, &policy, &topology)
        .expect("selector result");

    assert_eq!(first, second);
    assert!(first.is_some());
}

#[test]
fn committee_selection_reads_through_topology_model_estimates() {
    let topology = sample_configuration();
    let goal = objective(DestinationId::Service(ServiceId(vec![9, 9])));
    let policy = profile();

    let selector_for_node_two = DeterministicCommitteeSelector::with_topology_model(
        NodeId([1; 32]),
        PreferredCommitteeTopologyModel::new(NodeId([2; 32])),
    );
    let selector_for_node_four = DeterministicCommitteeSelector::with_topology_model(
        NodeId([1; 32]),
        PreferredCommitteeTopologyModel::new(NodeId([4; 32])),
    );

    let committee_two = selector_for_node_two
        .select_committee(&goal, &policy, &topology)
        .expect("selector result")
        .expect("committee");
    let committee_four = selector_for_node_four
        .select_committee(&goal, &policy, &topology)
        .expect("selector result")
        .expect("committee");

    assert_ne!(
        committee_two.members[0].node_id,
        committee_four.members[0].node_id
    );
}

#[derive(Clone)]
struct ErroringCommitteeSelector;

impl CommitteeSelector for ErroringCommitteeSelector {
    type TopologyView = Configuration;

    fn select_committee(
        &self,
        _objective: &jacquard_traits::jacquard_core::RoutingObjective,
        _profile: &jacquard_traits::jacquard_core::SelectedRoutingParameters,
        _topology: &jacquard_traits::jacquard_core::Observation<Self::TopologyView>,
    ) -> Result<Option<jacquard_traits::jacquard_core::CommitteeSelection>, RouteError>
    {
        Err(RouteError::Runtime(RouteRuntimeError::Invalidated))
    }
}

type SelectorEngine<Selector> = PathwayEngine<
    DeterministicPathwayTopologyModel,
    TestTransport,
    TestRetentionStore,
    TestRuntimeEffects,
    Blake3Hashing,
    Selector,
>;

fn build_engine_with_selector<Selector>(
    selector: Selector,
) -> SelectorEngine<Selector> {
    PathwayEngine::with_committee_selector(
        LOCAL_NODE_ID,
        DeterministicPathwayTopologyModel::new(),
        TestTransport::default(),
        TestRetentionStore::default(),
        TestRuntimeEffects::with_now(Tick(2)),
        Blake3Hashing,
        selector,
    )
}

fn node_with_identity_and_scope(
    node_byte: u8,
    controller_id: ControllerId,
    discovery_scope: DiscoveryScopeId,
) -> Node {
    let mut node = node(node_byte);
    node.controller_id = controller_id;
    for service in &mut node.profile.services {
        service.controller_id = controller_id;
        service.scope = ServiceScope::Discovery(discovery_scope);
    }
    node
}

// long-block-exception: deterministic committee-diversity topology fixture.
fn diversity_topology() -> Observation<Configuration> {
    let node_two = NodeId([2; 32]);
    let node_four = NodeId([4; 32]);
    let node_five = NodeId([5; 32]);
    let node_six = NodeId([6; 32]);

    Observation {
        value: Configuration {
            epoch: RouteEpoch(2),
            nodes: BTreeMap::from([
                (LOCAL_NODE_ID, node(1)),
                (
                    node_two,
                    node_with_identity_and_scope(
                        2,
                        ControllerId([2; 32]),
                        DiscoveryScopeId([2; 16]),
                    ),
                ),
                (
                    node_four,
                    node_with_identity_and_scope(
                        4,
                        ControllerId([2; 32]),
                        DiscoveryScopeId([4; 16]),
                    ),
                ),
                (
                    node_five,
                    node_with_identity_and_scope(
                        5,
                        ControllerId([5; 32]),
                        DiscoveryScopeId([2; 16]),
                    ),
                ),
                (
                    node_six,
                    node_with_identity_and_scope(
                        6,
                        ControllerId([6; 32]),
                        DiscoveryScopeId([6; 16]),
                    ),
                ),
            ]),
            links: BTreeMap::from([
                ((LOCAL_NODE_ID, node_two), link(2, 980)),
                ((LOCAL_NODE_ID, node_four), link(4, 960)),
                ((LOCAL_NODE_ID, node_five), link(5, 940)),
                ((LOCAL_NODE_ID, node_six), link(6, 920)),
            ]),
            environment: Environment {
                reachable_neighbor_count: 4,
                churn_permille: RatioPermille(120),
                contention_permille: RatioPermille(110),
            },
        },
        source_class: jacquard_traits::jacquard_core::FactSourceClass::Local,
        evidence_class:
            jacquard_traits::jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication:
            jacquard_traits::jacquard_core::OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

#[test]
fn committee_selector_none_keeps_candidate_admissible() {
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([4; 32])));
    let policy = profile_with_connectivity(
        RouteRepairClass::BestEffort,
        RoutePartitionClass::ConnectedOnly,
    );
    let engine = build_engine_with_selector(jacquard_pathway::NoCommitteeSelector);

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let check = engine
        .check_candidate(&goal, &policy, &candidate, &topology)
        .expect("admission check");

    assert_eq!(check.decision, AdmissionDecision::Admissible);
}

#[test]
fn committee_selector_some_is_carried_into_active_route() {
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();
    let mut engine =
        build_engine_with_selector(DeterministicCommitteeSelector::new(LOCAL_NODE_ID));

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("route admission");
    let input = materialization_input(route_id, admission, lease(Tick(2), Tick(12)));
    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("prime pathway topology");
    engine
        .materialize_route(input)
        .expect("materialize route with committee");

    assert!(
        engine
            .active_route(&route_id)
            .expect("active route")
            .has_committee
    );
}

#[test]
fn committee_selector_errors_surface_as_backend_unavailable() {
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([4; 32])));
    let policy = profile_with_connectivity(
        RouteRepairClass::BestEffort,
        RoutePartitionClass::ConnectedOnly,
    );
    let engine = build_engine_with_selector(ErroringCommitteeSelector);

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let check = engine
        .check_candidate(&goal, &policy, &candidate, &topology)
        .expect("admission check");

    assert_eq!(
        check.decision,
        AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable),
    );
    let admission = engine.admit_route(&goal, &policy, candidate, &topology);
    assert!(matches!(
        admission,
        Err(RouteError::Selection(
            jacquard_traits::jacquard_core::RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            ),
        ))
    ));
}

#[test]
fn committee_selector_failure_survives_cache_eviction_via_plan_token() {
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([4; 32])));
    let policy = profile_with_connectivity(
        RouteRepairClass::BestEffort,
        RoutePartitionClass::ConnectedOnly,
    );
    let mut engine = build_engine_with_selector(ErroringCommitteeSelector);

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("evict planner cache");
    let check = engine
        .check_candidate(&goal, &policy, &candidate, &topology)
        .expect("cache-miss admission check");

    assert_eq!(
        check.decision,
        AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable),
    );
}

#[test]
fn committee_selection_enforces_controller_and_discovery_scope_diversity() {
    let selector = DeterministicCommitteeSelector::new(LOCAL_NODE_ID);
    let topology = diversity_topology();
    let goal = objective(DestinationId::Node(NodeId([6; 32])));
    let policy = profile();

    let committee = selector
        .select_committee(&goal, &policy, &topology)
        .expect("selector result")
        .expect("committee");
    let member_ids = committee
        .members
        .iter()
        .map(|member| member.node_id)
        .collect::<Vec<_>>();

    assert!(member_ids.contains(&NodeId([2; 32])));
    assert!(member_ids.contains(&NodeId([6; 32])));
    assert!(!member_ids.contains(&NodeId([4; 32])));
    assert!(!member_ids.contains(&NodeId([5; 32])));
}
