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
    DestinationInterestClass, HopBand, NeighborContinuation, ObservationClass, SupportBucket,
    MAX_ACTIVE_DESTINATIONS,
};
use crate::summary::{
    EvidenceContributionClass, FieldSummary, SummaryDestinationKey, SummaryUncertaintyClass,
};

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

#[derive(Clone, Debug)]
struct TestDestinationSeed {
    destination: DestinationId,
    interest_class: DestinationInterestClass,
    observed_at_tick: Tick,
    top_corridor_mass: u16,
    usability_entropy: u16,
    observation_class: ObservationClass,
    hop_band: HopBand,
    delivery_support: u16,
    retention_affinity: u16,
    frontier: Vec<NeighborContinuation>,
    endpoint_neighbors: Vec<NodeId>,
}

impl TestDestinationSeed {
    fn node_destination(node_id: NodeId) -> Self {
        Self {
            destination: DestinationId::Node(node_id),
            interest_class: DestinationInterestClass::Transit,
            observed_at_tick: Tick(4),
            top_corridor_mass: 850,
            usability_entropy: 200,
            observation_class: ObservationClass::DirectOnly,
            hop_band: HopBand::new(1, 2),
            delivery_support: 800,
            retention_affinity: 300,
            frontier: Vec::new(),
            endpoint_neighbors: Vec::new(),
        }
    }

    fn with_frontier_neighbor(
        mut self,
        neighbor_id: NodeId,
        net_value: u16,
        downstream_support: u16,
        freshness: Tick,
    ) -> Self {
        self.frontier.push(NeighborContinuation {
            neighbor_id,
            net_value: SupportBucket::new(net_value),
            downstream_support: SupportBucket::new(downstream_support),
            expected_hop_band: self.hop_band,
            freshness,
        });
        self
    }

    fn with_endpoint_neighbor(mut self, neighbor_id: NodeId) -> Self {
        self.endpoint_neighbors.push(neighbor_id);
        self
    }

    fn apply<Transport>(&self, engine: &mut FieldEngine<Transport, ()>) {
        let state = engine.state.upsert_destination_interest(
            &self.destination,
            self.interest_class,
            self.observed_at_tick,
        );
        state.posterior.top_corridor_mass = SupportBucket::new(self.top_corridor_mass);
        state.posterior.usability_entropy =
            crate::state::EntropyBucket::new(self.usability_entropy);
        state.posterior.predicted_observation_class = self.observation_class;
        state.corridor_belief.expected_hop_band = self.hop_band;
        state.corridor_belief.delivery_support = SupportBucket::new(self.delivery_support);
        state.corridor_belief.retention_affinity = SupportBucket::new(self.retention_affinity);
        for continuation in &self.frontier {
            state.frontier = state.frontier.clone().insert(continuation.clone());
        }
        for neighbor_id in &self.endpoint_neighbors {
            engine.state.neighbor_endpoints.insert(
                *neighbor_id,
                jacquard_host_support::opaque_endpoint(
                    jacquard_core::TransportKind::WifiAware,
                    vec![neighbor_id.0[0]],
                    ByteCount(128),
                ),
            );
        }
    }
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
                            jacquard_host_support::opaque_endpoint(
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
                            jacquard_host_support::opaque_endpoint(
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
    TestDestinationSeed::node_destination(node(2))
        .with_frontier_neighbor(node(2), 900, 850, Tick(4))
        .with_endpoint_neighbor(node(2))
        .apply(&mut engine);
    engine
}

fn seeded_transport_engine() -> FieldEngine<InMemoryTransport, ()> {
    let mut engine = FieldEngine::new(node(1), InMemoryTransport::default(), ());
    TestDestinationSeed::node_destination(node(2))
        .with_frontier_neighbor(node(2), 900, 850, Tick(4))
        .with_endpoint_neighbor(node(2))
        .apply(&mut engine);
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
