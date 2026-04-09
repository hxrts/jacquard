//! `SimulatedServiceDescriptor`, a builder for one shared `ServiceDescriptor`
//! emitted by a simulated node.
//!
//! This module assembles a single `ServiceDescriptor` that a node advertises to
//! peers. Each descriptor carries the service kind (`Discover`, `Move`, or
//! `Hold`), one or more `LinkEndpoint` values, a `ServiceScope`, a validity
//! `TimeWindow`, saturation level, and repair capacity. The descriptor is bound
//! to a `(NodeId, ControllerId)` identity pair at `build` time.
//!
//! Three preset constructors mirror the standard service triple:
//! - `discover_service`: advertises route-discovery participation.
//! - `move_service`: advertises payload forwarding capability.
//! - `hold_service`: advertises deferred-delivery buffering with a hold
//!   capacity hint.
//!
//! `RouteServiceBundle` captures the standard discover/move/hold triple as a
//! named concept so human-facing node presets do not need to assemble the
//! service set imperatively.
//!
//! The generic `advertised` constructor covers non-standard service kinds.
//! Routing engines are attached via `with_routing_engine` before building.
//!
//! This builder is used by `SimulatedNodeProfile` and `NodePreset`; callers
//! should rarely need to construct it directly.

use jacquard_core::{
    ByteCount, CapacityHint, ControllerId, LinkEndpoint, NodeId, RatioPermille,
    RepairCapacitySlots, RouteServiceKind, RoutingEngineId, ServiceDescriptor,
    ServiceDescriptorBuilder, ServiceScope, Tick, TimeWindow,
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

/// Named bundle for the standard route-service triple (discover, move, hold).
#[derive(Clone, Debug)]
pub struct RouteServiceBundle {
    services: Vec<SimulatedServiceDescriptor>,
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
    pub fn discover_service(
        endpoint: LinkEndpoint,
        scope: ServiceScope,
        valid_for: TimeWindow,
        observed_at_tick: Tick,
    ) -> Self {
        Self::advertised(
            RouteServiceKind::Discover,
            endpoint,
            scope,
            valid_for,
            observed_at_tick,
        )
        .with_capacity_profile(4, None)
    }

    #[must_use]
    pub fn move_service(
        endpoint: LinkEndpoint,
        scope: ServiceScope,
        valid_for: TimeWindow,
        observed_at_tick: Tick,
    ) -> Self {
        Self::advertised(
            RouteServiceKind::Move,
            endpoint,
            scope,
            valid_for,
            observed_at_tick,
        )
        .with_capacity_profile(4, None)
    }

    #[must_use]
    pub fn hold_service(
        endpoint: LinkEndpoint,
        scope: ServiceScope,
        valid_for: TimeWindow,
        observed_at_tick: Tick,
    ) -> Self {
        Self::advertised(
            RouteServiceKind::Hold,
            endpoint,
            scope,
            valid_for,
            observed_at_tick,
        )
        .with_capacity_profile(4, Some(ByteCount(4096)))
    }

    #[must_use]
    pub fn build(
        self,
        node_id: NodeId,
        controller_id: ControllerId,
    ) -> ServiceDescriptor {
        let capacity_hint = self.capacity_hint();
        let mut builder =
            ServiceDescriptorBuilder::new(node_id, controller_id, self.service_kind)
                .with_scope(self.scope)
                .with_valid_for(self.valid_for)
                .with_capacity(capacity_hint, self.observed_at_tick);
        for endpoint in self.endpoints {
            builder = builder.with_endpoint(endpoint);
        }
        for routing_engine in self.routing_engines {
            builder = builder.with_routing_engine(&routing_engine);
        }
        builder.build()
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

impl RouteServiceBundle {
    #[must_use]
    pub fn route_capable(
        endpoint: &LinkEndpoint,
        routing_engines: &[RoutingEngineId],
        scope: &ServiceScope,
        valid_for: TimeWindow,
        observed_at_tick: Tick,
    ) -> Self {
        let mut services = Vec::with_capacity(routing_engines.len().saturating_mul(3));
        for routing_engine in routing_engines {
            services.push(
                SimulatedServiceDescriptor::discover_service(
                    endpoint.clone(),
                    scope.clone(),
                    valid_for,
                    observed_at_tick,
                )
                .with_routing_engine(routing_engine),
            );
            services.push(
                SimulatedServiceDescriptor::move_service(
                    endpoint.clone(),
                    scope.clone(),
                    valid_for,
                    observed_at_tick,
                )
                .with_routing_engine(routing_engine),
            );
            services.push(
                SimulatedServiceDescriptor::hold_service(
                    endpoint.clone(),
                    scope.clone(),
                    valid_for,
                    observed_at_tick,
                )
                .with_routing_engine(routing_engine),
            );
        }
        Self { services }
    }

    pub(crate) fn into_services(self) -> Vec<SimulatedServiceDescriptor> {
        self.services
    }
}
