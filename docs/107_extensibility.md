# Extensibility

This page describes the main extension surfaces in Jacquard. It focuses on the production surfaces that other teams are expected to implement. It does not cover simulator hooks. See [Core Types](102_core_types.md) for the shared model vocabulary and [Routing Logic](105_routing_logic.md) for the route lifecycle that these traits participate in.

## Extension Model

Jacquard is extended at several layers. World extensions add observed objects to the shared world picture. Routing engines consume that world picture and produce route behavior. Policy and coordination traits decide how engines are selected, composed, or locally coordinated. Operational subcomponents and runtime effects support those higher layers.

The ordering matters. A team can extend the world without becoming a routing-engine author. A team can add a routing engine without redefining the world schema. A host can add policy and layering behavior without modifying a routing engine. This separation is the main reason the system composes cleanly across teams.

## World Extensions

World extensions are the entry point for teams that know a specific radio stack, runtime environment, discovery surface, or device class. A world extension adds observed objects to the shared world through self-describing observational surfaces. It does not redefine the shared `Node` or `Link` schema in `jacquard-core`.

```rust
pub trait WorldExtensionDescriptor {
    #[must_use]
    fn extension_id(&self) -> &str;

    #[must_use]
    fn supported_transports(&self) -> Vec<TransportProtocol>;
}

pub trait WorldExtension: WorldExtensionDescriptor {
    fn poll_observations(&mut self) -> Result<Vec<WorldObservation>, RouteError>;
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

`WorldExtensionDescriptor` is pure metadata. All world-extension polling traits are effectful. `WorldObservation` is `Observation<ObservedValue>`, so the payload itself says what was observed. The narrower facet traits exist for teams that only add one part of the world picture.

This is the main cooperative extension surface in Jacquard. One team may add observed BLE nodes. Another may add observed Wi-Fi links. Another may add platform-specific service or transport observations. A host merges those contributions into one world picture above this boundary. Routing engines then consume that merged picture through the shared routing traits.

If a host wants batching, diffs, partial snapshots, merge policy, checkpointing, or prioritization, that happens above these traits. World extensions do not publish canonical route state directly. They contribute observations only.

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
        route: &mut MaterializedRoute,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}
```

`RoutingEnginePlanner` is pure. `RoutingEngine` is effectful. The split keeps candidate production deterministic and keeps runtime mutation inside explicit realization and maintenance methods. The router allocates canonical route identity first. The engine realizes the admitted route under that identity and returns `RouteInstallation`. The final `MaterializedRoute` is assembled above the engine boundary.

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

`PolicyEngine`, `CommitteeSelector`, `SubstratePlanner`, and `LayeredRoutingEnginePlanner` are pure planning or decision surfaces. `SubstrateRuntime`, `LayeredRoutingEngine`, and `LayeringPolicyEngine` are effectful. `CommitteeSelector` is optional. Jacquard commits to the result shape of `CommitteeSelection`. It does not standardize one committee algorithm.

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
