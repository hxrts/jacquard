# World Extensions

This page describes how external code contributes observed nodes, links, environment state, services, and transport activity to Jacquard's shared world picture.

## Extension Model

Jacquard is extended at several layers. World extensions add observed objects to the shared world picture. Routing engines consume that picture and produce route behavior. Policy and coordination traits decide how engines are selected, composed, or locally coordinated. Operational subcomponents and runtime effects support those higher layers.

These layers stay separate on purpose. A team can extend the world without becoming a routing-engine author, add a routing engine without redefining the world schema, or add host policy without modifying a routing engine.

This page covers the world layer. See [Routing Engines](108_routing_engines.md) for the engine and policy contracts, [Runtime Effects](104_runtime_effects.md) for the host capability surface, and [Mesh Routing](110_mesh_routing.md) for the in-tree mesh implementation and its swappable subcomponents.

## World Extension Surface

World extensions are the entry point for teams that know a specific radio stack, runtime environment, discovery surface, or device class. The key idea is simple. Jacquard has one shared world schema in `jacquard-core`. A world extension adds observations of that schema. It does not define a private alternative node or link type.

The shared world schema is documented in [Pipeline and World Observations](105_pipeline_observations.md). An extension constructs `Node`, `Link`, `Environment`, and the related observation types defined there, and emits them through the trait surface defined below. The example in the next section shows how to wire a real device into that schema end-to-end.

### Example: Adding A New Device

In practice, adding support for a new device means translating that device's capabilities into a concrete `NodeProfile`, pairing it with the current observed `NodeState`, and returning the result as a `NodeObservation`. In this example, the device is a BLE relay with one BLE endpoint, four concurrent connections, limited transfer concurrency, and a moderate local retention budget.

```rust
// Link objects describe the device's carrier endpoint and current observed link health.
let ble_relay_endpoint = LinkEndpoint {
    protocol: TransportProtocol::BleGatt,
    address: EndpointAddress::Opaque(vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06]),
    mtu_bytes: ByteCount(185),
};

let ble_relay_link_state = LinkState {
    state: LinkRuntimeState::Active,
    median_rtt_ms: DurationMs(35),
    transfer_rate_bytes_per_sec: Belief::Estimated(Estimate {
        value: 12_000,
        confidence_permille: RatioPermille(900),
        updated_at_tick: Tick(42),
    }),
    stability_horizon_ms: Belief::Estimated(Estimate {
        value: DurationMs(20_000),
        confidence_permille: RatioPermille(850),
        updated_at_tick: Tick(42),
    }),
    loss_permille: RatioPermille(15),
    delivery_confidence_permille: Belief::Estimated(Estimate {
        value: RatioPermille(970),
        confidence_permille: RatioPermille(900),
        updated_at_tick: Tick(42),
    }),
    symmetry_permille: Belief::Estimated(Estimate {
        value: RatioPermille(950),
        confidence_permille: RatioPermille(800),
        updated_at_tick: Tick(42),
    }),
};

let ble_relay_link = Link {
    endpoint: ble_relay_endpoint.clone(),
    state: ble_relay_link_state,
};

// Node objects describe the device's stable capabilities and current observed node state.
let ble_relay_profile = NodeProfile {
    services: vec![
        /* Discover / Move / Hold descriptors for this node */
    ],
    endpoints: vec![ble_relay_endpoint.clone()],
    connection_count_max: 4,
    neighbor_state_count_max: 16,
    simultaneous_transfer_count_max: 2,
    active_route_count_max: 8,
    relay_work_budget_max: 64,
    maintenance_work_budget_max: 32,
    hold_item_count_max: 128,
    hold_capacity_bytes_max: ByteCount(65_536),
};

let ble_relay_state = NodeState {
    relay_budget: Belief::Absent,
    available_connection_count: Belief::Absent,
    hold_capacity_available_bytes: Belief::Absent,
    information_summary: Belief::Absent,
};

let ble_relay_node = Node {
    controller_id: ControllerId([7; 32]),
    profile: ble_relay_profile,
    state: ble_relay_state,
};

// Assembly turns those shared objects into a world extension that emits observations.
struct BleRelayExtension;

impl WorldExtensionDescriptor for BleRelayExtension {
    fn extension_id(&self) -> &str {
        "ble-relay"
    }

    fn supported_transports(&self) -> Vec<TransportProtocol> {
        vec![TransportProtocol::BleGatt]
    }
}

impl NodeWorldExtension for BleRelayExtension {
    fn poll_node_observations(&mut self) -> Result<Vec<NodeObservation>, WorldError> {
        Ok(vec![Observation {
            value: ble_relay_node,
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(42),
        }])
    }
}

impl LinkWorldExtension for BleRelayExtension {
    fn poll_link_observations(&mut self) -> Result<Vec<LinkObservation>, WorldError> {
        Ok(vec![Observation {
            value: ble_relay_link,
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(42),
        }])
    }
}
```

In a real routing participant, that `services` list would normally advertise `Discover`, `Move`, and `Hold` for the engine contracts the node can join. The example keeps those descriptors abbreviated so the focus stays on mapping the device into the shared world schema.

### World Extension Trait Options

These are the main world-extension entry points. Most contributors implement only the ones that match the kinds of objects they observe.

```rust
pub trait WorldExtensionDescriptor {
    #[must_use]
    fn extension_id(&self) -> &str;

    #[must_use]
    fn supported_transports(&self) -> Vec<TransportProtocol>;
}

pub trait NodeWorldExtension: WorldExtensionDescriptor {
    fn poll_node_observations(&mut self) -> Result<Vec<NodeObservation>, WorldError>;
}

pub trait LinkWorldExtension: WorldExtensionDescriptor {
    fn poll_link_observations(&mut self) -> Result<Vec<LinkObservation>, WorldError>;
}

pub trait EnvironmentWorldExtension: WorldExtensionDescriptor {
    fn poll_environment_observations(&mut self) -> Result<Vec<EnvironmentObservation>, WorldError>;
}

pub trait ServiceWorldExtension: WorldExtensionDescriptor {
    fn poll_service_observations(&mut self) -> Result<Vec<ServiceObservation>, WorldError>;
}

pub trait TransportWorldExtension: WorldExtensionDescriptor {
    fn poll_transport_observations(
        &mut self,
    ) -> Result<Vec<Observation<TransportObservation>>, WorldError>;
}
```

A team that adds a new device will often implement `NodeWorldExtension`, `LinkWorldExtension`, or both. The other facets are available when an extension also emits environment, service, or transport observations. These boundaries use `WorldError` rather than `RouteError` because a world extension contributes world input. It does not own routing semantics.

### Umbrella World Extension

This surface is optional. It is useful when an extension naturally wants to emit one combined world-observation stream instead of separate node, link, environment, service, or transport streams.

```rust
pub trait WorldExtension: WorldExtensionDescriptor {
    fn poll_observations(&mut self) -> Result<Vec<WorldObservation>, WorldError>;
}
```

`WorldExtensionDescriptor` is pure metadata. `WorldExtension` is effectful. Use it when one extension discovers many kinds of observed objects together and the host prefers to ingest them as one self-describing stream. Higher-level batching, diffing, merging, and checkpointing still happen above this boundary.

This is the main cooperative extension surface in Jacquard. One team may add observed BLE nodes. Another may add observed Wi-Fi links. Another may add platform-specific service or transport observations. A host merges those contributions into one world picture. Routing engines then consume that merged picture through the shared routing traits.

## Capability Advertisement

Jacquard does not use one global "which algorithm are you running?" handshake. Instead, nodes advertise shared routing-engine participation and cooperative services through `ServiceDescriptor`.

```rust
pub struct ServiceDescriptor {
    pub provider_node_id: NodeId,
    pub controller_id: ControllerId,
    pub service_kind: RouteServiceKind,
    pub endpoints: Vec<LinkEndpoint>,
    pub routing_engines: Vec<RoutingEngineId>,
    pub scope: ServiceScope,
    pub valid_for: TimeWindow,
    pub capacity: Belief<CapacityHint>,
}
```

This advertisement tells peers what the node can participate in and what it offers. It is enough for discovery and coarse selection.

For nodes that participate in routing at all, Jacquard treats the default shared capability surface as:

- `Discover` for service and route-relevant discovery
- `Move` for admitted-route payload carriage
- `Hold` for retention-backed delayed or partition-tolerant delivery
- shared relay budget, connection headroom, hold capacity, link-quality observations, and coarse environment observations

Routing engines may add stricter interpretation on top of that surface, but they should not need a second node-capability vocabulary just to participate.

If stronger terms are needed, they should be negotiated narrowly and per service. For example, `Hold` may negotiate retention limits, and `Repair` may negotiate route-specific participation. Jacquard does not currently define one universal negotiation object for all services.
