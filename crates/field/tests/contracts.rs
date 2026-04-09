//! Framework routing trait contract tests for the field engine.
//!
//! These tests verify that `FieldEngine` correctly implements the shared
//! `RoutingEngine` and `RoutingEnginePlanner` trait surfaces. A minimal
//! two-node topology with no links is used so that `candidate_routes` returns
//! empty, exercising the planner's admission path without requiring a live
//! attractor. `field_engine_advertises_corridor_envelope_visibility` confirms
//! the declared engine ID and route shape visibility. `field_engine_compiles_
//! against_shared_routing_traits` confirms the tick and planner APIs accept
//! the standard context and objective shapes without panicking.

use jacquard_adapter::opaque_endpoint;
use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId,
    DurationMs, Environment, FactSourceClass, Observation, OriginAuthenticationClass,
    RatioPermille, RouteEpoch, RoutePartitionClass, RouteProtectionClass,
    RouteRepairClass, RouteShapeVisibility, RoutingEvidenceClass, RoutingObjective,
    RoutingTickContext, SelectedRoutingParameters, Tick, TransportKind,
};
use jacquard_field::{FieldEngine, FIELD_ENGINE_ID};
use jacquard_mem_link_profile::{InMemoryRuntimeEffects, InMemoryTransport};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_traits::{RoutingEngine, RoutingEnginePlanner};

fn node(byte: u8) -> jacquard_core::NodeId {
    jacquard_core::NodeId([byte; 32])
}

fn endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
    opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(128))
}

fn topology() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: std::collections::BTreeMap::from([
                (
                    node(1),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(1), ControllerId([1; 32])),
                            endpoint(1),
                            Tick(1),
                        ),
                        &FIELD_ENGINE_ID,
                    )
                    .build(),
                ),
                (
                    node(2),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(2), ControllerId([2; 32])),
                            endpoint(2),
                            Tick(1),
                        ),
                        &FIELD_ENGINE_ID,
                    )
                    .build(),
                ),
            ]),
            links: std::collections::BTreeMap::new(),
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

fn objective() -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(node(2)),
        service_kind: jacquard_core::RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: jacquard_core::PriorityPoints(10),
        connectivity_priority: jacquard_core::PriorityPoints(20),
    }
}

fn profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile: jacquard_core::OperatingMode::FieldPartitionTolerant,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy:
            jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}

#[test]
fn field_engine_advertises_corridor_envelope_visibility() {
    let engine = FieldEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects { now: Tick(1), ..Default::default() },
    );

    assert_eq!(engine.engine_id(), FIELD_ENGINE_ID);
    assert_eq!(
        engine.capabilities().route_shape_visibility,
        RouteShapeVisibility::CorridorEnvelope
    );
}

#[test]
fn field_engine_compiles_against_shared_routing_traits() {
    let mut engine = FieldEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects { now: Tick(1), ..Default::default() },
    );
    let topology = topology();

    let outcome = engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("default tick shape");
    assert_eq!(outcome.topology_epoch, topology.value.epoch);
    assert!(engine
        .candidate_routes(&objective(), &profile(), &topology)
        .is_empty());
}
