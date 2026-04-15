# Scatter Routing

`jacquard-scatter` is Jacquard's bounded deferred-delivery diffusion engine.
It does not maintain a topology graph.
It does not publish a best next hop.
It does not compute an explicit end-to-end path or corridor envelope.

`scatter` publishes a narrow router claim.
The claim states that an objective is supportable somewhere in the current world model.
The claim is opaque, partition-tolerant, and hold-capable.
After materialization, the engine moves data through engine-private transport packets under the standard `RoutingEngine` and `RouterManagedEngine` boundary.

See [Crate Architecture](999_crate_architecture.md) for the shared ownership and boundary rules that constrain this engine.

## Core Model

The first in-tree `scatter` implementation keeps a small deterministic model.
The engine retains messages, summarizes peer observations, and tracks per-route progress.
It does not assume stable endpoint identity beyond the router objective vocabulary already present in Jacquard.

- payloads carry a stable local message id
- expiry is local and typed through `created_tick` plus bounded `DurationMs`
- replication is bounded by hard copy budgets
- forwarding is local and opportunistic
- handoff is preferential rather than ack-driven custody transfer
- published route shape visibility is `Opaque`

## Policy Surface

`ScatterEngineConfig` defines the deterministic policy surface for the engine.
It keeps the behavior-critical constants named and typed.
It avoids anonymous literals in the runtime.

- `ScatterExpiryPolicy`
- `ScatterBudgetPolicy`
- `ScatterRegimeThresholds`
- `ScatterDecisionThresholds`
- `ScatterTransportPolicy`
- `ScatterOperationalBounds`

These policies cover message lifetime, replication budgets, regime detection, carrier thresholds, contact feasibility, and bounded runtime work.

## Route Lifecycle

Planner behavior is conservative.
`candidate_routes` emits at most one candidate for a supportable objective.
The router remains the owner of canonical route truth.

1. the planner confirms that the destination or service objective is supportable in the current observation
2. the router admits an opaque and partition-tolerant `scatter` claim
3. the runtime materializes a route-local progress surface
4. payloads are retained, carried, replicated, or handed off according to local regime and peer score
5. maintenance can report hold-fallback viability even when no direct next hop exists

This split keeps route publication router-owned.
It lets `scatter` own its private deferred-delivery mechanics.

## Transport Boundary

`scatter` follows the standard Jacquard ownership split.
The engine consumes explicit `TransportObservation`.
The engine sends only through `TransportSenderEffects`.
The engine does not own async transport streams or assign `Tick`.

Host bridges own ingress draining and time attachment.
Transport choice stays a local contact-feasibility judgment.
The first implementation keeps that judgment reduced and deterministic.
It does not build separate routing models per transport.

## Contrast With Other Engines

- `batman-bellman`, `batman-classic`, `babel`, and `olsrv2` retain routing control state but do not buffer payloads for deferred delivery
- `pathway` supports deferred delivery through explicit path and retention boundaries plus full-route search
- `field` carries forward bounded routing and service evidence rather than general payload custody
- `scatter` is the in-tree opaque deferred-delivery baseline, so payload custody stays local, bounded, and diffusion-oriented

## Current Non-Goals

The current engine does not attempt to provide a full DTN control plane.
It keeps the surface intentionally narrow.

- topology reconstruction
- stable semantic identity routing beyond Jacquard objectives
- ack-driven authoritative custody transfer
- multipath planning
- distributed time agreement
- remote-clock freshness claims
