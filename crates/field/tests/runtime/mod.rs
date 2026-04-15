use std::collections::BTreeMap;

use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, Environment,
    FactSourceClass, LinkEndpoint, Observation, OriginAuthenticationClass, PublicationId,
    RatioPermille, RouteEpoch, RouteHandle, RouteLease, RoutePartitionClass, RouteProtectionClass,
    RouteRepairClass, RouteSelectionError, RouteServiceKind, RoutingEvidenceClass,
    RoutingObjective, SelectedRoutingParameters, ServiceId, Tick, TimeWindow, TransportError,
};
use jacquard_mem_link_profile::InMemoryTransport;
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_traits::{
    effect_handler, RouterManagedEngine, RoutingEngine, RoutingEnginePlanner,
    TransportSenderEffects,
};

use super::*;
use crate::state::{
    DestinationInterestClass, HopBand, NeighborContinuation, SupportBucket, MAX_ACTIVE_DESTINATIONS,
};
use crate::summary::{
    EvidenceContributionClass, FieldSummary, SummaryDestinationKey, SummaryUncertaintyClass,
};

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NoopTransport;

#[effect_handler]
impl TransportSenderEffects for NoopTransport {
    fn send_transport(
        &mut self,
        _endpoint: &LinkEndpoint,
        _payload: &[u8],
    ) -> Result<(), TransportError> {
        Ok(())
    }
}

fn sample_objective(destination: NodeId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(destination),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(jacquard_core::DurationMs(100)),
        protection_priority: jacquard_core::PriorityPoints(10),
        connectivity_priority: jacquard_core::PriorityPoints(10),
    }
}

fn sample_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: jacquard_core::OperatingMode::SparseLowPower,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}

fn supported_topology() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(4),
            nodes: BTreeMap::from([
                (
                    node(1),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(1), ControllerId([1; 32])),
                            jacquard_adapter::opaque_endpoint(
                                jacquard_core::TransportKind::WifiAware,
                                vec![1],
                                ByteCount(128),
                            ),
                            Tick(1),
                        ),
                        &crate::FIELD_ENGINE_ID,
                    )
                    .build(),
                ),
                (
                    node(2),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(2), ControllerId([2; 32])),
                            jacquard_adapter::opaque_endpoint(
                                jacquard_core::TransportKind::WifiAware,
                                vec![2],
                                ByteCount(128),
                            ),
                            Tick(1),
                        ),
                        &crate::FIELD_ENGINE_ID,
                    )
                    .build(),
                ),
            ]),
            links: BTreeMap::new(),
            environment: Environment {
                reachable_neighbor_count: 1,
                churn_permille: RatioPermille(100),
                contention_permille: RatioPermille(100),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(4),
    }
}

fn seeded_engine() -> FieldEngine<NoopTransport, ()> {
    let mut engine = FieldEngine::new(node(1), NoopTransport, ());
    let state = engine.state.upsert_destination_interest(
        &DestinationId::Node(node(2)),
        DestinationInterestClass::Transit,
        Tick(4),
    );
    state.posterior.top_corridor_mass = SupportBucket::new(850);
    state.posterior.usability_entropy = crate::state::EntropyBucket::new(200);
    state.posterior.predicted_observation_class = crate::state::ObservationClass::DirectOnly;
    state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
    state.corridor_belief.delivery_support = SupportBucket::new(800);
    state.corridor_belief.retention_affinity = SupportBucket::new(300);
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: node(2),
        net_value: SupportBucket::new(900),
        downstream_support: SupportBucket::new(850),
        expected_hop_band: HopBand::new(1, 2),
        freshness: Tick(4),
    });
    engine.state.neighbor_endpoints.insert(
        node(2),
        jacquard_adapter::opaque_endpoint(
            jacquard_core::TransportKind::WifiAware,
            vec![2],
            ByteCount(128),
        ),
    );
    engine
}

fn seeded_transport_engine() -> FieldEngine<InMemoryTransport, ()> {
    let mut engine = FieldEngine::new(node(1), InMemoryTransport::default(), ());
    let state = engine.state.upsert_destination_interest(
        &DestinationId::Node(node(2)),
        DestinationInterestClass::Transit,
        Tick(4),
    );
    state.posterior.top_corridor_mass = SupportBucket::new(850);
    state.posterior.usability_entropy = crate::state::EntropyBucket::new(200);
    state.posterior.predicted_observation_class = crate::state::ObservationClass::DirectOnly;
    state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
    state.corridor_belief.delivery_support = SupportBucket::new(800);
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: node(2),
        net_value: SupportBucket::new(900),
        downstream_support: SupportBucket::new(850),
        expected_hop_band: HopBand::new(1, 2),
        freshness: Tick(4),
    });
    engine.state.neighbor_endpoints.insert(
        node(2),
        jacquard_adapter::opaque_endpoint(
            jacquard_core::TransportKind::WifiAware,
            vec![2],
            ByteCount(128),
        ),
    );
    engine
}

fn lease() -> RouteLease {
    RouteLease {
        owner_node_id: node(1),
        lease_epoch: RouteEpoch(4),
        valid_for: TimeWindow::new(Tick(4), Tick(10)).expect("lease window"),
    }
}

fn materialization_input(
    route_id: RouteId,
    admission: jacquard_core::RouteAdmission,
) -> RouteMaterializationInput {
    let lease = lease();
    RouteMaterializationInput {
        handle: RouteHandle {
            stamp: jacquard_core::RouteIdentityStamp {
                route_id,
                topology_epoch: lease.lease_epoch,
                materialized_at_tick: lease.valid_for.start_tick(),
                publication_id: PublicationId([7; 16]),
            },
        },
        admission,
        lease,
    }
}

mod commitments;
mod control_observer;
mod lifecycle;
mod replay_recovery;
mod service_runtime;
