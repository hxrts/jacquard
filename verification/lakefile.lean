import Lake
open Lake DSL

/-! # Jacquard Verification

Lake build definition for the Jacquard formal verification library.
Proofs are organized as theorem packs over the routing protocols defined
as Telltale choreographies inside `jacquard-pathway`.

Run `just lean-setup` once from inside `nix develop` to pin the manifest
and fetch prebuilt Mathlib oleans. Mathlib is never rebuilt from source.
-/

package jacquard where
  moreLeanArgs := #[
    "-Dlinter.unusedSectionVars=false",
    "-Dlinter.unusedVariables=false"
  ]

-- Telltale provides the full protocol verification infrastructure:
-- Protocol, Choreography, Semantics, SessionTypes, Distributed.
-- Mathlib and Paco are transitive dependencies through Telltale.
-- Revision is pinned; run `lake update` to advance.
require telltale from git
  "https://github.com/hxrts/telltale" @ "main" / "lean"

/-! ## Theorem Packs

Each module proves properties about one or more of Jacquard's routing
choreographies (defined with `tell!` in `crates/pathway/src/`).
Theorem packs are consumed by `jacquard-simulator` to gate capabilities
and validate scenarios at compile time.
-/

lean_exe hello where
  root := `Hello

/-- Routing invariants: determinism, boundedness, ordered-time separation. -/
@[default_target]
lean_lib RoutingInvariants where
  globs := #[`RoutingInvariants.*]

/-- Time model: Tick monotonicity, epoch versioning, typed time separation. -/
lean_lib TimeModel where
  globs := #[`TimeModel.*]

/-- Protocol correctness: session type properties for pathway choreographies. -/
lean_lib ProtocolCorrectness where
  globs := #[`ProtocolCorrectness.*]
