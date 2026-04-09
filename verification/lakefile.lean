import Lake
open Lake DSL

/-! # Jacquard Verification

Lake build definition for the Jacquard formal verification library.
The verification package currently contains one real proof surface:
the field local model, protocol boundary, and parity notes.

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

/-! ## Verification Root

The package is organized by feature, not by placeholder theorem packs.
Today the only built verification root is `Jacquard.Verification`, which
re-exports the field model and protocol boundary modules.
-/

/-- Jacquard verification root: feature-organized Lean modules and notes. -/
@[default_target]
lean_lib JacquardVerification where
  roots := #[
    `Verification,
    `Field.Field,
    `Field.Model.API,
    `Field.Model.Instance,
    `Field.Model.Boundary,
    `Field.Protocol.API,
    `Field.Protocol.Instance
  ]
