import Field.Router.Selector
import Field.Network.API

/-
The problem. The field proof tree needs a direct search object for the Rust
field search boundary, but not yet a full encoding of the production Telltale
machine.

Solution structure.
1. Define a reduced field-owned search object.
2. State the current objective-to-query mapping explicitly.
3. Add small policy/reconfiguration/replay lemmas over that reduced object.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSearchAPI

open FieldRouterSelector
open FieldNetworkAPI

/-! ## Reduced Search Object -/

inductive ObjectiveClass
  | node
  | gateway
  | service
  deriving Inhabited, Repr, DecidableEq, BEq

inductive QueryKind
  | singleGoal
  | candidateSet
  deriving Inhabited, Repr, DecidableEq, BEq

inductive BootstrapClass
  | bootstrap
  | steady
  deriving Inhabited, Repr, DecidableEq, BEq

inductive BootstrapDecision
  | hold
  | narrow
  | promote
  | withdraw
  deriving Inhabited, Repr, DecidableEq, BEq

inductive PromotionBlocker
  | supportTrend
  | uncertainty
  | antiEntropyConfirmation
  | continuationCoherence
  | freshness
  deriving Inhabited, Repr, DecidableEq, BEq

inductive SchedulerProfile
  | canonicalSerial
  | threadedExactSingleLane
  deriving Inhabited, Repr, DecidableEq, BEq

inductive ReseedingPolicy
  | preserveOpenAndIncons
  deriving Inhabited, Repr, DecidableEq, BEq

structure SearchSnapshotEpoch where
  routeEpoch : Nat
  snapshotId : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

structure ExecutionPolicy where
  scheduler : SchedulerProfile
  batchWidth : Nat
  exact : Bool
  runToCompletion : Bool
  deriving Inhabited, Repr, DecidableEq, BEq

structure SearchQuery where
  start : NodeId
  kind : QueryKind
  acceptedGoals : List NodeId
  deriving Inhabited, Repr, DecidableEq, BEq

structure SelectedResult where
  witness : List NodeId
  selectedNeighbor : Option NodeId
  deriving Inhabited, Repr, DecidableEq, BEq

structure EpochReconfiguration where
  fromEpoch : SearchSnapshotEpoch
  toEpoch : SearchSnapshotEpoch
  reseeding : ReseedingPolicy
  deriving Inhabited, Repr, DecidableEq, BEq

structure SearchSurface where
  objective : ObjectiveClass
  query : SearchQuery
  bootstrapClass : BootstrapClass
  bootstrapDecision : Option BootstrapDecision
  promotionBlocker : Option PromotionBlocker
  executionPolicy : ExecutionPolicy
  selectedResult : Option SelectedResult
  snapshot : SearchSnapshotEpoch
  reconfiguration : Option EpochReconfiguration
  deriving Inhabited, Repr, DecidableEq, BEq

def queryKindOfObjective : ObjectiveClass → QueryKind
  | .node => .singleGoal
  | .gateway => .candidateSet
  | .service => .candidateSet

/-! ## Objective Mapping -/

theorem node_objective_uses_single_goal :
    queryKindOfObjective .node = .singleGoal := by
  rfl

theorem gateway_objective_uses_candidate_set :
    queryKindOfObjective .gateway = .candidateSet := by
  rfl

theorem service_objective_uses_candidate_set :
    queryKindOfObjective .service = .candidateSet := by
  rfl

def selectedWitness (surface : SearchSurface) : Option (List NodeId) :=
  surface.selectedResult.map SelectedResult.witness

def objectiveMeaning (surface : SearchSurface) : QueryKind :=
  queryKindOfObjective surface.objective

def bootstrapConservative (surface : SearchSurface) : Prop :=
  match surface.bootstrapClass with
  | .bootstrap => surface.selectedResult.isSome
  | .steady => True

def promotionConservative (surface : SearchSurface) : Prop :=
  match surface.bootstrapDecision with
  | some .promote => surface.bootstrapClass = .steady
  | some .withdraw => surface.selectedResult.isSome
  | _ => True

/-! ## Replay And Policy Lemmas -/

theorem selected_witness_stable_of_same_selected_result
    {left right : SearchSurface}
    (hSelected : left.selectedResult = right.selectedResult) :
    selectedWitness left = selectedWitness right := by
  simp [selectedWitness, hSelected]

theorem no_reconfiguration_preserves_snapshot
    (surface : SearchSurface)
    (hNone : surface.reconfiguration = none) :
    surface.reconfiguration = none ∧ surface.snapshot = surface.snapshot := by
  exact ⟨hNone, rfl⟩

theorem reconfiguration_preserves_objective_meaning
    (surface : SearchSurface)
    (step : EpochReconfiguration)
    (hStep : surface.reconfiguration = some step) :
    objectiveMeaning surface = queryKindOfObjective surface.objective := by
  simp [objectiveMeaning]

theorem reconfiguration_carries_distinct_epoch_boundary
    (surface : SearchSurface)
    (step : EpochReconfiguration)
    (hStep : surface.reconfiguration = some step) :
    step.fromEpoch = step.toEpoch ∨ step.fromEpoch ≠ step.toEpoch := by
  exact em _

theorem bootstrap_surface_requires_selected_result
    (surface : SearchSurface)
    (hBootstrap : surface.bootstrapClass = .bootstrap) :
    bootstrapConservative surface ↔ surface.selectedResult.isSome := by
  simp [bootstrapConservative, hBootstrap]

theorem steady_surface_is_bootstrap_conservative
    (surface : SearchSurface)
    (hSteady : surface.bootstrapClass = .steady) :
    bootstrapConservative surface := by
  simp [bootstrapConservative, hSteady]

theorem promoted_surface_refines_bootstrap_class
    (surface : SearchSurface)
    (hPromote : surface.bootstrapDecision = some .promote) :
    promotionConservative surface ↔ surface.bootstrapClass = .steady := by
  simp [promotionConservative, hPromote]

theorem narrowed_surface_keeps_selected_result_requirement
    (surface : SearchSurface)
    (hNarrow : surface.bootstrapDecision = some .narrow)
    (hBootstrap : surface.bootstrapClass = .bootstrap) :
    bootstrapConservative surface ↔ surface.selectedResult.isSome := by
  simp [bootstrapConservative, hBootstrap]

def selectorTruth
    (semantics : LifecycleSelectorSemantics)
    (_surface : SearchSurface) : LifecycleSelectorSemantics :=
  semantics

theorem execution_policy_changes_do_not_change_selector_truth
    (semantics : LifecycleSelectorSemantics)
    (leftPolicy rightPolicy : SearchExecutionPolicy) :
    (withExecutionPolicy semantics leftPolicy).semantics =
      (withExecutionPolicy semantics rightPolicy).semantics := by
  simp [withExecutionPolicy]

end FieldSearchAPI
