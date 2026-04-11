import Field.Model.API
import Field.Protocol.Bridge
import Field.Quality.API
import Field.Router.Canonical

/- 
The Problem. The field proof stack needs a narrow adequacy-facing API between
Rust-visible runtime artifacts and the reduced Lean protocol/router objects.
This layer must expose just enough structure to talk about traces, evidence,
and router-facing lifecycle projections without letting adequacy become the
owner of canonical route truth.

Solution Structure.
1. Define the reduced runtime artifact vocabulary and admission predicates.
2. Define the protocol-trace and router-facing wrapper functions used
   downstream by adequacy theorems.
3. Package extraction and simulation laws through a small model/laws API.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyAPI

open FieldBoundary
open FieldModelAPI
open FieldProtocolAPI
open FieldRouterCanonical
open FieldRouterLifecycle

/- Projection taxonomy note:

- protocol projection:
  choreography/session structure -> local protocol surface
- local public projection:
  local field semantics -> corridor/public observable surface
- runtime projection / adequacy reduction:
  runtime artifacts or runtime state -> reduced Lean protocol/router/system
  surface

This module owns only the runtime projection / adequacy-reduction side. -/

/-! ## Runtime Artifact Vocabulary -/

/-- Reduced router-facing runtime projection carried by one runtime artifact.
This is still only an extracted observational/runtime view, not a new owner of
router truth. -/
structure RuntimeRouterArtifact where
  lifecycleRoute : LifecycleRoute
  deriving Repr, DecidableEq, BEq

/-- Narrowest Rust-facing round artifact currently worth relating to the Lean
protocol object. This mirrors the controller-relevant fields of
`FieldChoreographyRoundResult`. -/
structure RuntimeRoundArtifact where
  blockedReceive : Option SummaryLabel
  disposition : HostDisposition
  emittedCount : Nat
  stepBudgetRemaining : Nat
  routerArtifact : Option RuntimeRouterArtifact
  deriving Repr, DecidableEq, BEq

def runtimeLifecycleRouteOfArtifact
    (artifact : RuntimeRoundArtifact) : Option LifecycleRoute :=
  artifact.routerArtifact.map RuntimeRouterArtifact.lifecycleRoute

/-- Runtime projection to the public route-shape coordinate carried by one
artifact. -/
def runtimeProjectionShapeOfArtifact
    (artifact : RuntimeRoundArtifact) : Option CorridorShape :=
  (runtimeLifecycleRouteOfArtifact artifact).map fun route => route.candidate.shape

/-- Runtime projection to the public route-support coordinate carried by one
artifact. -/
def runtimeProjectionSupportOfArtifact
    (artifact : RuntimeRoundArtifact) : Option Nat :=
  (runtimeLifecycleRouteOfArtifact artifact).map fun route => route.candidate.support

structure RuntimeProbabilisticSlice where
  disposition : HostDisposition
  blockedReceive : Option SummaryLabel
  emittedCount : Nat
  routeShape : Option CorridorShape
  routeSupport : Option Nat
  deriving Repr, DecidableEq, BEq

def runtimeProbabilisticSliceOfArtifact
    (artifact : RuntimeRoundArtifact) : RuntimeProbabilisticSlice :=
  { disposition := artifact.disposition
    blockedReceive := artifact.blockedReceive
    emittedCount := artifact.emittedCount
    routeShape := runtimeProjectionShapeOfArtifact artifact
    routeSupport := runtimeProjectionSupportOfArtifact artifact }

/-- Admitted router-facing runtime projections must stay inside the current
reduced lifecycle honesty envelope. -/
def RuntimeRouterArtifactAdmitted (artifact : RuntimeRoundArtifact) : Prop :=
  match runtimeLifecycleRouteOfArtifact artifact with
  | none => True
  | some route => LifecycleHonest route

/-- Envelope expected from the Rust private runtime before we claim any
adequacy bridge. -/
def RuntimeArtifactAdmitted (artifact : RuntimeRoundArtifact) : Prop :=
  artifact.stepBudgetRemaining ≤ 8 ∧
    artifact.emittedCount ≤ 8 ∧
    ((artifact.disposition = HostDisposition.complete ∨
        artifact.disposition = HostDisposition.failedClosed) →
        artifact.blockedReceive = none) ∧
    (artifact.disposition = HostDisposition.blocked →
      artifact.blockedReceive.isSome) ∧
    RuntimeRouterArtifactAdmitted artifact

/-- Execution-level admission: every runtime artifact stays inside the reduced
private protocol envelope. -/
def RuntimeExecutionAdmitted
    (artifacts : List RuntimeRoundArtifact) : Prop :=
  ∀ artifact ∈ artifacts, RuntimeArtifactAdmitted artifact

/-- Reduced protocol-trace envelope used by the field adequacy bridge. A trace
stays inside the reduced envelope when all replay-visible semantic objects
remain observational-only. -/
def ProtocolTraceAdmitted (trace : ProtocolTrace) : Prop :=
  ∀ object ∈ traceSemanticObjects trace,
    object.authority = OutputAuthority.observationalOnly

/-! ## Extraction Interface -/

class Model where
  extractSnapshot : RuntimeRoundArtifact → MachineSnapshot
  extractTrace : List RuntimeRoundArtifact → ProtocolTrace
  runtimeEvidence : List RuntimeRoundArtifact → List EvidenceInput

section Wrappers

variable [Model]

def extractSnapshot (artifact : RuntimeRoundArtifact) : MachineSnapshot :=
  Model.extractSnapshot artifact

def extractTrace (artifacts : List RuntimeRoundArtifact) : ProtocolTrace :=
  Model.extractTrace artifacts

def runtimeEvidence (artifacts : List RuntimeRoundArtifact) : List EvidenceInput :=
  Model.runtimeEvidence artifacts

def runtimeLifecycleRoutes
    (artifacts : List RuntimeRoundArtifact) : List LifecycleRoute :=
  artifacts.filterMap runtimeLifecycleRouteOfArtifact

def runtimeCanonicalRoute
    (destination : FieldNetworkAPI.DestinationClass)
    (artifacts : List RuntimeRoundArtifact) : Option LifecycleRoute :=
  canonicalBestRoute destination (runtimeLifecycleRoutes artifacts)

def runtimeCanonicalRouteView
    (destination : FieldNetworkAPI.DestinationClass)
    (artifacts : List RuntimeRoundArtifact) : Option FieldQualityAPI.RouteComparisonView :=
  Option.map FieldQualityAPI.routeComparisonView (runtimeCanonicalRoute destination artifacts)

/-- Minimal simulation relation between a Rust runtime execution artifact list
and a reduced Lean protocol trace. -/
structure RuntimeTraceSimulation
    (artifacts : List RuntimeRoundArtifact) where
  trace : ProtocolTrace
  trace_eq_extract : trace = extractTrace artifacts
  trace_admitted : ProtocolTraceAdmitted trace

end Wrappers

/-! ## Law Interfaces -/

abbrev RuntimeAdmittedImpliesBoundedAndCoherent (M : Model) : Prop :=
  ∀ artifact,
    RuntimeArtifactAdmitted artifact →
      MachineBounded (@Model.extractSnapshot M artifact) ∧
        MachineCoherent (@Model.extractSnapshot M artifact)

abbrev RuntimeEvidenceAgreesWithSemanticTrace (M : Model) : Prop :=
  ∀ artifacts,
    @Model.runtimeEvidence M artifacts =
      controllerEvidenceFromTrace (@Model.extractTrace M artifacts)

/-- Execution-level observational adequacy for extracted traces. -/
abbrev RuntimeExecutionExtractsToObservationalTrace (M : Model) : Prop :=
  ∀ artifacts,
    RuntimeExecutionAdmitted artifacts →
      ∀ object ∈ traceSemanticObjects (@Model.extractTrace M artifacts),
        object.authority = OutputAuthority.observationalOnly

abbrev RuntimeExecutionSimulatesReducedProtocol (M : Model) : Prop :=
  ∀ artifacts,
    RuntimeExecutionAdmitted artifacts →
      Nonempty (@RuntimeTraceSimulation M artifacts)

class Laws extends Model where
  runtime_admitted_implies_bounded_and_coherent :
    RuntimeAdmittedImpliesBoundedAndCoherent toModel
  runtime_evidence_agrees_with_semantic_trace :
    RuntimeEvidenceAgreesWithSemanticTrace toModel
  runtime_execution_extracts_to_observational_trace :
    RuntimeExecutionExtractsToObservationalTrace toModel

instance (priority := 100) lawsToModel [Laws] : Model := Laws.toModel

/-! ## Law Wrappers -/

section LawWrappers

variable [Laws]

theorem runtime_admitted_implies_bounded_and_coherent
    (artifact : RuntimeRoundArtifact)
    (hAdmitted : RuntimeArtifactAdmitted artifact) :
    MachineBounded (extractSnapshot artifact) ∧
      MachineCoherent (extractSnapshot artifact) :=
  Laws.runtime_admitted_implies_bounded_and_coherent artifact hAdmitted

theorem runtime_evidence_agrees_with_semantic_trace
    (artifacts : List RuntimeRoundArtifact) :
    runtimeEvidence artifacts =
      controllerEvidenceFromTrace (extractTrace artifacts) :=
  Laws.runtime_evidence_agrees_with_semantic_trace artifacts

theorem runtime_execution_extracts_to_observational_trace
    (artifacts : List RuntimeRoundArtifact)
    (hAdmitted : RuntimeExecutionAdmitted artifacts) :
    ∀ object ∈ traceSemanticObjects (extractTrace artifacts),
      object.authority = OutputAuthority.observationalOnly :=
  Laws.runtime_execution_extracts_to_observational_trace artifacts hAdmitted

end LawWrappers

end FieldAdequacyAPI
