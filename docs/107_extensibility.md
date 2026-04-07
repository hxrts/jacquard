# Extensibility

This page describes the main extension surfaces in Jacquard. It focuses on the production surfaces that other teams are expected to implement.

## Extension Model

Jacquard is extended at several layers. World extensions add observed objects to the shared world picture. Routing engines consume that picture and produce route behavior. Policy and coordination traits decide how engines are selected, composed, or locally coordinated. Operational subcomponents and runtime effects support those higher layers.

These layers stay separate on purpose. A team can extend the world without becoming a routing-engine author, add a routing engine without redefining the world schema, or add host policy without modifying a routing engine.

## World Extensions

World extensions are the entry point for teams that know a specific radio stack, runtime environment, discovery surface, or device class. The key idea is simple. Jacquard has one shared world schema in `jacquard-core`. A world extension adds observations of that schema. It does not define a private alternative node or link type.

### Shared World Schema

Nodes are represented as a stable capability profile plus changing observed state.

```rust
pub struct NodeProfile {
    pub services: Vec<ServiceDescriptor>,
    pub endpoints: Vec<LinkEndpoint>,
    pub connection_count_max: u32,
    pub neighbor_state_count_max: u32,
    pub simultaneous_transfer_count_max: u32,
    pub active_route_count_max: u32,
    pub relay_work_budget_max: u32,
    pub maintenance_work_budget_max: u32,
    pub hold_item_count_max: u32,
    pub hold_capacity_bytes_max: ByteCount,
}

pub struct NodeState {
    pub relay_budget: Belief<NodeRelayBudget>,
    pub available_connection_count: Belief<u32>,
    pub hold_capacity_available_bytes: Belief<ByteCount>,
    pub information_summary: Belief<InformationSetSummary>,
}

pub struct Node {
    pub controller_id: ControllerId,
    pub profile: NodeProfile,
    pub state: NodeState,
}
```

Links are represented as a stable endpoint description plus changing observed link state.

```rust
pub struct LinkEndpoint {
    pub protocol: TransportProtocol,
    pub address: EndpointAddress,
    pub mtu_bytes: ByteCount,
}

pub struct LinkState {
    pub state: LinkRuntimeState,
    pub median_rtt_ms: DurationMs,
    pub transfer_rate_bytes_per_sec: Belief<u32>,
    pub stability_horizon_ms: Belief<DurationMs>,
    pub loss_permille: RatioPermille,
    pub delivery_confidence_permille: Belief<RatioPermille>,
    pub symmetry_permille: Belief<RatioPermille>,
}

pub struct Link {
    pub endpoint: LinkEndpoint,
    pub state: LinkState,
}
```

Environment captures shared local conditions around the current world view, and `Configuration` gathers the current world picture.

```rust
pub struct Environment {
    pub reachable_neighbor_count: u32,
    pub churn_permille: RatioPermille,
    pub contention_permille: RatioPermille,
}

pub struct Configuration {
    pub epoch: RouteEpoch,
    pub nodes: BTreeMap<NodeId, Node>,
    pub links: BTreeMap<(NodeId, NodeId), Link>,
    pub environment: Environment,
}
```

This is the shared world schema that every world extension targets. A developer who wants to add a new observed node or link needs to construct these exact objects. The extension point is not defining new world schema. The extension point is emitting self-describing observations over the shared world schema.

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

A team that adds a new device will often implement `NodeWorldExtension`, `LinkWorldExtension`, or both. The other facets are available when an extension also emits environment, service, or transport observations. These boundaries use `WorldError` rather than `RouteError` because a world extension contributes world input; it does not own routing semantics.

### Umbrella World Extension

This surface is optional. It is useful when an extension naturally wants to emit one combined world-observation stream instead of separate node, link, environment, service, or transport streams.

```rust
pub trait WorldExtension: WorldExtensionDescriptor {
    fn poll_observations(&mut self) -> Result<Vec<WorldObservation>, WorldError>;
}
```

`WorldExtensionDescriptor` is pure metadata. `WorldExtension` is effectful. Use it when one extension discovers many kinds of observed objects together and the host prefers to ingest them as one self-describing stream. Higher-level batching, diffing, merging, and checkpointing still happen above this boundary.

This is the main cooperative extension surface in Jacquard. One team may add observed BLE nodes. Another may add observed Wi-Fi links. Another may add platform-specific service or transport observations. A host merges those contributions into one world picture. Routing engines then consume that merged picture through the shared routing traits.

### Capability Advertisement

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

For nodes that participate in routing at all, Jacquard should treat the default shared capability surface as:

- `Discover` for service and route-relevant discovery
- `Move` for admitted-route payload carriage
- `Hold` for retention-backed delayed or partition-tolerant delivery
- shared relay budget, connection headroom, hold capacity, link-quality observations, and coarse environment observations

Routing engines may add stricter interpretation on top of that surface, but they should not need a second node-capability vocabulary just to participate.

If stronger terms are needed, they should be negotiated narrowly and per service. For example, `Hold` may negotiate retention limits, and `Repair` may negotiate route-specific participation. Jacquard does not currently define one universal negotiation object for all services.

## Routing Engines

A routing engine is a routing algorithm that consumes the shared world picture and realizes routes under router-provided identity. Mesh is the first-party engine. External engines such as onion routing plug into the same contract without depending on mesh internals.

```rust
pub trait RoutingEnginePlanner {
    #[must_use]
    fn engine_id(&self) -> RoutingEngineId;

    #[must_use]
    fn capabilities(&self) -> RoutingEngineCapabilities;

    #[must_use]
    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate>;

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError>;

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError>;
}

pub trait RoutingEngine: RoutingEnginePlanner {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError>;

    fn route_commitments(&self, route: &MaterializedRoute) -> Vec<RouteCommitment>;

    fn engine_tick(&mut self, topology: &Observation<Configuration>) -> Result<(), RouteError> {
        Ok(())
    }

    fn maintain_route(
        &mut self,
        identity: &MaterializedRouteIdentity,
        runtime: &mut RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}
```

`RoutingEnginePlanner` is pure. `RoutingEngine` is effectful. The split keeps candidate production deterministic and keeps runtime mutation inside explicit realization and maintenance methods. The router allocates canonical route identity first. The engine realizes the admitted route under that identity and returns `RouteInstallation`. The final `MaterializedRoute` is assembled above the engine boundary as router-owned identity plus engine-owned runtime state, and maintenance only receives the mutable runtime portion. That activation step also enforces the shared control-plane invariants: the admission decision must still be admissible, the realized protection must satisfy the objective protection floor, and lease validity must be checked explicitly before maintenance or publication proceeds.

`engine_tick` is the optional engine-wide bootstrap and convergence hook. An engine may use it as an internal middleware-style loop to refresh local regime estimates, decay stale local state, update coordination posture, or prepare engine-private planning state before any specific route is active. The host or router drives that cadence through the control plane's existing periodic tick path; the hook itself does not publish canonical route truth directly.

Two contract rules are worth keeping explicit. If a planning or admission judgment depends on observations, the current topology must be passed into that method directly rather than read from ambient engine state. And if an engine keeps planner caches, those caches are memoization only: cache hits and misses must not change the semantic result for the same topology.

External routing engines should depend on `jacquard-core` and `jacquard-traits`. They should not depend on mesh internals, router internals, or simulator-private helpers. The stable shared contract includes `RouteSummary`, `Estimate<RouteEstimate>`, `RouteAdmissionCheck`, `RouteWitness`, `RouteHandle`, `RouteLease`, `RouteMaterializationInput`, `RouteInstallation`, `RouteCommitment`, `RouteMaintenanceResult`, `CommitteeSelection`, `SubstrateRequirements`, `SubstrateLease`, `LayerParameters`, `Observation<T>`, and `Fact<T>`. External engines must not assume mesh route shape, mesh topology structure, mesh-specific maintenance semantics, or any authority model outside those shared route objects.

## Policy And Coordination

Policy and coordination traits are separate from route realization. They cover host policy, optional local coordination results, and engine layering without direct engine-to-engine awareness.

```rust
pub trait PolicyEngine {
    #[must_use]
    fn compute_profile(
        &self,
        objective: &RoutingObjective,
        inputs: &RoutingPolicyInputs,
    ) -> AdaptiveRoutingProfile;
}

pub trait CommitteeSelector {
    type TopologyView;

    fn select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, RouteError>;
}

pub trait CommitteeCoordinatedEngine {
    type Selector: CommitteeSelector;

    fn committee_selector(&self) -> Option<&Self::Selector>;
}

pub trait SubstratePlanner {
    #[must_use]
    fn candidate_substrates(
        &self,
        requirements: &SubstrateRequirements,
        topology: &Observation<Configuration>,
    ) -> Vec<SubstrateCandidate>;
}

pub trait SubstrateRuntime {
    fn acquire_substrate(
        &mut self,
        candidate: SubstrateCandidate,
    ) -> Result<SubstrateLease, RouteError>;

    fn release_substrate(&mut self, lease: &SubstrateLease) -> Result<(), RouteError>;

    fn observe_substrate_health(
        &self,
        lease: &SubstrateLease,
    ) -> Result<Observation<RouteHealth>, RouteError>;
}

pub trait LayeredRoutingEnginePlanner {
    #[must_use]
    fn candidate_routes_on_substrate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        substrate: &SubstrateLease,
        parameters: &LayerParameters,
    ) -> Vec<RouteCandidate>;

    fn admit_route_on_substrate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        substrate: &SubstrateLease,
        parameters: &LayerParameters,
        candidate: RouteCandidate,
    ) -> Result<RouteAdmission, RouteError>;
}

pub trait LayeredRoutingEngine: RoutingEngine + LayeredRoutingEnginePlanner {
    fn materialize_route_on_substrate(
        &mut self,
        input: RouteMaterializationInput,
        substrate: SubstrateLease,
        parameters: LayerParameters,
    ) -> Result<RouteInstallation, RouteError>;
}

pub trait LayeringPolicyEngine {
    fn activate_layered_route(
        &mut self,
        objective: RoutingObjective,
        outer_engine: RoutingEngineId,
        substrate_requirements: SubstrateRequirements,
        parameters: LayerParameters,
    ) -> Result<MaterializedRoute, RouteError>;
}
```

`PolicyEngine`, `CommitteeSelector`, `CommitteeCoordinatedEngine`, `SubstratePlanner`, and `LayeredRoutingEnginePlanner` are planning or read-only surfaces. `SubstrateRuntime`, `LayeredRoutingEngine`, and `LayeringPolicyEngine` are effectful. `CommitteeSelector` is optional: Jacquard standardizes the `CommitteeSelection` result shape, not one formation algorithm, and selectors may return `None` when no committee applies. Selector implementations may be engine-local, host-local, provisioned, or otherwise out of band. The substrate and layering traits are still forward-looking contract surfaces for host-owned composition.

## Mesh Specialization

`MeshRoutingEngine` is a specialization of the generic routing-engine boundary. It exposes the mesh subcomponents that need to remain independently swappable across crates and runtimes.

```rust
pub trait MeshTopologyModel {
    type PeerEstimate;
    type NeighborhoodEstimate;

    #[must_use]
    fn local_node(&self, local_node_id: &NodeId, configuration: &Configuration) -> Option<Node>;

    #[must_use]
    fn neighboring_nodes(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<(NodeId, Node)>;

    #[must_use]
    fn reachable_endpoints(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<LinkEndpoint>;

    #[must_use]
    fn adjacent_links(&self, local_node_id: &NodeId, configuration: &Configuration) -> Vec<Link>;

    #[must_use]
    fn peer_estimate(
        &self,
        local_node_id: &NodeId,
        peer_node_id: &NodeId,
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> Option<Self::PeerEstimate>;

    #[must_use]
    fn neighborhood_estimate(
        &self,
        local_node_id: &NodeId,
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> Option<Self::NeighborhoodEstimate>;
}

pub trait MeshRoutingEngine: RoutingEngine {
    type TopologyModel: MeshTopologyModel;
    type Transport: MeshTransport;
    type Retention: RetentionStore;

    fn topology_model(&self) -> &Self::TopologyModel;

    fn transport(&self) -> &Self::Transport;

    fn transport_mut(&mut self) -> &mut Self::Transport;

    fn retention_store(&self) -> &Self::Retention;

    fn retention_store_mut(&mut self) -> &mut Self::Retention;
}
```

`MeshTopologyModel` is read-only. `MeshTransport` and `RetentionStore` are effectful. `MeshRoutingEngine` binds one concrete topology model, one transport implementation, and one retention store to a mesh engine instance. This keeps mesh-specific internals swappable without exposing them as shared cross-engine assumptions, while still letting mesh route choice depend on mesh-owned peer and neighborhood estimates behind that boundary.

The associated estimate types are the important boundary here. If a mesh implementation wants novelty scores, reach estimates, bridge heuristics, or neighborhood flow signals, those stay mesh-owned behind `MeshTopologyModel`. They are not promoted into `jacquard-core` as shared `Node`, `Link`, or `Environment` schema.

## Operational Subcomponents

Operational subcomponents support routing engines at the boundary where bytes move or deferred delivery is stored. They are narrow effectful support surfaces for transport I/O and retained payload storage.

```rust
pub trait MeshTransport {
    #[must_use]
    fn transport_id(&self) -> TransportProtocol;

    fn send_frame(&mut self, endpoint: &LinkEndpoint, payload: &[u8])
        -> Result<(), TransportError>;

    fn poll_observations(&mut self) -> Result<Vec<TransportObservation>, TransportError>;
}
```

`MeshTransport` is the carrier boundary for sending frames and reporting transport observations.

```rust
pub trait RetentionStore {
    fn retain_payload(
        &mut self,
        object_id: ContentId<Blake3Digest>,
        payload: Vec<u8>,
    ) -> Result<(), RetentionError>;

    fn take_retained_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, RetentionError>;

    fn contains_retained_payload(
        &self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<bool, RetentionError>;
}
```

`RetentionStore` is the storage boundary for opaque deferred-delivery payloads during partitions.

New transport implementations such as BLE GATT, Wi-Fi LAN, or QUIC implement `MeshTransport` and are registered with the mesh routing engine. If transport implementations grow substantial platform logic, they should move into dedicated crates such as `jacquard-transport-ble`. `RetentionStore` stays intentionally narrow for the same reason.

## Runtime Effects

Runtime effects are the lowest-level extensibility surface in this document. They expose narrow runtime capabilities to pure routing logic. They are useful when a runtime or host needs to swap out how routing code gets time, storage, transport, or route-event logging services without changing the routing logic itself. Hashing is modeled separately as a pure deterministic boundary, not a runtime effect. They do not own route semantics, supervision, or canonical route state.

```rust
pub trait TimeEffects {
    #[must_use]
    fn now_tick(&self) -> Tick;
}

pub trait OrderEffects {
    #[must_use]
    fn next_order_stamp(&mut self) -> OrderStamp;
}

pub trait StorageEffects {
    fn load_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;

    fn store_bytes(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError>;

    fn remove_bytes(&mut self, key: &[u8]) -> Result<(), StorageError>;
}

pub trait RouteEventLogEffects {
    fn record_route_event(
        &mut self,
        event: RouteEventStamped,
    ) -> Result<(), RouteEventLogError>;
}

pub trait TransportEffects {
    fn send_transport(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError>;

    fn poll_transport(&mut self) -> Result<Vec<TransportObservation>, TransportError>;
}

pub trait RoutingRuntimeEffects:
    TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects + TransportEffects
{}
```

Each effect trait covers one concern. `RoutingRuntimeEffects` is the aggregate marker for runtimes that provide the current minimal effect set.
