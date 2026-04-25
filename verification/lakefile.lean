import Lake
open Lake DSL

/-! # Jacquard Verification

Lake build definition for the Jacquard formal verification library.
The active migrated theorem surface now lives in the sibling DualTide
repository. This package keeps a minimal Jacquard verification root for
future non-DualTide proof surfaces.

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
  "https://github.com/hxrts/telltale" @ "026fbdd645895a84a2215f81c857094a479dff77" / "lean"

/-! ## Verification Root

The package is organized by feature, not by placeholder theorem packs.
The built root intentionally excludes the migrated DualTide theorem pack.
-/

/-- Jacquard verification root: feature-organized Lean modules and notes. -/
@[default_target]
lean_lib JacquardVerification where
  roots := #[
    `Verification
  ]
