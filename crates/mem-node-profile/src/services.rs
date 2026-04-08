use jacquard_core::{
    Belief, CapacityHint, ControllerId, Estimate, LinkEndpoint, NodeId, RatioPermille,
    RouteServiceKind, ServiceDescriptor, ServiceScope, Tick, TimeWindow,
};

/// Builder for one shared service descriptor emitted by a simulated node.
#[derive(Clone, Debug)]
pub struct SimulatedServiceDescriptor {
    service_kind: RouteServiceKind,
    endpoints: Vec<LinkEndpoint>,
    scope: ServiceScope,
    valid_for: TimeWindow,
    saturation_permille: RatioPermille,
    repair_capacity: u32,
}

impl SimulatedServiceDescriptor {
    #[must_use]
    pub fn new(service_kind: RouteServiceKind) -> Self {
        Self {
            service_kind,
            endpoints: Vec::new(),
            scope: ServiceScope::Introduction { scope_token: vec![1] },
            valid_for: TimeWindow::new(Tick(0), Tick(64))
                .expect("valid default service window"),
            saturation_permille: RatioPermille(100),
            repair_capacity: 0,
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
    pub fn with_repair_capacity(mut self, repair_capacity: u32) -> Self {
        self.repair_capacity = repair_capacity;
        self
    }

    #[must_use]
    pub fn build(
        self,
        node_id: NodeId,
        controller_id: ControllerId,
    ) -> ServiceDescriptor {
        ServiceDescriptor {
            provider_node_id: node_id,
            controller_id,
            service_kind: self.service_kind,
            endpoints: self.endpoints,
            routing_engines: Vec::new(),
            scope: self.scope,
            valid_for: self.valid_for,
            capacity: Belief::Estimated(Estimate {
                value: CapacityHint {
                    saturation_permille: self.saturation_permille,
                    repair_capacity_slots: Belief::Estimated(Estimate {
                        value: jacquard_core::RepairCapacitySlots(self.repair_capacity),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(0),
                    }),
                    hold_capacity_bytes: Belief::Absent,
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(0),
            }),
        }
    }
}
