import Mathlib.Order.Basic
import FieldModelAPI

/-
The Problem. We need one concrete bounded realization of the destination-local
field model so the first invariants can be proved against an executable round.
That realization should stay small enough to read as one observer-controller
pipeline rather than a copy of the full Rust engine.

Solution Structure.
1. Reuse one bounded `Nat` clamp for all scalar state.
2. Define each semantic substep as a compact pure function.
3. Prove boundedness and harmony through helper lemmas for each substep.
4. Expose a few concrete evidence cases that exercise the unified round.
-/

/-! # FieldModelInstance

First bounded realization of the deterministic local field model.

This instance uses a small permille-style encoding. It is intentionally smaller
than the Rust engine; the goal is to capture the semantic shape needed for
early proofs and boundary documentation.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldModelInstance

open FieldModelAPI

/-! ## Bounded Helpers -/

/-- Clamp a scalar field value into the shared permille budget. -/
def clampPermille (value : Nat) : Nat := min value 1000

/-- Every clamped scalar stays inside the shared budget. -/
theorem clampPermille_bounded (value : Nat) : clampPermille value ≤ 1000 := by
  exact Nat.min_le_right value 1000

/-- Reverse feedback can only increase local support. -/
def feedbackBonus : EvidenceFeedback → Nat
  | .none => 0
  | .weakReverse => 75
  | .strongReverse => 150

/-- Missing refresh increases uncertainty in the simplified model. -/
def freshnessPenalty : RefreshSignal → Nat
  | .unchanged => 100
  | .explicitRefresh => 0

/-- Refresh input alone decides whether the next observation is fresh. -/
def nextFreshness : RefreshSignal → ObservationFreshness
  | .unchanged => .stale
  | .explicitRefresh => .fresh

/-- Reachability updates preserve the distinction between unknown, corridor,
and explicit-path knowledge. -/
def nextKnowledge :
    ReachabilitySignal → ReachabilityKnowledge → ReachabilityKnowledge
  | .preserve, previous => previous
  | .unknown, _ => .unknown
  | .unreachable, _ => .unreachable
  | .corridorOnly, _ => .corridor
  | .explicitPath, _ => .explicitPath

/-! ## Unified Local Substeps -/

/-- Update the destination-local posterior from bounded evidence. -/
def updatePosteriorImpl
    (evidence : EvidenceInput)
    (state : LocalState) : PosteriorState :=
  { support := clampPermille (evidence.supportSignal + feedbackBonus evidence.feedback)
    entropy := clampPermille (evidence.entropySignal + freshnessPenalty evidence.refresh)
    freshness := nextFreshness evidence.refresh
    knowledge := nextKnowledge evidence.reachability state.posterior.knowledge }

/-- Compress the posterior into a low-order mean-field summary. -/
def compressMeanFieldImpl
    (evidence : EvidenceInput)
    (posterior : PosteriorState) : MeanFieldState :=
  { fieldStrength := posterior.support
    relayAlignment :=
      clampPermille ((posterior.support + evidence.controllerPressure) / 2)
    riskAlignment :=
      clampPermille ((posterior.entropy + evidence.controllerPressure) / 2) }

/-- Update the slow controller state from mean-field pressure. -/
def updateControllerImpl
    (evidence : EvidenceInput)
    (meanField : MeanFieldState)
    (_controller : ControllerState) : ControllerState :=
  { congestionPrice :=
      clampPermille ((meanField.riskAlignment + evidence.controllerPressure) / 2)
    stabilityMargin := meanField.fieldStrength }

/-- Infer the current operating regime from bounded posterior and control state. -/
def inferRegimeImpl
    (posterior : PosteriorState)
    (meanField : MeanFieldState)
    (controller : ControllerState) : RegimeState :=
  let residual :=
    clampPermille (posterior.entropy + controller.congestionPrice - meanField.fieldStrength)
  let current :=
    match posterior.knowledge with
    | .unreachable => OperatingRegime.unstable
    | .explicitPath => OperatingRegime.sparse
    | .corridor =>
        if meanField.riskAlignment ≥ 700 then
          OperatingRegime.adversarial
        else if controller.congestionPrice ≥ 600 then
          OperatingRegime.congested
        else
          OperatingRegime.retentionFavorable
    | .unknown =>
        if residual ≥ 500 then
          OperatingRegime.unstable
        else
          OperatingRegime.sparse
  { current := current, residual := residual }

/-- Choose the routing posture implied by the inferred regime. -/
def choosePostureImpl
    (regime : RegimeState)
    (_controller : ControllerState) : PostureState :=
  { current :=
      match regime.current with
      | .sparse => .opportunistic
      | .congested => .structured
      | .retentionFavorable => .retentionBiased
      | .unstable => .riskSuppressed
      | .adversarial => .riskSuppressed }

/-- Score continuation choices under the current posture. -/
def scoreContinuationsImpl
    (posterior : PosteriorState)
    (meanField : MeanFieldState)
    (controller : ControllerState)
    (posture : PostureState) : ScoredContinuationSet :=
  let postureBonus :=
    match posture.current with
    | .opportunistic => 30
    | .structured => 20
    | .retentionBiased => 15
    | .riskSuppressed => 10
  let base :=
    clampPermille (
      (posterior.support + meanField.fieldStrength + controller.stabilityMargin + postureBonus)
        / 3
    )
  { primaryScore := base
    alternateScore := base / 2 }

/-- Project the strongest honest shared corridor claim from local state. -/
def projectCorridorImpl
    (posterior : PosteriorState)
    (_meanField : MeanFieldState)
    (_controller : ControllerState)
    (scores : ScoredContinuationSet) : CorridorEnvelopeProjection :=
  let shape :=
    match posterior.knowledge with
    | .explicitPath => CorridorShape.explicitPath
    | .corridor => CorridorShape.corridorEnvelope
    | .unknown => CorridorShape.opaque
    | .unreachable => CorridorShape.opaque
  let bounds :=
    match shape with
    | .explicitPath => (2, 2)
    | .corridorEnvelope => (1, 3)
    | .opaque => (0, 4)
  { shape := shape
    support := min posterior.support scores.primaryScore
    hopLower := bounds.1
    hopUpper := bounds.2 }

/-- Compose all local substeps into one deterministic round transition. -/
def roundStepImpl (evidence : EvidenceInput) (state : LocalState) : LocalState :=
  let posterior := updatePosteriorImpl evidence state
  let meanField := compressMeanFieldImpl evidence posterior
  let controller := updateControllerImpl evidence meanField state.controller
  let regime := inferRegimeImpl posterior meanField controller
  let posture := choosePostureImpl regime controller
  let scored := scoreContinuationsImpl posterior meanField controller posture
  let projection := projectCorridorImpl posterior meanField controller scored
  { posterior := posterior
    meanField := meanField
    controller := controller
    regime := regime
    posture := posture
    scored := scored
    projection := projection }

/-! ## Local Boundedness Lemmas -/

/-- The shared projection advertises an explicit path exactly when the local
knowledge is explicit-path knowledge. -/
private theorem projection_shape_explicit_iff
    (knowledge : ReachabilityKnowledge) :
    (match knowledge with
      | .explicitPath => CorridorShape.explicitPath
      | .corridor => CorridorShape.corridorEnvelope
      | .unknown => CorridorShape.opaque
      | .unreachable => CorridorShape.opaque) =
        CorridorShape.explicitPath ↔
      knowledge = ReachabilityKnowledge.explicitPath := by
  -- This is a small case split over the finite knowledge lattice.
  cases knowledge <;> simp

/-- Every fixed projection hop band preserves `lower ≤ upper`. -/
private theorem projected_hop_band_ordered
    (knowledge : ReachabilityKnowledge) :
    (match
        match knowledge with
        | .explicitPath => CorridorShape.explicitPath
        | .corridor => CorridorShape.corridorEnvelope
        | .unknown => CorridorShape.opaque
        | .unreachable => CorridorShape.opaque with
      | CorridorShape.explicitPath => (2, 2)
      | CorridorShape.corridorEnvelope => (1, 3)
      | CorridorShape.opaque => (0, 4)).fst ≤
    (match
        match knowledge with
        | .explicitPath => CorridorShape.explicitPath
        | .corridor => CorridorShape.corridorEnvelope
        | .unknown => CorridorShape.opaque
        | .unreachable => CorridorShape.opaque with
      | CorridorShape.explicitPath => (2, 2)
      | CorridorShape.corridorEnvelope => (1, 3)
      | CorridorShape.opaque => (0, 4)).snd := by
  -- Every fixed band in the reduced model is ordered by construction.
  cases knowledge <;> decide

/-- Posterior updates always stay inside the bounded scalar budget. -/
theorem updatePosteriorImpl_bounded
    (evidence : EvidenceInput)
    (state : LocalState) :
    PosteriorBounded (updatePosteriorImpl evidence state) := by
  -- Both scalar fields are clamped, so the posterior stays bounded.
  constructor <;> simp [BoundedNat, updatePosteriorImpl, clampPermille_bounded]

/-- Mean-field compression preserves the bounded scalar budget. -/
theorem compressMeanFieldImpl_bounded
    (evidence : EvidenceInput)
    (posterior : PosteriorState)
    (hPosterior : PosteriorBounded posterior) :
    MeanFieldBounded (compressMeanFieldImpl evidence posterior) := by
  -- The compressed summary is built only from clamped averages or copied support.
  rcases hPosterior with ⟨hSupport, _⟩
  constructor
  · simpa [compressMeanFieldImpl, BoundedNat] using hSupport
  constructor
  · simpa [compressMeanFieldImpl, BoundedNat] using clampPermille_bounded ((posterior.support + evidence.controllerPressure) / 2)
  · simpa [compressMeanFieldImpl, BoundedNat] using clampPermille_bounded ((posterior.entropy + evidence.controllerPressure) / 2)

/-- Controller updates preserve the bounded scalar budget. -/
theorem updateControllerImpl_bounded
    (evidence : EvidenceInput)
    (meanField : MeanFieldState)
    (_controller : ControllerState)
    (hMeanField : MeanFieldBounded meanField) :
    ControllerBounded (updateControllerImpl evidence meanField _controller) := by
  -- Congestion is clamped and the stability margin copies bounded field strength.
  rcases hMeanField with ⟨hField, _, _⟩
  constructor
  · simpa [ControllerBounded, BoundedNat, updateControllerImpl] using
      clampPermille_bounded ((meanField.riskAlignment + evidence.controllerPressure) / 2)
  · simpa [ControllerBounded, BoundedNat, updateControllerImpl] using hField

/-- Regime inference preserves the bounded residual budget. -/
theorem inferRegimeImpl_bounded
    (posterior : PosteriorState)
    (meanField : MeanFieldState)
    (controller : ControllerState) :
    RegimeBounded (inferRegimeImpl posterior meanField controller) := by
  -- The residual is explicitly clamped before the regime branch is chosen.
  simpa [RegimeBounded, BoundedNat, inferRegimeImpl] using
    clampPermille_bounded
      (posterior.entropy + controller.congestionPrice - meanField.fieldStrength)

/-- Continuation scoring stays bounded and preserves the primary/alternate
ordering relation. -/
theorem scoreContinuationsImpl_bounded
    (posterior : PosteriorState)
    (meanField : MeanFieldState)
    (controller : ControllerState)
    (posture : PostureState) :
    ContinuationScoresBounded
      (scoreContinuationsImpl posterior meanField controller posture) := by
  -- The primary score is clamped and the alternate score is a bounded half.
  let postureBonus :=
    match posture.current with
    | .opportunistic => 30
    | .structured => 20
    | .retentionBiased => 15
    | .riskSuppressed => 10
  let base :=
    clampPermille
      ((posterior.support + meanField.fieldStrength + controller.stabilityMargin + postureBonus) / 3)
  have hBase : base ≤ 1000 := by
    simpa [base] using
      clampPermille_bounded
        ((posterior.support + meanField.fieldStrength + controller.stabilityMargin + postureBonus) / 3)
  refine ⟨?_, ?_, ?_⟩
  · simpa [scoreContinuationsImpl, base]
  · simpa [scoreContinuationsImpl, base] using Nat.le_trans (Nat.div_le_self base 2) hBase
  · simpa [scoreContinuationsImpl, base] using Nat.div_le_self base 2

/-- Corridor projection stays bounded when the posterior and scores are
already bounded. -/
theorem projectCorridorImpl_bounded
    (posterior : PosteriorState)
    (meanField : MeanFieldState)
    (controller : ControllerState)
    (scores : ScoredContinuationSet)
    (hPosterior : PosteriorBounded posterior)
    (hScores : ContinuationScoresBounded scores) :
    ProjectionBounded
      (projectCorridorImpl posterior meanField controller scores) := by
  -- Projection support is the minimum of two bounded supports and the hop band
  -- is chosen from a fixed family of ordered bounds.
  rcases hPosterior with ⟨hSupport, _⟩
  rcases hScores with ⟨hPrimary, _, _⟩
  refine ⟨?_, ?_⟩
  · exact Nat.le_trans (Nat.min_le_left posterior.support scores.primaryScore) hSupport
  · simpa using projected_hop_band_ordered posterior.knowledge

/-! ## API Instance -/

instance : FieldModelAPI.Laws where
  updatePosterior := updatePosteriorImpl
  compressMeanField := compressMeanFieldImpl
  updateController := updateControllerImpl
  inferRegime := inferRegimeImpl
  choosePosture := choosePostureImpl
  scoreContinuations := scoreContinuationsImpl
  projectCorridor := projectCorridorImpl
  roundStep := roundStepImpl
  round_preserves_bounded := by
    intro evidence state
    -- The round is bounded because each substep is bounded independently.
    let posterior := updatePosteriorImpl evidence state
    let meanField := compressMeanFieldImpl evidence posterior
    let controller := updateControllerImpl evidence meanField state.controller
    let regime := inferRegimeImpl posterior meanField controller
    let posture := choosePostureImpl regime controller
    let scored := scoreContinuationsImpl posterior meanField controller posture
    let projection := projectCorridorImpl posterior meanField controller scored
    have hPosterior : PosteriorBounded posterior := by
      simpa [posterior] using updatePosteriorImpl_bounded evidence state
    have hMeanField : MeanFieldBounded meanField := by
      simpa [meanField] using compressMeanFieldImpl_bounded evidence posterior hPosterior
    have hController : ControllerBounded controller := by
      simpa [controller] using
        updateControllerImpl_bounded evidence meanField state.controller hMeanField
    have hRegime : RegimeBounded regime := by
      simpa [regime] using inferRegimeImpl_bounded posterior meanField controller
    have hScores : ContinuationScoresBounded scored := by
      simpa [scored] using scoreContinuationsImpl_bounded posterior meanField controller posture
    have hProjection : ProjectionBounded projection := by
      simpa [projection] using
        projectCorridorImpl_bounded posterior meanField controller scored hPosterior hScores
    exact ⟨hPosterior, hMeanField, hController, hRegime, hScores, hProjection⟩
  round_preserves_harmony := by
    intro evidence state
    -- The composed round wires subordinate state directly from the posterior.
    constructor
    · simp [roundStepImpl, compressMeanFieldImpl]
    constructor
    · simp [roundStepImpl, updateControllerImpl, compressMeanFieldImpl]
    constructor
    · simpa [roundStepImpl, projectCorridorImpl] using
        projection_shape_explicit_iff (updatePosteriorImpl evidence state).knowledge
    constructor
    · exact Nat.min_le_left _ _
    · exact Nat.div_le_self _ 2
  fresh_requires_refresh := by
    intro evidence state hRefresh
    -- Freshness is controlled only by the refresh signal.
    simp [roundStepImpl, updatePosteriorImpl, nextFreshness, hRefresh]
  unknown_signal_stays_unknown := by
    intro evidence state hReachability
    -- Unknown reachability remains unknown after one full round.
    simp [roundStepImpl, updatePosteriorImpl, nextKnowledge, hReachability]
  explicit_projection_requires_explicit_knowledge := by
    intro evidence state hShape
    -- Projection shape is computed directly from posterior knowledge.
    have hProjection :
        (match (updatePosteriorImpl evidence state).knowledge with
          | .explicitPath => CorridorShape.explicitPath
          | .corridor => CorridorShape.corridorEnvelope
          | .unknown => CorridorShape.opaque
          | .unreachable => CorridorShape.opaque) =
          CorridorShape.explicitPath := by
      simpa [roundStepImpl, projectCorridorImpl] using hShape
    exact (projection_shape_explicit_iff (updatePosteriorImpl evidence state).knowledge).mp
      hProjection
  multi_layer_projection_subordinate := by
    intro evidence state
    -- The compressed field and shared projection remain subordinate to the posterior.
    constructor
    · simp [roundStepImpl, compressMeanFieldImpl]
    constructor
    · simp [roundStepImpl, updateControllerImpl, compressMeanFieldImpl]
    · exact Nat.min_le_left _ _

/-! ## Example States And Representative Theorems -/

/-- Small default state used by the first executable examples. -/
def initialState : LocalState :=
  { posterior :=
      { support := 400
        entropy := 200
        freshness := .stale
        knowledge := .corridor }
    meanField :=
      { fieldStrength := 400, relayAlignment := 300, riskAlignment := 250 }
    controller :=
      { congestionPrice := 200, stabilityMargin := 400 }
    regime := { current := .sparse, residual := 100 }
    posture := { current := .opportunistic }
    scored := { primaryScore := 400, alternateScore := 200 }
    projection :=
      { shape := .corridorEnvelope, support := 400, hopLower := 1, hopUpper := 3 } }

/-- Evidence that keeps the destination stale and unknown. -/
def unknownEvidence : EvidenceInput :=
  { refresh := .unchanged
    reachability := .unknown
    supportSignal := 250
    entropySignal := 300
    controllerPressure := 150
    feedback := .none }

/-- Evidence that upgrades the destination to explicit-path knowledge. -/
def explicitPathEvidence : EvidenceInput :=
  { refresh := .explicitRefresh
    reachability := .explicitPath
    supportSignal := 900
    entropySignal := 50
    controllerPressure := 100
    feedback := .strongReverse }

/-- Evidence that forces an adversarial posture through the full pipeline. -/
def adversarialEvidence : EvidenceInput :=
  { refresh := .explicitRefresh
    reachability := .corridorOnly
    supportSignal := 600
    entropySignal := 900
    controllerPressure := 800
    feedback := .none }

/-- The full round is deterministic because it is a pure function. -/
theorem local_round_deterministic
    (evidence : EvidenceInput)
    (state : LocalState) :
    FieldModelAPI.roundStep evidence state = FieldModelAPI.roundStep evidence state := by
  -- Determinism follows from definitional equality.
  rfl

/-- Unknown reachability is never silently collapsed to unreachable. -/
theorem unknown_signal_not_collapsed
    (state : LocalState) :
    (FieldModelAPI.roundStep unknownEvidence state).posterior.knowledge =
      ReachabilityKnowledge.unknown := by
  -- The round copies the unknown signal through `nextKnowledge`.
  exact FieldModelAPI.unknown_signal_stays_unknown unknownEvidence state rfl

/-- Stale evidence remains stale without an explicit refresh input. -/
theorem stale_without_refresh
    (state : LocalState) :
    (FieldModelAPI.roundStep unknownEvidence state).posterior.freshness =
      ObservationFreshness.stale := by
  -- Freshness is driven only by `nextFreshness`.
  exact FieldModelAPI.fresh_requires_refresh unknownEvidence state rfl

/-- Explicit-path evidence is strong enough to produce an explicit-path
projection after one full round. -/
theorem explicit_path_signal_yields_explicit_projection :
    (FieldModelAPI.roundStep explicitPathEvidence initialState).projection.shape =
      CorridorShape.explicitPath := by
  -- The signal flows from posterior knowledge to the shared projection.
  native_decide

/-- Strong corridor risk pushes the local controller into a risk-suppressed
posture. -/
theorem adversarial_corridor_signal_suppresses_posture :
    (FieldModelAPI.roundStep adversarialEvidence initialState).posture.current =
      RoutingPosture.riskSuppressed := by
  -- The evidence first induces the adversarial regime and then the posture map
  -- sends that regime to `riskSuppressed`.
  native_decide

/-- Corridor projection never manufactures explicit-path truth. -/
theorem corridor_projection_never_invents_explicit_path
    (evidence : EvidenceInput)
    (state : LocalState)
    (h :
      (FieldModelAPI.roundStep evidence state).projection.shape =
        CorridorShape.explicitPath) :
    (FieldModelAPI.roundStep evidence state).posterior.knowledge =
      ReachabilityKnowledge.explicitPath := by
  -- This is the main honesty wrapper exposed by the API layer.
  exact
    FieldModelAPI.explicit_projection_requires_explicit_knowledge evidence state h

/-- The projection remains subordinate to the posterior across the full round. -/
theorem unified_round_subordinate
    (evidence : EvidenceInput)
    (state : LocalState) :
    let next := FieldModelAPI.roundStep evidence state
    next.meanField.fieldStrength = next.posterior.support ∧
      next.controller.stabilityMargin = next.meanField.fieldStrength ∧
      next.projection.support ≤ next.posterior.support := by
  -- This is the first multi-layer theorem for the unified observer-controller.
  exact FieldModelAPI.multi_layer_projection_subordinate evidence state

end FieldModelInstance
