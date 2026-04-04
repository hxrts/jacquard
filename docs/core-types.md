# Core Types

The `core` crate defines the shared routing vocabulary. It contains only `Pure` data. It does not own runtime loops, network I/O, transport adapters, or family-private behavior.

Identity is split on purpose. `NodeId` identifies one running Contour client. `ControllerId` identifies the cryptographic actor that can authenticate for that node. `NodeBinding` is the explicit bridge between them. `RouteCommitmentId` gives long-lived family and router coordination work its own stable identity instead of overloading route or operation IDs.

Time and ordering are typed. `Tick` models local monotonic time. `DurationMs` models timeout budgets. `OrderStamp` and `RouteOrderingKey` model deterministic ordering. `RouteEpoch` models topology and reconfiguration versioning rather than elapsed time. Field names carry their domain when needed, for example `*_tick`, `*_ms`, and `*_epoch`.

## Route Lifecycle

The route lifecycle is described through `Observed<T>`, `Authoritative<T>`, `RouteCandidate`, `RouteAdmissionCheck`, `RouteAdmission`, `RouteWitness`, `InstalledRoute`, `RouteHandle`, `RouteMaterializationProof`, `RouteLease`, and `RouteTransition`. `RouteCandidate` is observational and advisory. `RouteWitness` is produced only at admission. `RouteHandle` is the strong canonical handle issued only when installation materially succeeds, and `RouteMaterializationProof` is the authoritative publication record that binds that handle back to the witness used for materialization. `RouteSummary.valid_for` and `RouteLease.valid_for` use `TimeWindow` to keep validity explicit instead of spreading ad hoc start and expiry ticks across the model. These objects make route selection, admission, ownership, and mutation explicit. They also keep observational facts, authoritative publications, and family-private runtime state separate.

`RouteCost`, `RouteProgressContract`, `RouteCommitment`, `DeterministicOrderKey<T>`, `RouteEvent`, and `RoutingAuditEvent` carry measurable bounds, ordering rules, coordination state, and replay-visible events into the shared model. `RouteCost.work_step_count_max` is expressed in deterministic abstract work steps. `RouteHealth`, `RouteMaintenanceTrigger`, and `RouteMaintenanceResult` define the narrow cross-family runtime surface for route upkeep without throwing away handoff, replacement, or failure payloads.

## Hashing and Content IDs

`ContentId` identifies immutable routing artifacts. It does not identify live owned route state. Live route state remains under lease and transition control.

The shared model also defines the capability token shapes used by the routing contract. `RouteAdmissionCapability`, `RouteOwnershipCapability`, `RouteEvidenceCapability`, and `RouteTransitionCapability` keep authority classes distinct even before concrete runtime enforcement is added.
