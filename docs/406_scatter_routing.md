# Scatter Routing

`jacquard-scatter` is Jacquard's bounded deferred-delivery diffusion engine.
Unlike the proactive next-hop engines, it does not maintain a topology graph or
publish a best next hop. Unlike `pathway`, it does not compute an explicit
end-to-end path. Unlike `field`, it does not publish a corridor envelope.

Its public router claim is deliberately narrow: `scatter` can publish an
opaque, partition-tolerant, hold-capable route viability claim for an objective
that is supportable somewhere in the current world model. After the router
materializes that claim, the engine performs bounded store-carry-forward data
movement through engine-private transport packets under the standard
`RoutingEngine` / `RouterManagedEngine` boundary.

## Core Model

The first in-tree `scatter` implementation is intentionally small:

- payloads are tagged with a stable local message id
- expiry is local and typed: `created_tick` plus bounded `DurationMs`
- replication is bounded by hard copy budgets
- forwarding is local and opportunistic only
- handoff is preferential, not ack-driven custody transfer
- route shape visibility is `Opaque`

The engine keeps a bounded retained-message store, peer observation summaries,
and per-route progress. It does not assume stable endpoint identity beyond the
router objective vocabulary already present in Jacquard.

## Policy Surface

`ScatterEngineConfig` centralizes the first deterministic policy surface:

- `ScatterExpiryPolicy`
- `ScatterBudgetPolicy`
- `ScatterRegimeThresholds`
- `ScatterDecisionThresholds`
- `ScatterTransportPolicy`
- `ScatterOperationalBounds`

These cover message lifetime, replication budgets, regime detection,
scope-relative carrier thresholds, contact-feasibility limits, and bounded
runtime storage/work ceilings. All behavior-critical constants are named and
typed rather than buried as anonymous literals.

## Route Lifecycle

Planner behavior is conservative. `candidate_routes` emits at most one
candidate for a supportable objective. The admission/runtime story is:

1. planner confirms the destination or service objective is supportable in the
   current topology observation
2. router admits an opaque, partition-tolerant Scatter claim
3. runtime materializes a route-local progress surface
4. payloads entering that route are retained, carried, replicated, or
   preferentially handed off according to local regime and peer score
5. maintenance can report hold-fallback viability even when no direct next hop
   exists

That keeps canonical route truth router-owned while allowing the engine to own
its private deferred-delivery mechanics.

## Transport Boundary

Scatter obeys the standard Jacquard ownership split:

- host bridges own ingress draining and time attachment
- the engine consumes explicit `TransportObservation`
- the engine sends only through `TransportSenderEffects`
- the engine does not own async transport streams or assign `Tick`

Transport choice is a local contact-feasibility judgment. The first
implementation keeps that judgment reduced and deterministic rather than
building separate routing models per transport.

## Contrast With Other Engines

- `batman-bellman`, `batman-classic`, `babel`, and `olsrv2` retain routing
  control state but do not buffer payloads for deferred delivery
- `pathway` supports deferred delivery through explicit path / retention
  boundaries and full-route search
- `field` carries forward bounded routing and service evidence, not general
  payload custody
- `scatter` is the in-tree opaque deferred-delivery baseline: payload custody
  is local, bounded, and diffusion-oriented rather than path- or
  corridor-oriented

## Current Non-Goals

The current engine does not attempt to provide:

- topology reconstruction
- stable semantic identity routing beyond Jacquard objectives
- ack-driven authoritative custody transfer
- multipath planning
- distributed time agreement or remote-clock freshness claims

The point of `scatter` is to be a true router engine with bounded local-only
deferred delivery, not to become a full DTN control plane.
