//! Concrete reference-client bridge builders.
//!
//! This module provides factory functions that assemble a complete host-side
//! client from its constituent parts: a `MultiEngineRouter`, one or more
//! routing engines (`PathwayEngine`, `BatmanEngine`), an in-memory transport
//! driver, and queue-backed sender capabilities for each engine. The result is
//! a `HostBridge` that owns the transport attachment and drives the router
//! through synchronous rounds.
//!
//! `build_pathway_client` registers a single pathway engine. The
//! `_with_profile` variants accept an explicit `SelectedRoutingParameters` for
//! tests that need non-default routing profiles (e.g. relay nodes with
//! best-effort connectivity). `build_pathway_batman_client` registers both
//! pathway and batman, wiring each engine to its own independent outbound queue
//! over the shared transport driver.
//!
//! `PathwayRouter` and `PathwayClient` are type aliases exported for use by
//! integration tests that need to name the bridge type concretely.

use jacquard_batman::BatmanEngine;
use jacquard_core::{
    Configuration, ConnectivityPosture, DiversityFloor, DurationMs, HealthScore,
    IdentityAssuranceClass, LinkEndpoint, NodeId, Observation, OperatingMode,
    RatioPermille, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
    RouteReplacementPolicy, RoutingEngineFallbackPolicy, RoutingPolicyInputs,
    SelectedRoutingParameters, Tick,
};
use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport,
    SharedInMemoryNetwork,
};
use jacquard_pathway::{DeterministicPathwayTopologyModel, PathwayEngine};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::Blake3Hashing;

use crate::{bridge::BridgeTransport, HostBridge};

pub type PathwayRouter = MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects>;
pub type PathwayClient = HostBridge<PathwayRouter>;

const DEFAULT_OUTBOUND_QUEUE_CAPACITY: usize = 64;

pub fn build_pathway_client(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
) -> PathwayClient {
    build_pathway_client_with_profile(
        local_node_id,
        topology,
        network,
        now,
        default_profile(),
    )
}

pub fn build_pathway_client_with_profile(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
    profile: SelectedRoutingParameters,
) -> PathwayClient {
    let local_endpoint = local_endpoint(&topology, local_node_id);
    let driver = InMemoryTransport::attach(local_node_id, [local_endpoint], network);
    let transport = BridgeTransport::new(driver);
    let pathway_sender = transport.sender(DEFAULT_OUTBOUND_QUEUE_CAPACITY);

    let engine = PathwayEngine::without_committee_selector(
        local_node_id,
        DeterministicPathwayTopologyModel::new(),
        pathway_sender,
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects { now, ..Default::default() },
        Blake3Hashing,
    );
    let mut router = MultiEngineRouter::new(
        local_node_id,
        FixedPolicyEngine::new(profile),
        InMemoryRuntimeEffects { now, ..Default::default() },
        topology.clone(),
        policy_inputs_for(&topology, local_node_id),
    );
    router
        .register_engine(Box::new(engine))
        .expect("register pathway engine");
    HostBridge::from_transport(
        topology,
        router,
        transport,
        DEFAULT_OUTBOUND_QUEUE_CAPACITY,
    )
}

pub fn build_pathway_batman_client(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
) -> PathwayClient {
    build_pathway_batman_client_with_profile(
        local_node_id,
        topology,
        network,
        now,
        default_profile(),
    )
}

pub fn build_pathway_batman_client_with_profile(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
    profile: SelectedRoutingParameters,
) -> PathwayClient {
    let local_endpoint = local_endpoint(&topology, local_node_id);
    let driver = InMemoryTransport::attach(local_node_id, [local_endpoint], network);
    let transport = BridgeTransport::new(driver);
    let pathway_sender = transport.sender(DEFAULT_OUTBOUND_QUEUE_CAPACITY);
    let batman_sender = transport.sender(DEFAULT_OUTBOUND_QUEUE_CAPACITY);

    let pathway_engine = PathwayEngine::without_committee_selector(
        local_node_id,
        DeterministicPathwayTopologyModel::new(),
        pathway_sender,
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects { now, ..Default::default() },
        Blake3Hashing,
    );
    let batman_engine = BatmanEngine::new(
        local_node_id,
        batman_sender,
        InMemoryRuntimeEffects { now, ..Default::default() },
    );

    let mut router = MultiEngineRouter::new(
        local_node_id,
        FixedPolicyEngine::new(profile),
        InMemoryRuntimeEffects { now, ..Default::default() },
        topology.clone(),
        policy_inputs_for(&topology, local_node_id),
    );
    router
        .register_engine(Box::new(pathway_engine))
        .expect("register pathway engine");
    router
        .register_engine(Box::new(batman_engine))
        .expect("register batman engine");
    HostBridge::from_transport(
        topology,
        router,
        transport,
        DEFAULT_OUTBOUND_QUEUE_CAPACITY,
    )
}

fn local_endpoint(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
) -> LinkEndpoint {
    topology.value.nodes[&local_node_id]
        .profile
        .endpoints
        .first()
        .cloned()
        .expect("reference topology must provide at least one local endpoint")
}

pub(crate) fn policy_inputs_for(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
) -> RoutingPolicyInputs {
    RoutingPolicyInputs {
        local_node: Observation {
            value: topology.value.nodes[&local_node_id].clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        local_environment: Observation {
            value: topology.value.environment.clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        routing_engine_count: 1,
        median_rtt_ms: DurationMs(40),
        loss_permille: RatioPermille(50),
        partition_risk_permille: RatioPermille(150),
        adversary_pressure_permille: RatioPermille(25),
        identity_assurance: IdentityAssuranceClass::ControllerBound,
        direct_reachability_score: HealthScore(900),
    }
}

#[cfg(test)]
pub(crate) fn policy_inputs_for_empty(local_node_id: NodeId) -> RoutingPolicyInputs {
    let empty_node = jacquard_mem_node_profile::ReferenceNode::route_capable(
        local_node_id,
        jacquard_core::ControllerId(local_node_id.0),
        jacquard_core::LinkEndpoint::new(
            jacquard_core::TransportKind::Custom("reference".to_owned()),
            jacquard_core::EndpointLocator::Opaque(vec![0]),
            jacquard_core::ByteCount(64),
        ),
        &jacquard_pathway::PATHWAY_ENGINE_ID,
        Tick(1),
    )
    .build();
    let topology = Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(1),
            nodes: std::collections::BTreeMap::from([(local_node_id, empty_node)]),
            links: std::collections::BTreeMap::new(),
            environment: jacquard_core::Environment {
                reachable_neighbor_count: 0,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: jacquard_core::FactSourceClass::Local,
        evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    };
    policy_inputs_for(&topology, local_node_id)
}

pub(crate) fn default_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile: OperatingMode::FieldPartitionTolerant,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}
