# Extensibility

Jacquard is extended through trait implementations. Each extension surface has a defined purity level, a narrow contract, and explicit dependency rules. This page catalogues all extension points, their contracts, and the shared types they consume.

## Hashing and Content Addressing

Hashing is provided through a pure trait so the digest algorithm can be swapped at one boundary.

```rust
pub trait Hashing {
    type Digest: Clone + Eq;

    #[must_use]
    fn hash_bytes(&self, input: &[u8]) -> Self::Digest;

    #[must_use]
    fn hash_tagged(&self, domain: &[u8], input: &[u8]) -> Self::Digest;
}

pub trait ContentAddressable {
    type Digest: Clone + Eq;

    fn canonical_bytes(&self) -> Result<Vec<u8>, ContentEncodingError>;

    #[must_use = "dropping a computed content id usually means the artifact identity was not checked or recorded"]
    fn content_id<H: Hashing<Digest = Self::Digest>>(
        &self,
        hasher: &H,
    ) -> Result<ContentId<Self::Digest>, ContentEncodingError>;
}

pub trait TemplateAddressable {
    type Digest: Clone + Eq;

    fn template_bytes(&self) -> Result<Vec<u8>, ContentEncodingError>;

    #[must_use = "dropping a computed template id usually means the template identity was not checked or recorded"]
    fn template_id<H: Hashing<Digest = Self::Digest>>(
        &self,
        hasher: &H,
    ) -> Result<ContentId<Self::Digest>, ContentEncodingError>;
}
```

`Blake3Hashing` is the default implementation. `ContentAddressable` is for immutable artifacts. `TemplateAddressable` is for partially-bound artifacts whose final identity is not yet resolved.

## Routing Engines

A routing engine is a routing algorithm. Mesh is the first-party engine. External engines such as onion routing plug into the same contract without depending on mesh internals.

A routing engine implements two traits: `RoutingEnginePlanner` for deterministic planning and `RoutingEngine` for effectful runtime behavior. The router interacts with all engines through these traits only.

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

    #[must_use]
    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: &RouteCandidate,
    ) -> Result<RouteAdmissionCheck, RouteError>;

    #[must_use]
    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
    ) -> Result<RouteAdmission, RouteError>;
}

pub trait RoutingEngine: RoutingEnginePlanner {
    #[must_use]
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError>;

    #[must_use]
    fn route_commitments(&self, route: &MaterializedRoute) -> Vec<RouteCommitment>;

    #[must_use]
    fn maintain_route(
        &mut self,
        route: &mut MaterializedRoute,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}
```

`RoutingEnginePlanner` is pure. `RoutingEngine` is effectful. The split keeps candidate production deterministic and testable while route realization owns runtime state mutation. The router allocates the canonical handle and lease first. The routing engine then realizes the admitted route under that identity and returns `RouteInstallation`. The control plane assembles the final `MaterializedRoute` from that router-owned identity plus the engine-owned installation result.

### Dependency Rules

External routing engines should depend on `jacquard-core` and `jacquard-traits`. They should not depend on mesh internals, router internals, or simulator-private helpers.

### Stable Contract Types

An external routing engine must treat these as the stable cross-crate contract:

`RouteSummary`, `Estimate<RouteEstimate>`, `RouteAdmissionCheck`, `RouteWitness`, `RouteHandle`, `RouteLease`, `RouteMaterializationInput`, `RouteInstallation`, `RouteCommitment`, `RouteMaintenanceResult`, `CommitteeSelection`, `SubstrateRequirements`, `SubstrateLease`, `LayerParameters`, `Observation<T>`, and `Fact<T>`.

It must not assume mesh route shape, mesh topology structure, mesh-specific maintenance semantics, or any authority model outside those shared route objects.

## Policy Engines

The policy engine decides how much protection to trade for connectivity. A mesh-only deployment may return a fixed profile. A richer host such as Aura can supply cross-engine policy.

```rust
pub trait PolicyEngine {
    #[must_use]
    fn compute_profile(
        &self,
        objective: &RoutingObjective,
        inputs: &RoutingPolicyInputs,
    ) -> AdaptiveRoutingProfile;
}
```

## Committee Selection

Routing engines that use local coordination can expose committee results through an optional trait. Jacquard commits to the shared result shape, not to one algorithm. Leaderless threshold sets, role-differentiated committees, and no committee at all are valid realizations.

```rust
pub trait CommitteeSelector {
    type TopologyView;

    #[must_use]
    fn select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<CommitteeSelection, RouteError>;
}
```

## Layering

Routing engines that can serve as lower-layer carriers expose substrate planning and runtime traits. Routing engines that can consume a substrate expose layered planning and materialization traits. A host-level policy engine owns the layering decision.

```rust
pub trait SubstratePlanner {
    #[must_use]
    fn candidate_substrates(
        &self,
        requirements: &SubstrateRequirements,
        topology: &Observation<Configuration>,
    ) -> Vec<SubstrateCandidate>;
}

pub trait SubstrateRuntime {
    #[must_use]
    fn acquire_substrate(
        &mut self,
        candidate: SubstrateCandidate,
    ) -> Result<SubstrateLease, RouteError>;

    fn release_substrate(&mut self, lease: &SubstrateLease) -> Result<(), RouteError>;

    #[must_use]
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

    #[must_use]
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
    #[must_use]
    fn materialize_route_on_substrate(
        &mut self,
        input: RouteMaterializationInput,
        substrate: SubstrateLease,
        parameters: LayerParameters,
    ) -> Result<RouteInstallation, RouteError>;
}

pub trait LayeringPolicyEngine {
    #[must_use]
    fn activate_layered_route(
        &mut self,
        objective: RoutingObjective,
        outer_engine: RoutingEngineId,
        substrate_requirements: SubstrateRequirements,
        parameters: LayerParameters,
    ) -> Result<MaterializedRoute, RouteError>;
}
```

`SubstratePlanner` and `LayeredRoutingEnginePlanner` are pure. `SubstrateRuntime`, `LayeredRoutingEngine`, and `LayeringPolicyEngine` are effectful. Neither routing engine needs direct awareness of the other. As with plain route realization, the canonical route handle and lease come from the router or host policy engine, not from the layered routing engine itself, and the final materialized-route record is assembled above the routing-engine boundary.

## Observation Extensions

Jacquard is also extended by observation extensions. This is the boundary for teams that know a specific radio stack, runtime environment, or discovery surface and want to contribute shared observations without becoming a routing-engine author.

The key goal is cooperative interoperability. One extension may contribute local BLE observations, another may contribute Wi-Fi transport observations, and another may contribute platform-specific service observations. The host merges those contributions into one shared model, and routing engines consume that shared model through their existing planning and runtime boundaries.

```rust
pub trait ObservationExtensionDescriptor {
    #[must_use]
    fn extension_id(&self) -> &str;

    #[must_use]
    fn supported_transports(&self) -> Vec<TransportProtocol>;
}

pub trait ObservationExtension: ObservationExtensionDescriptor {
    #[must_use]
    fn poll_observations(&mut self) -> Result<Vec<SharedObservation>, RouteError>;
}
```

`ObservationExtensionDescriptor` is pure metadata. `ObservationExtension` is effectful. `SharedObservation` is just `Observation<ObservedValue>`, so the observed payload itself says what was observed. If a host wants to add batches, diffs, partial snapshots, merge policy, or prioritization, that happens above this trait. Extensions do not publish canonical route state directly, and they do not need direct awareness of one another to have a cooperative effect.

## Transports

A transport is a frame carrier. It sends bytes and reports transport observations. It must not impose sequencing, traffic control, or routing truth.

```rust
pub trait MeshTransport {
    #[must_use]
    fn transport_id(&self) -> TransportProtocol;

    fn send_frame(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError>;

    #[must_use]
    fn poll_observations(&mut self) -> Result<Vec<TransportObservation>, TransportError>;
}
```

Every `MeshTransport` implementation automatically satisfies `TransportEffects` through a blanket impl. New transports such as BLE GATT, Wi-Fi LAN, or QUIC implement `MeshTransport` and are registered with the mesh routing engine.

If transport implementations grow substantial platform logic, they should be split into dedicated crates such as `jacquard-transport-ble` rather than expanding the stub `jacquard-transport` crate.

## Custody Stores

A custody store holds opaque deferred-delivery payloads during partitions. It must not interpret higher-level routing semantics.

```rust
pub trait CustodyStore {
    fn put_custody_payload(
        &mut self,
        object_id: ContentId<Blake3Digest>,
        payload: Vec<u8>,
    ) -> Result<(), CustodyError>;

    #[must_use]
    fn take_custody_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, CustodyError>;

    #[must_use]
    fn contains_custody_payload(
        &self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<bool, CustodyError>;
}
```

## Mesh Subcomponents

The mesh routing engine exposes its internal subcomponents through `MeshRoutingEngine`. This lets topology queries, transport I/O, and custody storage be swapped independently.

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
    fn adjacent_links(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<Link>;
}

pub trait MeshRoutingEngine: RoutingEngine {
    type TopologyModel: MeshTopologyModel;
    type Transport: MeshTransport;
    type Custody: CustodyStore;

    fn topology_model(&self) -> &Self::TopologyModel;
    fn transport(&self) -> &Self::Transport;
    fn transport_mut(&mut self) -> &mut Self::Transport;
    fn custody_store(&self) -> &Self::Custody;
    fn custody_store_mut(&mut self) -> &mut Self::Custody;
}
```

`MeshTopologyModel` is read-only. `MeshTransport` and `CustodyStore` are effectful. The associated types on `MeshRoutingEngine` bind one concrete set of subcomponents per mesh implementation.

## Runtime Effects

The routing core accesses platform capabilities through narrow effect traits. Each trait covers one concern. Alternate implementations enable deterministic testing and simulation without changing routing logic.

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
    #[must_use]
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

    #[must_use]
    fn poll_transport(&mut self) -> Result<Vec<TransportObservation>, TransportError>;
}

pub trait RoutingRuntimeEffects:
    TimeEffects + OrderEffects + HashEffects + StorageEffects + AuditEffects + TransportEffects
{}
```

`RoutingRuntimeEffects` is a blanket trait automatically satisfied when all six effect traits are implemented.

## Simulation

The simulator is extended through scenario description, environment evolution, and harness traits.

```rust
pub trait RoutingScenario {
    fn name(&self) -> &str;
    fn seed(&self) -> u64;
    fn deployment_profile(&self) -> &DeploymentProfile;
    fn initial_configuration(&self) -> &Observation<Configuration>;
    fn objectives(&self) -> &[RoutingObjective];
}

pub trait RoutingEnvironmentModel {
    type EnvironmentArtifact;

    fn advance_environment(
        &self,
        configuration: &Configuration,
        at_tick: Tick,
    ) -> (Observation<Configuration>, Vec<Self::EnvironmentArtifact>);
}

pub trait RoutingSimulator {
    type Scenario: RoutingScenario;
    type EnvironmentModel: RoutingEnvironmentModel;
    type ReplayArtifact;
    type SimulationStats;
    type Error;

    fn run_scenario(
        &mut self,
        scenario: &Self::Scenario,
        environment: &Self::EnvironmentModel,
    ) -> Result<(Self::ReplayArtifact, Self::SimulationStats), Self::Error>;

    fn resume_replay(
        &mut self,
        replay: &Self::ReplayArtifact,
    ) -> Result<(Self::ReplayArtifact, Self::SimulationStats), Self::Error>;
}

pub trait RoutingReplayView {
    type ReplayArtifact;

    fn route_events<'a>(&self, replay: &'a Self::ReplayArtifact) -> &'a [RouteEvent];
    fn audit_events<'a>(&self, replay: &'a Self::ReplayArtifact) -> &'a [RoutingAuditEvent];
}
```

`RoutingScenario` and `RoutingEnvironmentModel` are pure. `RoutingSimulator` is effectful. `RoutingReplayView` is read-only.
