//! `SimulatedServiceDescriptor`, a builder for one shared `ServiceDescriptor`
//! emitted by a simulated node. Carries the service kind, endpoints, scope,
//! validity window, saturation, and repair capacity, and binds the descriptor
//! to a `(NodeId, ControllerId)` pair on build.

use jacquard_core::{
    Belief, ByteCount, CapacityHint, ControllerId, LinkEndpoint, NodeId, RatioPermille,
    RepairCapacitySlots, RouteServiceKind, RoutingEngineId, ServiceDescriptor,
    ServiceScope, Tick, TimeWindow,
};

/// Builder for one shared service descriptor emitted by a simulated node.
#[derive(Clone, Debug)]
pub struct SimulatedServiceDescriptor {
    service_kind: RouteServiceKind,
    endpoints: Vec<LinkEndpoint>,
    routing_engines: Vec<RoutingEngineId>,
    scope: ServiceScope,
    valid_for: TimeWindow,
    saturation_permille: RatioPermille,
    repair_capacity: u32,
    hold_capacity_bytes: Option<ByteCount>,
    observed_at_tick: Tick,
}

impl SimulatedServiceDescriptor {
    #[must_use]
    pub fn new(service_kind: RouteServiceKind) -> Self {
        Self {
            service_kind,
            endpoints: Vec::new(),
            routing_engines: Vec::new(),
            scope: ServiceScope::Introduction { scope_token: vec![1] },
            valid_for: TimeWindow::new(Tick(0), Tick(64))
                .expect("valid default service window"),
            saturation_permille: RatioPermille(100),
            repair_capacity: 0,
            hold_capacity_bytes: None,
            observed_at_tick: Tick(0),
        }
    }

    #[must_use]
    pub fn with_endpoint(mut self, endpoint: LinkEndpoint) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    #[must_use]
    pub fn with_scope(mut self, scope: ServiceScope) -> Self {
        self.scope = scope;
        self
    }

    #[must_use]
    pub fn with_routing_engine(mut self, routing_engine: &RoutingEngineId) -> Self {
        self.routing_engines.push(routing_engine.clone());
        self
    }

    #[must_use]
    pub fn with_valid_for(mut self, valid_for: TimeWindow) -> Self {
        self.valid_for = valid_for;
        self
    }

    #[must_use]
    pub fn with_saturation(mut self, saturation_permille: RatioPermille) -> Self {
        self.saturation_permille = saturation_permille;
        self
    }

    #[must_use]
    pub fn with_capacity_profile(
        mut self,
        repair_capacity: u32,
        hold_capacity_bytes: Option<ByteCount>,
    ) -> Self {
        self.repair_capacity = repair_capacity;
        self.hold_capacity_bytes = hold_capacity_bytes;
        self
    }

    #[must_use]
    pub fn with_observed_at_tick(mut self, observed_at_tick: Tick) -> Self {
        self.observed_at_tick = observed_at_tick;
        self
    }

    #[must_use]
    pub fn advertised(
        service_kind: RouteServiceKind,
        endpoint: LinkEndpoint,
        scope: ServiceScope,
        valid_for: TimeWindow,
        observed_at_tick: Tick,
    ) -> Self {
        Self::new(service_kind)
            .with_endpoint(endpoint)
            .with_scope(scope)
            .with_valid_for(valid_for)
            .with_observed_at_tick(observed_at_tick)
    }

    #[must_use]
    pub fn build(
        self,
        node_id: NodeId,
        controller_id: ControllerId,
    ) -> ServiceDescriptor {
        let capacity_hint = self.capacity_hint();
        ServiceDescriptor {
            provider_node_id: node_id,
            controller_id,
            service_kind: self.service_kind,
            endpoints: self.endpoints,
            routing_engines: self.routing_engines,
            scope: self.scope,
            valid_for: self.valid_for,
            capacity: Belief::certain(capacity_hint, self.observed_at_tick),
        }
    }
}

impl SimulatedServiceDescriptor {
    fn capacity_hint(&self) -> CapacityHint {
        let mut capacity = CapacityHint::new(self.saturation_permille)
            .with_repair_capacity_slots(
                RepairCapacitySlots(self.repair_capacity),
                self.observed_at_tick,
            );
        if let Some(hold_capacity_bytes) = self.hold_capacity_bytes {
            capacity = capacity
                .with_hold_capacity_bytes(hold_capacity_bytes, self.observed_at_tick);
        }
        capacity
    }
}
