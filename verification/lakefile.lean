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
  "https://github.com/hxrts/telltale" @ "026fbdd645895a84a2215f81c857094a479dff77" / "lean"

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
    `Field.Architecture,
    `Field.CostAPI,
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
    `Field.AssumptionCore,
    `Field.AssumptionTheorems,
    `Field.Model.API,
    `Field.Model.Instance,
    `Field.Model.Decision,
    `Field.Model.Refinement,
    `Field.Model.Boundary,
    `Field.Information.API,
    `Field.Information.Instance,
    `Field.Information.Probabilistic,
    `Field.Information.Bayesian,
    `Field.Information.Calibration,
    `Field.Information.Blindness,
    `Field.Information.Quantitative,
    `Field.Protocol.API,
    `Field.Protocol.Boundary,
    `Field.Protocol.Instance,
    `Field.Protocol.Bridge,
    `Field.Protocol.Coherence,
    `Field.Protocol.Conservation,
    `Field.Protocol.Fixtures,
    `Field.Protocol.Closure,
    `Field.Protocol.ReceiveRefinement,
    `Field.Protocol.Reconfiguration,
    `Field.Network.API,
    `Field.Network.Safety,
    `Field.Router.Publication,
    `Field.Router.Admission,
    `Field.Router.Installation,
    `Field.Router.Lifecycle,
    `Field.Router.Selector,
    `Field.Router.Canonical,
    `Field.Router.CanonicalStrong,
    `Field.Router.Cost,
    `Field.Router.Optimality,
    `Field.Router.Probabilistic,
    `Field.Router.Resilience,
    `Field.Search,
    `Field.Search.API,
    `Field.Async,
    `Field.Async.API,
    `Field.Async.Safety,
    `Field.Async.Transport,
    `Field.Retention,
    `Field.Retention.API,
    `Field.Retention.Instance,
    `Field.Retention.Refinement,
    `Field.Retention.Fixtures,
    `Field.System,
    `Field.System.Statistics,
    `Field.System.Boundary,
    `Field.System.EndToEnd,
    `Field.System.Convergence,
    `Field.System.Canonical,
    `Field.System.CanonicalStrong,
    `Field.System.Cost,
    `Field.System.Probabilistic,
    `Field.System.Calibration,
    `Field.System.Optimality,
    `Field.System.Resilience,
    `Field.System.Retention,
    `Field.Quality.API,
    `Field.Quality.Reference,
    `Field.Quality.Refinement,
    `Field.Quality.System,
    `Field.Adequacy.API,
    `Field.Adequacy.Canonical,
    `Field.Adequacy.Cost,
    `Field.Adequacy.Projection,
    `Field.Adequacy.Probabilistic,
    `Field.Adequacy.ProbabilisticFixtures,
    `Field.Adequacy.ReplayFixtures,
    `Field.Adequacy.Search,
    `Field.Adequacy.Instance
  ]
