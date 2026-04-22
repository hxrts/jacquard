use std::collections::BTreeMap;

use jacquard_cast_support::{
    shape_broadcast_evidence, shape_multicast_evidence, shape_unicast_evidence, BroadcastEvidence,
    BroadcastObservation, BroadcastReverseConfirmation, CastEvidenceMeta, CastEvidencePolicy,
    CastGroupId, MulticastEvidence, MulticastObservation, ReceiverCoverageObservation,
    UnicastEvidence, UnicastObservation,
};
use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    Environment, FactSourceClass, Limit, Link, Node, NodeId, Observation, OperatingMode,
    OrderStamp, OriginAuthenticationClass, PriorityPoints, RatioPermille, RouteEpoch,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
    RoutingEvidenceClass, RoutingObjective, SelectedRoutingParameters, Tick, TransportKind,
};
use jacquard_host_support::opaque_endpoint;
use jacquard_mem_link_profile::{LinkPreset, LinkPresetOptions};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_mercator::MERCATOR_ENGINE_ID;
use jacquard_reference_client::{ClientBuilder, SharedInMemoryNetwork};
use jacquard_traits::Router;

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
    opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(512))
}

fn mercator_node(byte: u8) -> Node {
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

fn link(to: NodeId, confidence: RatioPermille) -> Link {
    LinkPreset::lossy(
        LinkPresetOptions::new(endpoint(to.0[0]), Tick(1)).with_confidence(confidence),
    )
    .build()
}

fn meta(order: u64) -> CastEvidenceMeta {
    CastEvidenceMeta::new(
        Tick(1),
        DurationMs(10),
        DurationMs(1_000),
        OrderStamp(order),
    )
}

#[derive(Clone, Debug)]
struct FixtureTopology {
    nodes: Vec<NodeId>,
    links: Vec<(NodeId, NodeId, RatioPermille)>,
}

impl FixtureTopology {
    fn observation(&self) -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(5),
                nodes: self
                    .nodes
                    .iter()
                    .map(|node_id| (*node_id, mercator_node(node_id.0[0])))
                    .collect(),
                links: self
                    .links
                    .iter()
                    .map(|(from, to, confidence)| ((*from, *to), link(*to, *confidence)))
                    .collect(),
                environment: Environment {
                    reachable_neighbor_count: u32::try_from(self.links.len()).unwrap_or(u32::MAX),
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
}

#[derive(Clone, Debug)]
struct UnicastFixtureProfile {
    observations: Vec<UnicastObservation>,
}

impl UnicastFixtureProfile {
    fn shape(&self) -> Vec<UnicastEvidence> {
        shape_unicast_evidence(self.observations.clone(), CastEvidencePolicy::default()).0
    }

    fn topology(&self) -> FixtureTopology {
        FixtureTopology {
            nodes: vec![node(1), node(2)],
            links: self
                .shape()
                .into_iter()
                .map(|evidence| {
                    (
                        evidence.from,
                        evidence.to,
                        evidence.directional_confidence_permille,
                    )
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
struct MulticastFixtureProfile {
    observations: Vec<MulticastObservation>,
}

impl MulticastFixtureProfile {
    fn shape(&self) -> Vec<MulticastEvidence> {
        shape_multicast_evidence(self.observations.clone(), CastEvidencePolicy::default()).0
    }
}

#[derive(Clone, Debug)]
struct BroadcastFixtureProfile {
    observations: Vec<BroadcastObservation>,
}

impl BroadcastFixtureProfile {
    fn shape(&self) -> Vec<BroadcastEvidence> {
        shape_broadcast_evidence(self.observations.clone(), CastEvidencePolicy::default()).0
    }
}

fn unicast_fixture() -> UnicastFixtureProfile {
    UnicastFixtureProfile {
        observations: vec![UnicastObservation {
            from: node(1),
            to: node(2),
            directional_confidence_permille: RatioPermille(850),
            reverse_confirmation_permille: Some(RatioPermille(800)),
            payload_bytes_max: ByteCount(512),
            meta: meta(1),
        }],
    }
}

fn multicast_fixture() -> MulticastFixtureProfile {
    MulticastFixtureProfile {
        observations: vec![MulticastObservation {
            sender: node(1),
            group_id: CastGroupId(b"team".to_vec()),
            receivers: vec![
                ReceiverCoverageObservation {
                    receiver: node(2),
                    confidence_permille: RatioPermille(800),
                },
                ReceiverCoverageObservation {
                    receiver: node(3),
                    confidence_permille: RatioPermille(700),
                },
            ],
            group_pressure_permille: RatioPermille(100),
            fanout_limit: 2,
            payload_bytes_max: ByteCount(512),
            meta: meta(2),
        }],
    }
}

fn broadcast_fixture() -> BroadcastFixtureProfile {
    BroadcastFixtureProfile {
        observations: vec![BroadcastObservation {
            sender: node(1),
            receivers: vec![ReceiverCoverageObservation {
                receiver: node(4),
                confidence_permille: RatioPermille(750),
            }],
            reverse_confirmation: BroadcastReverseConfirmation::Unavailable,
            transmission_window_quality_permille: RatioPermille(800),
            channel_pressure_permille: RatioPermille(100),
            copy_budget: 1,
            payload_bytes_max: ByteCount(512),
            meta: meta(3),
        }],
    }
}

fn objective(destination: NodeId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(destination),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::None,
        protection_floor: RouteProtectionClass::None,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Forbidden,
        latency_budget_ms: Limit::Bounded(DurationMs(100)),
        protection_priority: PriorityPoints(1),
        connectivity_priority: PriorityPoints(1),
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

#[test]
fn cast_fixture_profiles_shape_unicast_multicast_and_broadcast_outputs() {
    assert_eq!(unicast_fixture().shape().len(), 1);
    assert_eq!(multicast_fixture().shape()[0].covered_receiver_count, 2);
    assert_eq!(
        broadcast_fixture().shape()[0].connected_bidirectional_confidence(),
        RatioPermille(0)
    );
}

#[test]
fn cast_unicast_fixture_feeds_reference_bridge_and_mercator_router() {
    let topology = unicast_fixture().topology().observation();
    let mut client =
        ClientBuilder::mercator(node(1), topology, SharedInMemoryNetwork::default(), Tick(1))
            .with_profile(profile())
            .build()
            .expect("build mercator client");
    let mut bound = client.bind();

    let route = Router::activate_route(bound.router_mut(), objective(node(2)))
        .expect("activate helper-shaped route");
    bound.advance_round().expect("advance bridge-owned round");

    assert_eq!(route.identity.admission.summary.engine, MERCATOR_ENGINE_ID);
    assert_eq!(route.identity.topology_epoch(), RouteEpoch(5));
}

#[test]
fn cast_fixture_outputs_are_deterministic_across_repeated_runs() {
    let mut links = BTreeMap::new();
    for _ in 0..3 {
        let topology = unicast_fixture().topology();
        links.insert(links.len(), topology.links);
    }

    assert_eq!(links[&0], links[&1]);
    assert_eq!(links[&1], links[&2]);
}
