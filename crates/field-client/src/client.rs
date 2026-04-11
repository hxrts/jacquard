//! `FieldClientBuilder` and `FieldClient`: host handle for the field routing
//! engine.
//!
//! `FieldClient` drives a `FieldEngine` directly without a router or bridge
//! layer by calling `RoutingEngine` and `RouterManagedEngine` methods in the
//! order the engine expects. Each node in a multi-node scenario gets its own
//! `FieldClient` instance connected to a shared `SharedInMemoryNetwork`.
//!
//! The handle covers the full single-hop route lifecycle: `advance_round` seeds
//! local field state from a topology observation, `activate_route` runs
//! candidate selection through admission and materialization, `maintain_route`
//! applies maintenance triggers and handles expiry, `forward_payload` sends a
//! payload over in-memory transport, and `drain_peer_ingress` reads what the
//! remote peer received.

use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, ConnectivityPosture, DestinationId, DiversityFloor, DurationMs, LinkEndpoint,
    MaterializedRoute, NodeId, Observation, OperatingMode, PriorityPoints, PublicationId,
    RouteError, RouteHandle, RouteId, RouteIdentityStamp, RouteLease, RouteMaintenanceFailure,
    RouteMaintenanceResult, RouteMaintenanceTrigger, RouteMaterializationInput,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteRuntimeError,
    RouteSelectionError, RouteServiceKind, RoutingEngineFallbackPolicy, RoutingObjective,
    RoutingTickContext, RoutingTickOutcome, SelectedRoutingParameters, Tick, TimeWindow,
    TransportError, TransportIngressEvent,
};
use jacquard_field::FieldEngine;
use jacquard_mem_link_profile::{InMemoryRuntimeEffects, InMemoryTransport, SharedInMemoryNetwork};
use jacquard_traits::{RouterManagedEngine, RoutingEngine, RoutingEnginePlanner, TransportDriver};

const DEFAULT_ROUTE_LEASE_TICKS: u64 = 32;

pub struct FieldClientBuilder {
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
    now: Tick,
    profile: SelectedRoutingParameters,
}

impl FieldClientBuilder {
    #[must_use]
    pub fn new(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        now: Tick,
    ) -> Self {
        Self {
            local_node_id,
            topology,
            network,
            now,
            profile: default_profile(),
        }
    }

    #[must_use]
    pub fn with_profile(mut self, profile: SelectedRoutingParameters) -> Self {
        self.profile = profile;
        self
    }

    #[must_use]
    pub fn build(self) -> FieldClient {
        let mut peer_transports = BTreeMap::new();
        let local_endpoint = local_endpoint(&self.topology, self.local_node_id);
        let engine_transport =
            InMemoryTransport::attach(self.local_node_id, [local_endpoint], self.network.clone());
        for (node_id, node) in &self.topology.value.nodes {
            if *node_id == self.local_node_id {
                continue;
            }
            peer_transports.insert(
                *node_id,
                InMemoryTransport::attach(
                    *node_id,
                    node.profile.endpoints.clone(),
                    self.network.clone(),
                ),
            );
        }

        FieldClient {
            local_node_id: self.local_node_id,
            topology: self.topology,
            now: self.now,
            profile: self.profile,
            engine: FieldEngine::new(
                self.local_node_id,
                engine_transport,
                InMemoryRuntimeEffects {
                    now: self.now,
                    ..Default::default()
                },
            ),
            network: self.network,
            peer_transports,
            active_routes: BTreeMap::new(),
        }
    }
}

pub struct FieldClient {
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    now: Tick,
    profile: SelectedRoutingParameters,
    engine: FieldEngine<InMemoryTransport, InMemoryRuntimeEffects>,
    network: SharedInMemoryNetwork,
    peer_transports: BTreeMap<NodeId, InMemoryTransport>,
    active_routes: BTreeMap<RouteId, MaterializedRoute>,
}

impl FieldClient {
    pub fn ingest_topology(&mut self, topology: Observation<Configuration>) {
        for (node_id, node) in &topology.value.nodes {
            if *node_id == self.local_node_id || self.peer_transports.contains_key(node_id) {
                continue;
            }
            self.peer_transports.insert(
                *node_id,
                InMemoryTransport::attach(
                    *node_id,
                    node.profile.endpoints.clone(),
                    self.network.clone(),
                ),
            );
        }
        self.now = topology.observed_at_tick;
        self.topology = topology;
    }

    pub fn advance_round(&mut self) -> Result<RoutingTickOutcome, RouteError> {
        self.now = self.topology.observed_at_tick;
        self.engine
            .engine_tick(&RoutingTickContext::new(self.topology.clone()))
    }

    pub fn candidate_routes(
        &self,
        objective: &RoutingObjective,
    ) -> Vec<jacquard_core::RouteCandidate> {
        self.engine
            .candidate_routes(objective, &self.profile, &self.topology)
    }

    pub fn activate_route(
        &mut self,
        objective: &RoutingObjective,
    ) -> Result<MaterializedRoute, RouteError> {
        self.advance_round()?;
        let candidate = self
            .candidate_routes(objective)
            .into_iter()
            .next()
            .ok_or(RouteSelectionError::NoCandidate)?;
        let route_id = candidate.route_id;
        let admission =
            self.engine
                .admit_route(objective, &self.profile, candidate, &self.topology)?;
        let input = self.materialization_input(route_id, &admission)?;
        let installation = self.engine.materialize_route(input.clone())?;
        let route = MaterializedRoute::from_installation(input, installation);
        self.active_routes
            .insert(route.identity.stamp.route_id, route.clone());
        Ok(route)
    }

    pub fn active_route(&self, route_id: &RouteId) -> Option<&MaterializedRoute> {
        self.active_routes.get(route_id)
    }

    pub fn maintain_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        let route = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        let mut runtime = route.runtime.clone();
        let result = self
            .engine
            .maintain_route(&route.identity, &mut runtime, trigger)?;
        route.runtime = runtime;
        if result.event == jacquard_core::RouteLifecycleEvent::Expired
            || matches!(
                result.outcome,
                jacquard_core::RouteMaintenanceOutcome::Failed(
                    RouteMaintenanceFailure::LeaseExpired
                        | RouteMaintenanceFailure::LostReachability
                        | RouteMaintenanceFailure::CapacityExceeded
                )
            )
        {
            self.engine.teardown(route_id);
            self.active_routes.remove(route_id);
        }
        Ok(result)
    }

    pub fn forward_payload(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        self.engine.forward_payload_for_router(route_id, payload)
    }

    pub fn drain_peer_ingress(
        &mut self,
        peer_node_id: NodeId,
    ) -> Result<Vec<TransportIngressEvent>, TransportError> {
        self.peer_transports
            .get_mut(&peer_node_id)
            .ok_or(TransportError::Unavailable)?
            .drain_transport_ingress()
    }

    fn materialization_input(
        &self,
        route_id: RouteId,
        admission: &jacquard_core::RouteAdmission,
    ) -> Result<RouteMaterializationInput, RouteError> {
        let mut publication = [0_u8; 16];
        publication[..8].copy_from_slice(&self.now.0.to_le_bytes());
        let lease = RouteLease {
            owner_node_id: self.local_node_id,
            lease_epoch: self.topology.value.epoch,
            valid_for: TimeWindow::new(
                self.now,
                Tick(self.now.0.saturating_add(DEFAULT_ROUTE_LEASE_TICKS)),
            )
            .map_err(|_| RouteRuntimeError::Invalidated)?,
        };
        Ok(RouteMaterializationInput {
            handle: RouteHandle {
                stamp: RouteIdentityStamp {
                    route_id,
                    topology_epoch: self.topology.value.epoch,
                    materialized_at_tick: self.now,
                    publication_id: PublicationId(publication),
                },
            },
            admission: admission.clone(),
            lease,
        })
    }
}

fn local_endpoint(topology: &Observation<Configuration>, local_node_id: NodeId) -> LinkEndpoint {
    topology.value.nodes[&local_node_id]
        .profile
        .endpoints
        .first()
        .cloned()
        .expect("field topology must provide a local endpoint")
}

pub fn default_objective(destination: DestinationId) -> RoutingObjective {
    RoutingObjective {
        destination,
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

pub fn default_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile: OperatingMode::FieldPartitionTolerant,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}
