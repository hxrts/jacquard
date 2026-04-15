import Field.Adequacy.Refinement
import Field.Protocol.Reconfiguration
import Field.Quality.System
import Field.Search.API
import Field.Model.Boundary

/- 
The Problem. The adequacy layer already relates reduced runtime artifacts to
protocol traces and router/system views, and the search layer already has a
reduced proof-facing search object. The remaining closure gap is a single
adequacy object that carries both reduced runtime state and reduced search
projection together.

Solution Structure.
1. Define the reduced search projection extracted from the field search
   boundary.
2. Define the reduced runtime-search adequacy object and its admission
   predicate.
3. Prove the basic projection, trace, and canonical-route consequences of that
   combined adequacy object.
-/

/-!
 # FieldAdequacySearch

Search-aware adequacy closure for the reduced field runtime/search boundary.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacySearch

open FieldAdequacyAPI
open FieldAdequacyRefinement
open FieldAdequacyRuntime
open FieldModelAPI
open FieldNetworkAPI
open FieldProtocolAPI
open FieldProtocolReconfiguration
open FieldQualityAPI
open FieldQualitySystem
open FieldRouterSelector
open FieldSearchAPI
open FieldSystemCanonical
open FieldSystemEndToEnd
open FieldBoundary

/-! ## Reduced Search Projection -/

structure SearchProjection where
  surface : SearchSurface
  deriving Inhabited, Repr, DecidableEq, BEq

def SearchProjectionAdmitted
    (projection : SearchProjection) : Prop :=
  projection.surface.query.kind = objectiveMeaning projection.surface ∧
    bootstrapConservative projection.surface ∧
    degradedSteadyConservative projection.surface ∧
    promotionConservative projection.surface ∧
    match projection.surface.reconfiguration with
    | none => True
    | some step => step.toEpoch = projection.surface.snapshot

def selectedResultProjection
    (projection : SearchProjection) : Option SelectedResult :=
  projection.surface.selectedResult

def selectedWitnessProjection
    (projection : SearchProjection) : Option (List NodeId) :=
  selectedWitness projection.surface

def snapshotEpochProjection
    (projection : SearchProjection) : SearchSnapshotEpoch :=
  projection.surface.snapshot

def executionPolicyProjection
    (projection : SearchProjection) : ExecutionPolicy :=
  projection.surface.executionPolicy

def queryProjection
    (projection : SearchProjection) : SearchQuery :=
  projection.surface.query

def reconfigurationProjection
    (projection : SearchProjection) : Option EpochReconfiguration :=
  projection.surface.reconfiguration

/-! ## Search-Aware Runtime Adequacy Object -/

structure RustReplayExtraction where
  runtimeArtifacts : List RuntimeRoundArtifact
  search : Option SearchProjection
  protocolReconfiguration : Option ReducedReconfiguration
  deriving Repr, DecidableEq, BEq

structure RuntimeSearchState where
  runtimeState : RuntimeState
  search : SearchProjection
  protocolReconfiguration : Option ReducedReconfiguration
  deriving Repr, DecidableEq, BEq

def RustReplayExtractionAdmitted
    (replay : RustReplayExtraction) : Prop :=
  RuntimeExecutionAdmitted replay.runtimeArtifacts ∧
    match replay.search with
    | none => True
    | some projection => SearchProjectionAdmitted projection
  ∧ match replay.protocolReconfiguration with
    | none => True
    | some step => ReconfigurationAdmitted step

def runtimeSearchStateOfReplay
    (replay : RustReplayExtraction) : Option RuntimeSearchState :=
  replay.search.map fun search =>
    { runtimeState := initialRuntimeState replay.runtimeArtifacts
      search := search
      protocolReconfiguration := replay.protocolReconfiguration }

def protocolReconfigurationProjection
    (bundle : RuntimeSearchState) : Option ReducedReconfiguration :=
  bundle.protocolReconfiguration

def RuntimeSearchStateAdmitted
    (bundle : RuntimeSearchState) : Prop :=
  RuntimeStateAdmitted bundle.runtimeState ∧
    SearchProjectionAdmitted bundle.search ∧
      match bundle.protocolReconfiguration with
      | none => True
      | some step => ReconfigurationAdmitted step

def ReducedRuntimeSearchAdequacy
    (bundle : RuntimeSearchState)
    (state : EndToEndState) : Prop :=
  RuntimeSearchStateAdmitted bundle ∧
    RuntimeStateProjectsSystemState bundle.runtimeState state

def extractTraceOfBundle
    (bundle : RuntimeSearchState) : ProtocolTrace :=
  extractTraceOfState bundle.runtimeState

def runtimeEvidenceOfBundle
    (bundle : RuntimeSearchState) : List EvidenceInput :=
  runtimeEvidenceOfState bundle.runtimeState

def runtimeCanonicalRouteOfBundle
    (destination : DestinationClass)
    (bundle : RuntimeSearchState) : Option FieldRouterLifecycle.LifecycleRoute :=
  runtimeCanonicalRoute destination (runtimeArtifactsOfState bundle.runtimeState)

def runtimeCanonicalRouteViewOfBundle
    (destination : DestinationClass)
    (bundle : RuntimeSearchState) : Option RouteComparisonView :=
  runtimeCanonicalRouteView destination (runtimeArtifactsOfState bundle.runtimeState)

/-! ## Search Projection Fixtures -/

def exactNodeSearchProjection : SearchProjection :=
  { surface :=
      { objective := .node
        query :=
          { start := .alpha
            kind := .singleGoal
            acceptedGoals := [.beta] }
        bootstrapClass := .steady
        continuityBand := .steady
        bootstrapDecision := some .promote
        promotionBlocker := none
        executionPolicy :=
          { scheduler := .canonicalSerial
            batchWidth := 1
            exact := true
            runToCompletion := true }
        selectedResult := some
          { witness := [.alpha, .beta]
            selectedNeighbor := some .beta }
        snapshot := { routeEpoch := 3, snapshotId := 11 }
        reconfiguration := none } }

def candidateSetSearchProjection : SearchProjection :=
  { surface :=
      { objective := .service
        query :=
          { start := .alpha
            kind := .candidateSet
            acceptedGoals := [.beta, .gamma] }
        bootstrapClass := .steady
        continuityBand := .degradedSteady
        bootstrapDecision := some .promote
        promotionBlocker := none
        executionPolicy :=
          { scheduler := .threadedExactSingleLane
            batchWidth := 2
            exact := true
            runToCompletion := true }
        selectedResult := some
          { witness := [.alpha, .gamma]
            selectedNeighbor := some .gamma }
        snapshot := { routeEpoch := 3, snapshotId := 12 }
        reconfiguration := some
          { fromEpoch := { routeEpoch := 3, snapshotId := 11 }
            toEpoch := { routeEpoch := 3, snapshotId := 12 }
            reseeding := .preserveOpenAndIncons } } }

theorem exact_node_search_projection_admitted :
    SearchProjectionAdmitted exactNodeSearchProjection := by
  simp [SearchProjectionAdmitted, exactNodeSearchProjection, objectiveMeaning,
    queryKindOfObjective, bootstrapConservative, degradedSteadyConservative,
    promotionConservative]

theorem candidate_set_search_projection_admitted :
    SearchProjectionAdmitted candidateSetSearchProjection := by
  simp [SearchProjectionAdmitted, candidateSetSearchProjection, objectiveMeaning,
    queryKindOfObjective, bootstrapConservative, degradedSteadyConservative,
    promotionConservative]

theorem admitted_replay_extraction_with_search_yields_admitted_bundle
    (replay : RustReplayExtraction)
    (projection : SearchProjection)
    (hAdmitted : RustReplayExtractionAdmitted replay)
    (hSearch : replay.search = some projection) :
    RuntimeSearchStateAdmitted
      { runtimeState := initialRuntimeState replay.runtimeArtifacts
        search := projection
        protocolReconfiguration := replay.protocolReconfiguration } := by
  simp [RustReplayExtractionAdmitted, RuntimeSearchStateAdmitted, hSearch] at hAdmitted ⊢
  exact ⟨initialRuntimeState_admitted replay.runtimeArtifacts hAdmitted.1,
    hAdmitted.2.1, hAdmitted.2.2⟩

/-! ## Projection And Preservation Theorems -/

theorem selected_result_projection_eq_surface
    (projection : SearchProjection) :
    selectedResultProjection projection = projection.surface.selectedResult := by
  rfl

theorem snapshot_epoch_projection_eq_surface
    (projection : SearchProjection) :
    snapshotEpochProjection projection = projection.surface.snapshot := by
  rfl

theorem execution_policy_projection_eq_surface
    (projection : SearchProjection) :
    executionPolicyProjection projection = projection.surface.executionPolicy := by
  rfl

theorem query_projection_eq_surface
    (projection : SearchProjection) :
    queryProjection projection = projection.surface.query := by
  rfl

theorem selected_witness_projection_stable_of_same_selected_result
    {left right : SearchProjection}
    (hSelected :
      selectedResultProjection left = selectedResultProjection right) :
    selectedWitnessProjection left = selectedWitnessProjection right := by
  exact selected_witness_stable_of_same_selected_result hSelected

theorem admitted_search_projection_uses_objective_query_kind
    (projection : SearchProjection)
    (hAdmitted : SearchProjectionAdmitted projection) :
    (queryProjection projection).kind = objectiveMeaning projection.surface := by
  exact hAdmitted.1

theorem admitted_reconfiguration_projection_targets_current_snapshot
    (projection : SearchProjection)
    (step : EpochReconfiguration)
    (hAdmitted : SearchProjectionAdmitted projection)
    (hStep : reconfigurationProjection projection = some step) :
    step.toEpoch = snapshotEpochProjection projection := by
  cases hReconf : projection.surface.reconfiguration with
  | none =>
      simp [reconfigurationProjection, hReconf] at hStep
  | some actualStep =>
      simp [SearchProjectionAdmitted, reconfigurationProjection, hReconf] at hAdmitted hStep
      cases hStep
      exact hAdmitted.2.2.2.2

theorem selector_truth_is_policy_invariant
    (semantics : LifecycleSelectorSemantics)
    (leftPolicy rightPolicy : SearchExecutionPolicy) :
    (withExecutionPolicy semantics leftPolicy).semantics =
      (withExecutionPolicy semantics rightPolicy).semantics := by
  exact
    FieldSearchAPI.execution_policy_changes_do_not_change_selector_truth
      semantics leftPolicy rightPolicy

theorem admitted_runtime_search_state_extracts_to_observational_trace
    (bundle : RuntimeSearchState)
    (hAdmitted : RuntimeSearchStateAdmitted bundle) :
    ProtocolTraceAdmitted (extractTraceOfBundle bundle) := by
  exact
    admitted_runtime_state_extracts_to_observational_trace
      bundle.runtimeState hAdmitted.1

theorem runtime_search_state_evidence_agrees_with_semantic_trace
    (bundle : RuntimeSearchState) :
    runtimeEvidenceOfBundle bundle =
      controllerEvidenceFromTrace (extractTraceOfBundle bundle) := by
  exact runtime_state_evidence_agrees_with_semantic_trace bundle.runtimeState

theorem admitted_protocol_reconfiguration_of_bundle_is_observational_only
    (bundle : RuntimeSearchState)
    (step : ReducedReconfiguration)
    (hAdmitted : RuntimeSearchStateAdmitted bundle)
    (hStep : protocolReconfigurationProjection bundle = some step) :
    ReconfigurationObservationalOnly step := by
  cases hReconf : bundle.protocolReconfiguration with
  | none =>
      simp [protocolReconfigurationProjection, hReconf] at hStep
  | some actualStep =>
      simp [RuntimeSearchStateAdmitted, protocolReconfigurationProjection, hReconf] at hAdmitted hStep
      subst step
      exact admitted_reconfiguration_is_observational_only actualStep hAdmitted.2.2

theorem admitted_protocol_reconfiguration_of_bundle_does_not_own_route_truth
    (bundle : RuntimeSearchState)
    (step : ReducedReconfiguration)
    (hAdmitted : RuntimeSearchStateAdmitted bundle)
    (hStep : protocolReconfigurationProjection bundle = some step) :
    ReconfigurationDoesNotOwnRouteTruth step := by
  cases hReconf : bundle.protocolReconfiguration with
  | none =>
      simp [protocolReconfigurationProjection, hReconf] at hStep
  | some actualStep =>
      simp [RuntimeSearchStateAdmitted, protocolReconfigurationProjection, hReconf] at hAdmitted hStep
      subst step
      exact admitted_reconfiguration_does_not_own_route_truth actualStep hAdmitted.2.2

theorem reduced_runtime_search_adequacy_projects_canonical_route
    (destination : DestinationClass)
    (bundle : RuntimeSearchState)
    (state : EndToEndState)
    (hAdequacy : ReducedRuntimeSearchAdequacy bundle state)
    (hQuiescent : RuntimeStateQuiescent bundle.runtimeState) :
    runtimeCanonicalRouteOfBundle destination bundle =
      canonicalSystemRoute destination state := by
  exact
    quiescent_runtime_state_canonical_route_eq_canonicalSystemRoute
      destination bundle.runtimeState state hAdequacy.2 hQuiescent

theorem reduced_runtime_search_adequacy_projects_canonical_route_view
    (destination : DestinationClass)
    (bundle : RuntimeSearchState)
    (state : EndToEndState)
    (hAdequacy : ReducedRuntimeSearchAdequacy bundle state)
    (hQuiescent : RuntimeStateQuiescent bundle.runtimeState) :
    runtimeCanonicalRouteViewOfBundle destination bundle =
      bestSystemRouteView .supportDominance destination state := by
  exact
    quiescent_runtime_state_route_view_eq_bestSystemRouteView_supportDominance
      destination bundle.runtimeState state hAdequacy.2 hQuiescent

theorem bundles_with_same_runtime_state_have_same_canonical_route
    (destination : DestinationClass)
    (left right : RuntimeSearchState)
    (hRuntime : left.runtimeState = right.runtimeState) :
    runtimeCanonicalRouteOfBundle destination left =
      runtimeCanonicalRouteOfBundle destination right := by
  simp [runtimeCanonicalRouteOfBundle, hRuntime]

theorem bundles_with_same_runtime_state_have_same_canonical_route_view
    (destination : DestinationClass)
    (left right : RuntimeSearchState)
    (hRuntime : left.runtimeState = right.runtimeState) :
    runtimeCanonicalRouteViewOfBundle destination left =
      runtimeCanonicalRouteViewOfBundle destination right := by
  simp [runtimeCanonicalRouteViewOfBundle, hRuntime]

end FieldAdequacySearch
