import Mathlib.Data.Nat.Basic

/-
The Problem. The field engine needs a small formal model for one local
destination round that preserves the unification of the observer-controller
pipeline. We need that model to be compact enough for early proofs, but stable
enough that downstream proofs can depend on abstract operations and laws rather
than on the first concrete bucket encoding.

Solution Structure.
1. Define the semantic state components for one local destination round.
2. Define an abstract `Model` interface for the named substeps and unified round
   transition.
3. Define compact law interfaces for boundedness, honesty, and harmony.
4. Re-export stable wrappers so downstream proofs depend on the API surface.
-/

/-! # FieldModelAPI

Abstract API for the deterministic local field-routing model.

This module defines the proof-facing interface for one destination-local field
round. It deliberately separates:

- semantic state shapes
- abstract local-model operations
- law bundles used by downstream proofs

Concrete bounded realizations live in `FieldModelInstance`.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldModelAPI

/-! ## Core Enumerations -/

inductive RefreshSignal
  | unchanged
  | explicitRefresh
  deriving Inhabited, Repr, DecidableEq, BEq

inductive EvidenceFeedback
  | none
  | weakReverse
  | strongReverse
  deriving Inhabited, Repr, DecidableEq, BEq

inductive ReachabilitySignal
  | preserve
  | unknown
  | unreachable
  | corridorOnly
  | explicitPath
  deriving Inhabited, Repr, DecidableEq, BEq

inductive ObservationFreshness
  | stale
  | fresh
  deriving Inhabited, Repr, DecidableEq, BEq

inductive ReachabilityKnowledge
  | unknown
  | unreachable
  | corridor
  | explicitPath
  deriving Inhabited, Repr, DecidableEq, BEq

inductive OperatingRegime
  | sparse
  | congested
  | retentionFavorable
  | unstable
  | adversarial
  deriving Inhabited, Repr, DecidableEq, BEq

inductive RoutingPosture
  | opportunistic
  | structured
  | retentionBiased
  | riskSuppressed
  deriving Inhabited, Repr, DecidableEq, BEq

inductive CorridorShape
  | opaque
  | corridorEnvelope
  | explicitPath
  deriving Inhabited, Repr, DecidableEq, BEq

/-! ## Local Semantic State -/

/-- Bounded observational evidence consumed by one local round. -/
structure EvidenceInput where
  refresh : RefreshSignal
  reachability : ReachabilitySignal
  supportSignal : Nat
  entropySignal : Nat
  controllerPressure : Nat
  feedback : EvidenceFeedback
  deriving Repr, DecidableEq, BEq

/-- Local posterior over corridor viability and knowledge strength. -/
structure PosteriorState where
  support : Nat
  entropy : Nat
  freshness : ObservationFreshness
  knowledge : ReachabilityKnowledge
  deriving Repr, DecidableEq, BEq

/-- Low-order summary subordinate to the posterior. -/
structure MeanFieldState where
  fieldStrength : Nat
  relayAlignment : Nat
  riskAlignment : Nat
  deriving Repr, DecidableEq, BEq

/-- Slow-moving controller state carried across rounds. -/
structure ControllerState where
  congestionPrice : Nat
  stabilityMargin : Nat
  deriving Repr, DecidableEq, BEq

/-- Inferred regime and residual fit summary. -/
structure RegimeState where
  current : OperatingRegime
  residual : Nat
  deriving Repr, DecidableEq, BEq

/-- Current routing posture. -/
structure PostureState where
  current : RoutingPosture
  deriving Repr, DecidableEq, BEq

/-- Ranked local continuation scores for the current corridor belief. -/
structure ScoredContinuationSet where
  primaryScore : Nat
  alternateScore : Nat
  deriving Repr, DecidableEq, BEq

/-- Conservative shared corridor projection derived from local state. -/
structure CorridorEnvelopeProjection where
  shape : CorridorShape
  support : Nat
  hopLower : Nat
  hopUpper : Nat
  deriving Repr, DecidableEq, BEq

/-- One destination-local round state for the unified observer-controller. -/
structure LocalState where
  posterior : PosteriorState
  meanField : MeanFieldState
  controller : ControllerState
  regime : RegimeState
  posture : PostureState
  scored : ScoredContinuationSet
  projection : CorridorEnvelopeProjection
  deriving Repr, DecidableEq, BEq

/-! ## Structural Invariants -/

def BoundedNat (n : Nat) : Prop := n ≤ 1000

def PosteriorBounded (state : PosteriorState) : Prop :=
  BoundedNat state.support ∧ BoundedNat state.entropy

def MeanFieldBounded (state : MeanFieldState) : Prop :=
  BoundedNat state.fieldStrength ∧
    BoundedNat state.relayAlignment ∧
    BoundedNat state.riskAlignment

def ControllerBounded (state : ControllerState) : Prop :=
  BoundedNat state.congestionPrice ∧ BoundedNat state.stabilityMargin

def RegimeBounded (state : RegimeState) : Prop := BoundedNat state.residual

def ContinuationScoresBounded (state : ScoredContinuationSet) : Prop :=
  BoundedNat state.primaryScore ∧
    BoundedNat state.alternateScore ∧
    state.alternateScore ≤ state.primaryScore

def ProjectionBounded (state : CorridorEnvelopeProjection) : Prop :=
  BoundedNat state.support ∧ state.hopLower ≤ state.hopUpper

def StateBounded (state : LocalState) : Prop :=
  PosteriorBounded state.posterior ∧
    MeanFieldBounded state.meanField ∧
    ControllerBounded state.controller ∧
    RegimeBounded state.regime ∧
    ContinuationScoresBounded state.scored ∧
    ProjectionBounded state.projection

/-- The local model is harmonious when the downstream states remain subordinate
to the posterior and shared projection. -/
def Harmony (state : LocalState) : Prop :=
  state.meanField.fieldStrength = state.posterior.support ∧
    state.controller.stabilityMargin = state.meanField.fieldStrength ∧
    (state.projection.shape = CorridorShape.explicitPath ↔
      state.posterior.knowledge = ReachabilityKnowledge.explicitPath) ∧
    state.projection.support ≤ state.posterior.support ∧
    state.scored.alternateScore ≤ state.scored.primaryScore

/-! ## Abstract Operations -/

class Model where
  updatePosterior : EvidenceInput → LocalState → PosteriorState
  compressMeanField : EvidenceInput → PosteriorState → MeanFieldState
  updateController : EvidenceInput → MeanFieldState → ControllerState → ControllerState
  inferRegime :
    PosteriorState → MeanFieldState → ControllerState → RegimeState
  choosePosture : RegimeState → ControllerState → PostureState
  scoreContinuations :
    PosteriorState →
      MeanFieldState →
      ControllerState →
      PostureState →
      ScoredContinuationSet
  projectCorridor :
    PosteriorState →
      MeanFieldState →
      ControllerState →
      ScoredContinuationSet →
      CorridorEnvelopeProjection
  roundStep : EvidenceInput → LocalState → LocalState

section Wrappers

variable [Model]

def updatePosterior (evidence : EvidenceInput) (state : LocalState) : PosteriorState :=
  Model.updatePosterior evidence state

def compressMeanField
    (evidence : EvidenceInput)
    (posterior : PosteriorState) : MeanFieldState :=
  Model.compressMeanField evidence posterior

def updateController
    (evidence : EvidenceInput)
    (meanField : MeanFieldState)
    (controller : ControllerState) : ControllerState :=
  Model.updateController evidence meanField controller

def inferRegime
    (posterior : PosteriorState)
    (meanField : MeanFieldState)
    (controller : ControllerState) : RegimeState :=
  Model.inferRegime posterior meanField controller

def choosePosture
    (regime : RegimeState)
    (controller : ControllerState) : PostureState :=
  Model.choosePosture regime controller

def scoreContinuations
    (posterior : PosteriorState)
    (meanField : MeanFieldState)
    (controller : ControllerState)
    (posture : PostureState) : ScoredContinuationSet :=
  Model.scoreContinuations posterior meanField controller posture

def projectCorridor
    (posterior : PosteriorState)
    (meanField : MeanFieldState)
    (controller : ControllerState)
    (scored : ScoredContinuationSet) : CorridorEnvelopeProjection :=
  Model.projectCorridor posterior meanField controller scored

def roundStep (evidence : EvidenceInput) (state : LocalState) : LocalState :=
  Model.roundStep evidence state

end Wrappers

/-! ## Law Interfaces -/

abbrev RoundPreservesBounded (M : Model) : Prop :=
  ∀ evidence state, StateBounded (@Model.roundStep M evidence state)

abbrev RoundPreservesHarmony (M : Model) : Prop :=
  ∀ evidence state, Harmony (@Model.roundStep M evidence state)

abbrev FreshRequiresRefresh (M : Model) : Prop :=
  ∀ evidence state,
    evidence.refresh = RefreshSignal.unchanged →
      (@Model.roundStep M evidence state).posterior.freshness =
        ObservationFreshness.stale

abbrev UnknownSignalStaysUnknown (M : Model) : Prop :=
  ∀ evidence state,
    evidence.reachability = ReachabilitySignal.unknown →
      (@Model.roundStep M evidence state).posterior.knowledge =
        ReachabilityKnowledge.unknown

abbrev ExplicitProjectionRequiresExplicitKnowledge (M : Model) : Prop :=
  ∀ evidence state,
    (@Model.roundStep M evidence state).projection.shape =
        CorridorShape.explicitPath →
      (@Model.roundStep M evidence state).posterior.knowledge =
        ReachabilityKnowledge.explicitPath

abbrev MultiLayerProjectionSubordinate (M : Model) : Prop :=
  ∀ evidence state,
    let next := @Model.roundStep M evidence state
    next.meanField.fieldStrength = next.posterior.support ∧
      next.controller.stabilityMargin = next.meanField.fieldStrength ∧
      next.projection.support ≤ next.posterior.support

class Laws extends Model where
  round_preserves_bounded : RoundPreservesBounded toModel
  round_preserves_harmony : RoundPreservesHarmony toModel
  fresh_requires_refresh : FreshRequiresRefresh toModel
  unknown_signal_stays_unknown : UnknownSignalStaysUnknown toModel
  explicit_projection_requires_explicit_knowledge :
    ExplicitProjectionRequiresExplicitKnowledge toModel
  multi_layer_projection_subordinate : MultiLayerProjectionSubordinate toModel

instance (priority := 100) lawsToModel [Laws] : Model := Laws.toModel

section LawWrappers

variable [Laws]

theorem round_preserves_bounded
    (evidence : EvidenceInput)
    (state : LocalState) : StateBounded (roundStep evidence state) :=
  Laws.round_preserves_bounded evidence state

theorem round_preserves_harmony
    (evidence : EvidenceInput)
    (state : LocalState) : Harmony (roundStep evidence state) :=
  Laws.round_preserves_harmony evidence state

theorem fresh_requires_refresh
    (evidence : EvidenceInput)
    (state : LocalState)
    (h : evidence.refresh = RefreshSignal.unchanged) :
    (roundStep evidence state).posterior.freshness = ObservationFreshness.stale :=
  Laws.fresh_requires_refresh evidence state h

theorem unknown_signal_stays_unknown
    (evidence : EvidenceInput)
    (state : LocalState)
    (h : evidence.reachability = ReachabilitySignal.unknown) :
    (roundStep evidence state).posterior.knowledge =
      ReachabilityKnowledge.unknown :=
  Laws.unknown_signal_stays_unknown evidence state h

theorem explicit_projection_requires_explicit_knowledge
    (evidence : EvidenceInput)
    (state : LocalState)
    (h :
      (roundStep evidence state).projection.shape = CorridorShape.explicitPath) :
    (roundStep evidence state).posterior.knowledge =
      ReachabilityKnowledge.explicitPath :=
  Laws.explicit_projection_requires_explicit_knowledge evidence state h

theorem multi_layer_projection_subordinate
    (evidence : EvidenceInput)
    (state : LocalState) :
    let next := roundStep evidence state
    next.meanField.fieldStrength = next.posterior.support ∧
      next.controller.stabilityMargin = next.meanField.fieldStrength ∧
      next.projection.support ≤ next.posterior.support :=
  Laws.multi_layer_projection_subordinate evidence state

end LawWrappers

end FieldModelAPI
