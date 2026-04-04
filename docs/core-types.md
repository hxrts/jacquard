# Core Types

The `core` crate defines the shared routing vocabulary. It contains only `Pure` data. It does not own runtime loops, network I/O, transport adapters, or family-private behavior.

Identity is split on purpose. `NodeId` identifies one running Contour client. `ControllerId` identifies the cryptographic actor that can authenticate for that node. `NodeBinding` is the explicit bridge between them.

Time and ordering are typed. `Tick` models local monotonic time. `DurationMs` models timeout budgets. `OrderStamp` and `RouteOrderingKey` model deterministic ordering. `RouteEpoch` models topology and reconfiguration versioning rather than elapsed time. Field names carry their domain when needed, for example `*_tick`, `*_ms`, and `*_epoch`.

## Route Lifecycle

The route lifecycle is described through `RouteCandidate`, `RouteAdmissionCheck`, `RouteAdmission`, `RouteWitness`, `InstalledRoute`, `RouteHandle`, `RouteLease`, and `RouteTransition`. `RouteCandidate` is observational and advisory. `RouteWitness` is produced only at admission. `RouteHandle` is the strong canonical handle issued only when installation materially succeeds. `RouteSummary.valid_for` and `RouteLease.valid_for` use `TimeWindow` to keep validity explicit instead of spreading ad hoc start and expiry ticks across the model. These objects make route selection, admission, ownership, and mutation explicit. They also keep canonical route truth separate from family-private runtime state.

`RouteCost`, `RouteProgressContract`, and `RouteCommitment` carry measurable bounds and commitment state into the shared model. `RouteCost.work_step_count_max` is expressed in deterministic abstract work steps. `RouteHealth`, `RouteMaintenanceTrigger`, and `RouteMaintenanceResult` define the narrow cross-family runtime surface for route upkeep without throwing away handoff, replacement, or failure payloads.

## Hashing and Content IDs

`ContentId` identifies immutable routing artifacts. It does not identify live owned route state. Live route state remains under lease and transition control.

The shared model also defines the capability token shapes used by the routing contract. `RouteAdmissionCapability`, `RouteOwnershipCapability`, `RouteEvidenceCapability`, and `RouteTransitionCapability` keep authority classes distinct even before concrete runtime enforcement is added.
