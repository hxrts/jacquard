import Lake
open Lake DSL

/-! # Jacquard Verification

Lake build definition for the Jacquard formal verification library.
Proofs are organized by subject: routing invariants, time model, and
protocol correctness.
-/

package jacquard where
  moreLeanArgs := #[
    "-Dlinter.unusedSectionVars=false",
    "-Dlinter.unusedVariables=false"
  ]

-- Mathlib provides standard lemmas and automation for proofs.
require mathlib from git
  "https://github.com/leanprover-community/mathlib4" @ "v4.26.0"

/-! ## Libraries -/

/-- Core routing model invariants: determinism, boundedness, ordering. -/
@[default_target]
lean_lib RoutingInvariants where
  globs := #[`RoutingInvariants.*]

/-- Time model: Tick monotonicity, epoch versioning, typed time separation. -/
lean_lib TimeModel where
  globs := #[`TimeModel.*]

/-- Protocol correctness: session type properties for choreography protocols. -/
lean_lib ProtocolCorrectness where
  globs := #[`ProtocolCorrectness.*]
