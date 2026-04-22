use std::collections::BTreeMap;

use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    Environment, FactSourceClass, Limit, NodeId, Observation, OperatingMode,
    OriginAuthenticationClass, RatioPermille, RouteEpoch, RoutePartitionClass,
    RouteProtectionClass, RouteRepairClass, RouteSelectionError, RouteServiceKind, RouteSummary,
    RoutingEvidenceClass, RoutingObjective, RoutingTickContext, SelectedRoutingParameters, Tick,
    TimeWindow, TransportKind,
};
use jacquard_host_support::opaque_endpoint;
use jacquard_mem_link_profile::{LinkPreset, LinkPresetOptions};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_mercator::{MercatorEngine, MERCATOR_CAPABILITIES, MERCATOR_ENGINE_ID};
use jacquard_traits::{RouterManagedEngine, RoutingEngine, RoutingEnginePlanner};

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
    opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(256))
}

fn mercator_node(byte: u8) -> jacquard_core::Node {
    NodePreset::route_capable(
        NodePresetOptions::new(
            NodeIdentity::new(node(byte), ControllerId([byte; 32])),
            endpoint(byte),
            Tick(1),
        ),
        &MERCATOR_ENGINE_ID,
    )
    .build()
}

fn connected_fixture() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([(node(1), mercator_node(1)), (node(2), mercator_node(2))]),
            links: BTreeMap::from([
                (
                    (node(1), node(2)),
                    LinkPreset::active(LinkPresetOptions::new(endpoint(2), Tick(1))).build(),
                ),
                (
                    (node(2), node(1)),
                    LinkPreset::active(LinkPresetOptions::new(endpoint(1), Tick(1))).build(),
                ),
            ]),
            environment: Environment {
                reachable_neighbor_count: 1,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    }
}

fn disconnected_fixture() -> Observation<Configuration> {
    let mut topology = connected_fixture();
    topology.value.links.clear();
    topology.value.environment.reachable_neighbor_count = 0;
    topology
}

fn objective() -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(node(2)),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::None,
        protection_floor: RouteProtectionClass::None,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Forbidden,
        latency_budget_ms: Limit::Bounded(DurationMs(100)),
        protection_priority: jacquard_core::PriorityPoints(1),
        connectivity_priority: jacquard_core::PriorityPoints(1),
    }
}

fn profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::None,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: OperatingMode::SparseLowPower,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}

fn assert_trait_surface<T: RoutingEnginePlanner + RoutingEngine + RouterManagedEngine>() {}

#[test]
fn mercator_implements_router_managed_engine_surface() {
    assert_trait_surface::<MercatorEngine>();
}

#[test]
fn mercator_phase_zero_is_inert_on_connected_and_disconnected_fixtures() {
    let engine = MercatorEngine::new(node(1));
    assert_eq!(engine.engine_id(), MERCATOR_ENGINE_ID);
    assert_eq!(engine.capabilities(), MERCATOR_CAPABILITIES);
    assert_eq!(
        engine
            .candidate_routes(&objective(), &profile(), &connected_fixture())
            .len(),
        1
    );
    assert!(engine
        .candidate_routes(&objective(), &profile(), &disconnected_fixture())
        .is_empty());
}

#[test]
fn mercator_phase_zero_records_topology_epoch_without_route_change() {
    let mut engine = MercatorEngine::new(node(1));
    let topology = connected_fixture();

    let outcome = engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("engine tick");

    assert_eq!(engine.latest_topology_epoch(), Some(topology.value.epoch));
    assert_eq!(outcome.topology_epoch, topology.value.epoch);
    assert_eq!(outcome.change, jacquard_core::RoutingTickChange::NoChange);
}

#[test]
fn mercator_phase_zero_rejects_admission_when_given_any_candidate() {
    let engine = MercatorEngine::new(node(1));
    let topology = connected_fixture();
    let foreign_candidate = jacquard_core::RouteCandidate {
        route_id: jacquard_core::RouteId([9; 16]),
        backend_ref: jacquard_core::BackendRouteRef {
            engine: MERCATOR_ENGINE_ID,
            backend_route_id: jacquard_core::BackendRouteId(vec![9]),
        },
        summary: RouteSummary {
            engine: MERCATOR_ENGINE_ID,
            protection: RouteProtectionClass::None,
            connectivity: ConnectivityPosture {
                repair: RouteRepairClass::BestEffort,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            protocol_mix: vec![TransportKind::WifiAware],
            hop_count_hint: jacquard_core::Belief::certain(1, Tick(1)),
            valid_for: TimeWindow::new(Tick(1), Tick(2)).expect("valid window"),
        },
        estimate: jacquard_core::Estimate::certain(
            jacquard_core::RouteEstimate {
                estimated_protection: RouteProtectionClass::None,
                estimated_connectivity: ConnectivityPosture {
                    repair: RouteRepairClass::BestEffort,
                    partition: RoutePartitionClass::ConnectedOnly,
                },
                topology_epoch: topology.value.epoch,
                degradation: jacquard_core::RouteDegradation::None,
            },
            Tick(1),
        ),
    };

    let error = engine
        .check_candidate(&objective(), &profile(), &foreign_candidate, &topology)
        .expect_err("phase zero rejects candidates");

    assert_eq!(
        error,
        RouteSelectionError::Inadmissible(
            jacquard_core::RouteAdmissionRejection::BackendUnavailable
        )
        .into()
    );
}
