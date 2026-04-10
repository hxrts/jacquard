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
-- Temporary local override while the GitHub-pinned Lean package is being fixed.
-- Restore the git dependency after the next working Telltale release is cut.
-- require telltale from git
--   "https://github.com/hxrts/telltale" @ "main" / "lean"
require telltale from "../../telltale/lean"

/-! ## Verification Root

The package is organized by feature, not by placeholder theorem packs.
The built roots now include the field theorem-pack modules as well as the
underlying feature modules they re-export.
-/

/-- Jacquard verification root: feature-organized Lean modules and notes. -/
@[default_target]
lean_lib JacquardVerification where
  roots := #[
    `Verification,
    `Field.Field,
    `Field.LocalModel,
    `Field.Information,
    `Field.PrivateProtocol,
    `Field.Network,
    `Field.Router,
    `Field.Quality,
    `Field.Boundary,
    `Field.Adequacy,
    `Field.Assumptions,
    `Field.Model.API,
    `Field.Model.Instance,
    `Field.Model.Decision,
    `Field.Model.Refinement,
    `Field.Model.Boundary,
    `Field.Information.API,
    `Field.Information.Instance,
    `Field.Information.Blindness,
    `Field.Information.Quantitative,
    `Field.Protocol.API,
    `Field.Protocol.Instance,
    `Field.Protocol.Bridge,
    `Field.Protocol.Coherence,
    `Field.Protocol.Conservation,
    `Field.Protocol.ReceiveRefinement,
    `Field.Protocol.Reconfiguration,
    `Field.Network.API,
    `Field.Network.Safety,
    `Field.Router.Publication,
    `Field.Router.Admission,
    `Field.Router.Installation,
    `Field.Router.Lifecycle,
    `Field.Async,
    `Field.Async.API,
    `Field.Async.Safety,
    `Field.Async.Transport,
    `Field.System,
    `Field.System.Statistics,
    `Field.System.Boundary,
    `Field.System.EndToEnd,
    `Field.System.Convergence,
    `Field.Quality.API,
    `Field.Quality.System,
    `Field.Adequacy.API,
    `Field.Adequacy.Instance
  ]
