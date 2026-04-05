# Extensibility

This page describes the main extension surfaces in Jacquard. It focuses on the production surfaces that other teams are expected to implement. It does not cover simulator hooks. See [Core Types](102_core_types.md) for the shared model vocabulary and [Routing Logic](105_routing_logic.md) for the route lifecycle that these traits participate in.

## Extension Model

Jacquard is extended at several layers. World extensions add observed objects to the shared world picture. Routing engines consume that world picture and produce route behavior. Policy and coordination traits decide how engines are selected, composed, or locally coordinated. Operational subcomponents and runtime effects support those higher layers.

The ordering matters. A team can extend the world without becoming a routing-engine author. A team can add a routing engine without redefining the world schema. A host can add policy and layering behavior without modifying a routing engine. This separation is the main reason the system composes cleanly across teams.

## World Extensions

World extensions are the entry point for teams that know a specific radio stack, runtime environment, discovery surface, or device class. The key idea is simple. Jacquard has one shared world schema in `jacquard-core`. A world extension adds observations of that schema. It does not define a private alternative node or link type.

### Shared World Schema

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

pub struct Environment {
    pub reachable_neighbor_count: u32,
    pub churn_permille: RatioPermille,
    pub contention_permille: RatioPermille,
}
```

This is the shared world schema that every world extension targets. A developer who wants to add a new observed node or link needs to construct these exact objects. The extension point is not schema definition. The extension point is emitting observations of these shared objects.

### Schema-Bound Observation Surfaces

```rust
pub type NodeObservation = Observation<Node>;
pub type LinkObservation = Observation<Link>;
pub type EnvironmentObservation = Observation<Environment>;
pub type ServiceObservation = Observation<ServiceDescriptor>;

pub enum ObservedValue {
    Node(Node),
    Link(Link),
    Environment(Environment),
    Service(ServiceDescriptor),
    Transport(TransportObservation),
}

pub type WorldObservation = Observation<ObservedValue>;
```

These aliases bind the shared world schema directly into the extension surface. `NodeObservation` is `Observation<Node>`. `LinkObservation` is `Observation<Link>`. The concrete `Node`, `Link`, and `Environment` schema is therefore already part of the trait contract before any trait method is shown.

`WorldObservation` is the umbrella surface. `ObservedValue` makes that umbrella form self-describing. A host can consume the narrow forms when it wants object-specific handling and can consume the umbrella form when it wants one uniform world-observation stream.

### Schema-Bound Facet Traits

```rust
pub trait WorldExtensionDescriptor {
    #[must_use]
    fn extension_id(&self) -> &str;

    #[must_use]
    fn supported_transports(&self) -> Vec<TransportProtocol>;
}

pub trait NodeWorldExtension: WorldExtensionDescriptor {
    fn poll_node_observations(&mut self) -> Result<Vec<NodeObservation>, RouteError>;
}

pub trait LinkWorldExtension: WorldExtensionDescriptor {
    fn poll_link_observations(&mut self) -> Result<Vec<LinkObservation>, RouteError>;
}

pub trait EnvironmentWorldExtension: WorldExtensionDescriptor {
    fn poll_environment_observations(&mut self) -> Result<Vec<EnvironmentObservation>, RouteError>;
}

pub trait ServiceWorldExtension: WorldExtensionDescriptor {
    fn poll_service_observations(&mut self) -> Result<Vec<ServiceObservation>, RouteError>;
}

pub trait TransportWorldExtension: WorldExtensionDescriptor {
    fn poll_transport_observations(
        &mut self,
    ) -> Result<Vec<Observation<TransportObservation>>, RouteError>;
}
```

These facet traits are already schema-bound. `NodeWorldExtension` is not a generic hook for inventing a new node model. It is a concrete contract that returns `Vec<NodeObservation>`, and `NodeObservation` is `Observation<Node>`. The same rule applies to links, environment, services, and transport observations.

### Umbrella World Extension

```rust
pub trait WorldExtension: WorldExtensionDescriptor {
    fn poll_observations(&mut self) -> Result<Vec<WorldObservation>, RouteError>;
}
```

`WorldExtensionDescriptor` is pure metadata. All world-extension polling traits are effectful. A team may implement the umbrella trait, one or more narrow facet traits, or both. A host may later batch, diff, merge, checkpoint, or prioritize these observations above this boundary.

This is the main cooperative extension surface in Jacquard. One team may add observed BLE nodes. Another may add observed Wi-Fi links. Another may add platform-specific service or transport observations. A host merges those contributions into one world picture. Routing engines then consume that merged picture through the shared routing traits.

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
    ) -> Result<RouteAdmissionCheck, RouteError>;

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
    ) -> Result<RouteAdmission, RouteError>;
}

pub trait RoutingEngine: RoutingEnginePlanner {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError>;

    fn route_commitments(&self, route: &MaterializedRoute) -> Vec<RouteCommitment>;

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

External routing engines should depend on `jacquard-core` and `jacquard-traits`. They should not depend on mesh internals, router internals, or simulator-private helpers. The stable shared contract includes `RouteSummary`, `Estimate<RouteEstimate>`, `RouteAdmissionCheck`, `RouteWitness`, `RouteHandle`, `RouteLease`, `RouteMaterializationInput`, `RouteInstallation`, `RouteCommitment`, `RouteMaintenanceResult`, `CommitteeSelection`, `SubstrateRequirements`, `SubstrateLease`, `LayerParameters`, `Observation<T>`, and `Fact<T>`. External engines must not assume mesh route shape, mesh topology structure, mesh-specific maintenance semantics, or any authority model outside those shared route objects.

## Policy And Coordination

Policy and coordination traits sit above or beside routing engines. They do not redefine route ownership. They decide how a host computes adaptive policy, how an engine may expose local coordination results, and how engines may be layered without direct engine-to-engine awareness.

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
    ) -> Result<CommitteeSelection, RouteError>;
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

`PolicyEngine`, `CommitteeSelector`, `SubstratePlanner`, and `LayeredRoutingEnginePlanner` are pure planning or decision surfaces. `SubstrateRuntime`, `LayeredRoutingEngine`, and `LayeringPolicyEngine` are effectful. `CommitteeSelector` is optional. Jacquard commits to the result shape of `CommitteeSelection`. It does not standardize one committee algorithm. The substrate and layering traits are still forward-looking contract surfaces. They exist so host-owned composition can stabilize at the type boundary now, but the current in-tree coverage is still contract-oriented rather than a mature production layering stack.

Layering follows the same ownership rule as ordinary route realization. The canonical route handle and lease come from the router or host policy layer. A layered routing engine does not allocate canonical route identity for itself. The lower engine exposes substrate capabilities and leases. The upper engine consumes them through the shared substrate contract.

## Operational Subcomponents

Operational subcomponents support routing engines at the boundary where bytes move or deferred delivery is stored. These are not top-level semantic extensions. They are narrow effectful support surfaces.

```rust
pub trait MeshTransport {
    #[must_use]
    fn transport_id(&self) -> TransportProtocol;

    fn send_frame(&mut self, endpoint: &LinkEndpoint, payload: &[u8])
        -> Result<(), TransportError>;

    fn poll_observations(&mut self) -> Result<Vec<TransportObservation>, TransportError>;
}

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

`MeshTransport` is a frame carrier. It sends bytes and reports transport observations. It must not impose sequencing, traffic control, or routing truth. `RetentionStore` holds opaque deferred-delivery payloads during partitions. It must not interpret higher-level routing semantics.

New transport implementations such as BLE GATT, Wi-Fi LAN, or QUIC implement `MeshTransport` and are registered with the mesh routing engine. If transport implementations grow substantial platform logic, they should move into dedicated crates such as `jacquard-transport-ble`. `RetentionStore` stays intentionally narrow for the same reason.

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
        configuration: &Configuration,
    ) -> Option<Self::PeerEstimate>;

    #[must_use]
    fn neighborhood_estimate(
        &self,
        local_node_id: &NodeId,
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

`MeshTopologyModel` is read-only. `MeshTransport` and `RetentionStore` are effectful. `MeshRoutingEngine` binds one concrete topology model, one transport implementation, and one retention store to a mesh engine instance. This keeps mesh-specific internals swappable without exposing them as shared cross-engine assumptions.

The associated estimate types are the important boundary here. If a mesh implementation wants novelty scores, reach estimates, bridge heuristics, or neighborhood flow signals, those stay mesh-owned behind `MeshTopologyModel`. They are not promoted into `jacquard-core` as shared `Node`, `Link`, or `Environment` schema.

## Runtime Effects

Runtime effects are the lowest-level extensibility surface in this document. They expose narrow runtime capabilities to pure routing logic. They do not own route semantics, supervision, or canonical route state.

```rust
pub trait TimeEffects {
    #[must_use]
    fn now_tick(&self) -> Tick;
}

pub trait OrderEffects {
    #[must_use]
    fn next_order_stamp(&mut self) -> OrderStamp;
}

pub trait HashEffects {
    #[must_use]
    fn hash_bytes(&self, input: &[u8]) -> Blake3Digest;

    #[must_use]
    fn hash_tagged(&self, domain: &[u8], input: &[u8]) -> Blake3Digest;
}

pub trait StorageEffects {
    fn load_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;

    fn store_bytes(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError>;

    fn remove_bytes(&mut self, key: &[u8]) -> Result<(), StorageError>;
}

pub trait AuditEffects {
    fn emit_audit(&mut self, event: RoutingAuditEvent) -> Result<(), AuditError>;
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
    TimeEffects + OrderEffects + HashEffects + StorageEffects + AuditEffects + TransportEffects
{}
```

Each effect trait covers one concern. `RoutingRuntimeEffects` is the aggregate marker for runtimes that provide the current minimal effect set. This surface is lower-level than world extensions, routing engines, or policy traits. It exists so native execution, tests, and deterministic replay can share one routing model without sharing one concrete runtime.
