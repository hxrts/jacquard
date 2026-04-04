# Core Types

The `core` crate defines the shared routing vocabulary. It contains only `Pure` data. It does not own runtime loops, network I/O, transport adapters, or family-private behavior.

Identity is split on purpose. `NodeId` identifies one running Contour client. `ControllerId` identifies the cryptographic actor that can authenticate for that node. `NodeBinding` is the explicit bridge between them.

Time and ordering are typed. `Tick` models local monotonic time. `DurationMs` models timeout budgets. `OrderStamp` and `RouteOrderingKey` model deterministic ordering. `RouteEpoch` models topology and reconfiguration versioning rather than elapsed time.

## Route Lifecycle

The route lifecycle is described through `RouteCandidate`, `RouteAdmissionCheck`, `RouteAdmission`, `RouteWitness`, `InstalledRoute`, `RouteLease`, and `RouteTransition`. These objects make route selection, admission, ownership, and mutation explicit. They also keep canonical route truth separate from family-private runtime state.

`RouteCost` and `RouteProgressContract` carry measurable bounds into the shared model. `RouteHealth`, `RouteMaintenanceTrigger`, and `RouteMaintenanceDisposition` define the narrow cross-family runtime surface for route upkeep.

## Hashing and Content IDs

`ContentId` identifies immutable routing artifacts. It does not identify live owned route state. Live route state remains under lease and transition control.

The shared model also defines the capability token shapes used by the routing contract. `RouteAdmissionCapability`, `RouteOwnershipCapability`, `RouteEvidenceCapability`, and `RouteTransitionCapability` keep authority classes distinct even before concrete runtime enforcement is added.
