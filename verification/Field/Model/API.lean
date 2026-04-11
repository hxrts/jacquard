import ClassicalAnalysisAPI
import Mathlib.Data.Fintype.Basic
import Mathlib.Data.Nat.Basic
import Mathlib.Tactic.DeriveFintype

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

State taxonomy used across `verification/Field`:

- epistemic state:
  latent/private belief and posterior objects
- control state:
  retained aggregates, controller state, regime state, posture state, and
  scoring state
- publication state:
  public corridor/macrostates and publication-facing route summaries
- lifecycle state:
  admitted/installed/maintained route objects owned by the router
- execution state:
  async, end-to-end, and runtime operational state
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldModelAPI

open EntropyAPI

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

inductive FieldHypothesis
  | unknown
  | unreachable
  | corridor
  | explicitPath
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

inductive ProbabilisticRouteExistence
  | absent
  | present
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

inductive RouteQualityBand
  | low
  | medium
  | high
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

inductive TransportReliabilityBand
  | lossy
  | delayed
  | reliable
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

inductive ObservationReliabilityBand
  | noisy
  | corroborated
  | trusted
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

structure ProbabilisticRouteHypothesis where
  existence : ProbabilisticRouteExistence
  quality : RouteQualityBand
  transportReliability : TransportReliabilityBand
  observationReliability : ObservationReliabilityBand
  knowledge : FieldHypothesis
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

structure ProbabilisticRouteBelief where
  distribution : Distribution ProbabilisticRouteHypothesis

namespace ProbabilisticRouteBelief

def pmf (belief : ProbabilisticRouteBelief) : ProbabilisticRouteHypothesis → ℝ :=
  belief.distribution.pmf

theorem nonneg
    (belief : ProbabilisticRouteBelief)
    (hypothesis : ProbabilisticRouteHypothesis) :
    0 ≤ belief.pmf hypothesis :=
  belief.distribution.nonneg hypothesis

theorem sum_one (belief : ProbabilisticRouteBelief) :
    ∑ h, belief.pmf h = 1 :=
  belief.distribution.sum_one

end ProbabilisticRouteBelief

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

/-- Finite bounded belief weights over the reduced local hypothesis space. -/
structure FiniteBelief where
  unknownWeight : Nat
  unreachableWeight : Nat
  corridorWeight : Nat
  explicitPathWeight : Nat
  deriving Repr, DecidableEq, BEq

/-- Local epistemic state over corridor viability and knowledge strength.
Support and uncertainty are derived from the finite belief object rather than
stored as independent fields. -/
structure PosteriorState where
  belief : FiniteBelief
  freshness : ObservationFreshness
  knowledge : ReachabilityKnowledge
  deriving Repr, DecidableEq, BEq

/-- Explicit posterior-derived reduction boundary used as the compact
controller-facing summary input. This object is intentionally finite and keeps
posterior-derived quantities separate from exogenous control fusion. -/
structure ReducedBeliefSummary where
  supportMass : Nat
  uncertaintyMass : Nat
  publicMacrostate : CorridorShape
  deriving Repr, DecidableEq, BEq

/-- Explicit order-parameter view extracted from the reduced summary. This is
the local phase/regime-facing object: it is still reduced and finite, but it
is conceptually prior to exogenous controller-input fusion. -/
structure LocalOrderParameter where
  supportCoordinate : Nat
  uncertaintyCoordinate : Nat
  macrostate : CorridorShape
  deriving Repr, DecidableEq, BEq

/-- Within-regime stability coordinate currently tracked by the reduced order
parameter. In the current model this is just support mass; the separate name is
introduced so stronger regime-indexed theory can land later without changing
every theorem surface. -/
def LocalOrderParameter.withinRegimeStability
    (parameter : LocalOrderParameter) : Nat :=
  parameter.supportCoordinate

/-- Threshold-proximity coordinate currently tracked by the reduced order
parameter. In the current model this is uncertainty mass. -/
def LocalOrderParameter.thresholdProximity
    (parameter : LocalOrderParameter) : Nat :=
  parameter.uncertaintyCoordinate

/-- Instability indicator derived from the reduced order parameter. The current
reduced model reads corridor/opaque uncertainty as the first instability signal,
while explicit-path macrostates suppress it. -/
def LocalOrderParameter.instabilityIndicator
    (parameter : LocalOrderParameter) : Nat :=
  match parameter.macrostate with
  | .explicitPath => 0
  | .corridorEnvelope => parameter.uncertaintyCoordinate
  | .opaque => parameter.uncertaintyCoordinate

/-- Classify reduced control quantities without overstating them as proved
Lyapunov objects. -/
inductive ControlQuantityRole
  | boundedRankingCandidate
  | futureLyapunovCandidate
  | orderParameterAdjacent
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Controller-facing mean-field/control-fusion state derived from the reduced
summary plus explicit exogenous controller inputs. -/
structure MeanFieldState where
  fieldStrength : Nat
  relayAlignment : Nat
  riskAlignment : Nat
  deriving Repr, DecidableEq, BEq

/-- Slow-moving control state carried across rounds. -/
structure ControllerState where
  congestionPrice : Nat
  stabilityMargin : Nat
  deriving Repr, DecidableEq, BEq

/-- Control/regime state: local phase or operating-regime classification plus
its residual fit summary. -/
structure RegimeState where
  current : OperatingRegime
  residual : Nat
  deriving Repr, DecidableEq, BEq

/-- Control posture state derived from the inferred regime. -/
structure PostureState where
  current : RoutingPosture
  deriving Repr, DecidableEq, BEq

/-- Control/scoring state for local continuation ranking. -/
structure ScoredContinuationSet where
  primaryScore : Nat
  alternateScore : Nat
  deriving Repr, DecidableEq, BEq

/-- Publication/public-observable state: the bounded macro-observable exported
from local state. This is not router truth. -/
structure CorridorEnvelopeProjection where
  shape : CorridorShape
  support : Nat
  hopLower : Nat
  hopUpper : Nat
  deriving Repr, DecidableEq, BEq

/-- One destination-local round state for the unified observer-controller. -/
structure LocalState where
  posterior : PosteriorState
  summary : ReducedBeliefSummary
  orderParameter : LocalOrderParameter
  meanField : MeanFieldState
  controller : ControllerState
  regime : RegimeState
  posture : PostureState
  scored : ScoredContinuationSet
  projection : CorridorEnvelopeProjection
  deriving Repr, DecidableEq, BEq

/-! ## Structural Invariants -/

def BoundedNat (n : Nat) : Prop := n ≤ 1000

def FiniteBelief.weight (belief : FiniteBelief) : FieldHypothesis → Nat
  | .unknown => belief.unknownWeight
  | .unreachable => belief.unreachableWeight
  | .corridor => belief.corridorWeight
  | .explicitPath => belief.explicitPathWeight

def FiniteBelief.totalWeight (belief : FiniteBelief) : Nat :=
  belief.unknownWeight + belief.unreachableWeight + belief.corridorWeight +
    belief.explicitPathWeight

def FiniteBelief.supportMass (belief : FiniteBelief) : Nat :=
  min (belief.corridorWeight + belief.explicitPathWeight) 1000

def FiniteBelief.uncertaintyMass (belief : FiniteBelief) : Nat :=
  min (belief.unknownWeight + belief.unreachableWeight) 1000

def PosteriorState.support (state : PosteriorState) : Nat :=
  state.belief.supportMass

def PosteriorState.entropy (state : PosteriorState) : Nat :=
  state.belief.uncertaintyMass

def PosteriorBounded (state : PosteriorState) : Prop :=
  BoundedNat state.support ∧ BoundedNat state.entropy

def ReducedBeliefSummaryBounded (state : ReducedBeliefSummary) : Prop :=
  BoundedNat state.supportMass ∧ BoundedNat state.uncertaintyMass

def LocalOrderParameterBounded (parameter : LocalOrderParameter) : Prop :=
  BoundedNat parameter.supportCoordinate ∧
    BoundedNat parameter.uncertaintyCoordinate

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
    ReducedBeliefSummaryBounded state.summary ∧
    LocalOrderParameterBounded state.orderParameter ∧
    MeanFieldBounded state.meanField ∧
    ControllerBounded state.controller ∧
    RegimeBounded state.regime ∧
    ContinuationScoresBounded state.scored ∧
    ProjectionBounded state.projection

/-- Conservative quantitative summary of how much local uncertainty and control
pressure remain to be worked off. This is only a bounded ranking candidate for
later paper-2-style analysis, not yet a proved Lyapunov function. -/
def UncertaintyBurden (state : LocalState) : Nat :=
  state.posterior.entropy + state.controller.congestionPrice + state.regime.residual

/-- Current classification of `UncertaintyBurden`. It is deliberately kept
separate from the order parameter itself: the quantity is downstream and
control-adjacent, not the order parameter. -/
def uncertaintyBurdenRole : ControlQuantityRole :=
  .orderParameterAdjacent

/-- The local model is harmonious when the downstream states remain subordinate
to the posterior and shared projection. -/
def Harmony (state : LocalState) : Prop :=
  state.meanField.fieldStrength = state.posterior.support ∧
    state.controller.stabilityMargin = state.meanField.fieldStrength ∧
    (state.projection.shape = CorridorShape.explicitPath ↔
      state.posterior.knowledge = ReachabilityKnowledge.explicitPath) ∧
    state.projection.support ≤ state.posterior.support ∧
    state.scored.alternateScore ≤ state.scored.primaryScore ∧
    state.summary.supportMass = state.posterior.support ∧
    state.summary.uncertaintyMass = state.posterior.entropy ∧
    state.orderParameter.supportCoordinate = state.summary.supportMass ∧
    state.orderParameter.uncertaintyCoordinate = state.summary.uncertaintyMass ∧
    state.orderParameter.macrostate = state.summary.publicMacrostate

/-! ## Abstract Operations -/

class Model where
  updatePosterior : EvidenceInput → LocalState → PosteriorState
  bayesianPosterior : EvidenceInput → LocalState → ProbabilisticRouteBelief
  reducePosterior :
    PosteriorState → ProbabilisticRouteBelief → ReducedBeliefSummary
  extractOrderParameter : ReducedBeliefSummary → LocalOrderParameter
  compressMeanField : EvidenceInput → ReducedBeliefSummary → MeanFieldState
  updateController : EvidenceInput → MeanFieldState → ControllerState → ControllerState
  inferRegime :
    LocalOrderParameter → MeanFieldState → ControllerState → RegimeState
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

def bayesianPosterior
    (evidence : EvidenceInput)
    (state : LocalState) : ProbabilisticRouteBelief :=
  Model.bayesianPosterior evidence state

def reducePosterior
    (posterior : PosteriorState)
    (belief : ProbabilisticRouteBelief) : ReducedBeliefSummary :=
  Model.reducePosterior posterior belief

def extractOrderParameter
    (summary : ReducedBeliefSummary) : LocalOrderParameter :=
  Model.extractOrderParameter summary

def compressMeanField
    (evidence : EvidenceInput)
    (summary : ReducedBeliefSummary) : MeanFieldState :=
  Model.compressMeanField evidence summary

def updateController
    (evidence : EvidenceInput)
    (meanField : MeanFieldState)
    (controller : ControllerState) : ControllerState :=
  Model.updateController evidence meanField controller

def inferRegime
    (orderParameter : LocalOrderParameter)
    (meanField : MeanFieldState)
    (controller : ControllerState) : RegimeState :=
  Model.inferRegime orderParameter meanField controller

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

structure BayesianRoundView where
  posteriorBelief : ProbabilisticRouteBelief
  reducedSummary : ReducedBeliefSummary
  nextState : LocalState

section BayesianRoundView

variable [Model]

def bayesianRoundView
    (evidence : EvidenceInput)
    (state : LocalState) : BayesianRoundView :=
  let posterior := updatePosterior evidence state
  let belief := bayesianPosterior evidence state
  let reduced := reducePosterior posterior belief
  { posteriorBelief := belief
    reducedSummary := reduced
    nextState := roundStep evidence state }

end BayesianRoundView

/-! ## Compression Boundary Vocabulary -/

abbrev CompressionPreserves
    {source compressed observed : Type*}
    (compress : source → compressed)
    (observeSource : source → observed)
    (observeCompressed : compressed → observed) : Prop :=
  ∀ value, observeCompressed (compress value) = observeSource value

abbrev CompressionSufficientFor
    {source compressed result : Type*}
    (compress : source → compressed)
    (runCompressed : compressed → result)
    (runSource : source → result) : Prop :=
  ∀ value, runCompressed (compress value) = runSource value

abbrev CompressionConservative
    {source compressed observed : Type*}
    (compress : source → compressed)
    (observeCompressed : compressed → observed)
    (observeSource : source → observed)
    (leq : observed → observed → Prop) : Prop :=
  ∀ value, leq (observeCompressed (compress value)) (observeSource value)

abbrev ReductionDivergenceHook
    (M : Model)
    (posteriorMetric :
      (PosteriorState × ProbabilisticRouteBelief) →
        (PosteriorState × ProbabilisticRouteBelief) → ℝ)
    (summaryMetric : ReducedBeliefSummary → ReducedBeliefSummary → ℝ) : Prop :=
  ∀ left right,
    summaryMetric
        (@Model.reducePosterior M left.1 left.2)
        (@Model.reducePosterior M right.1 right.2) ≤
      posteriorMetric left right

abbrev ReductionComparisonHook
    (M : Model)
    (summaryStatistic : ReducedBeliefSummary → Nat)
    (posteriorStatistic : PosteriorState → Nat) : Prop :=
  ∀ posterior belief,
    summaryStatistic (@Model.reducePosterior M posterior belief) ≤
      posteriorStatistic posterior

abbrev RoundPreservesBounded (M : Model) : Prop :=
  ∀ evidence state, StateBounded (@Model.roundStep M evidence state)

abbrev BayesianPosteriorNormalized (M : Model) : Prop :=
  ∀ evidence state, ∑ h, (@Model.bayesianPosterior M evidence state).pmf h = 1

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
  bayesian_posterior_normalized : BayesianPosteriorNormalized toModel
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

theorem bayesian_posterior_normalized
    (evidence : EvidenceInput)
    (state : LocalState) :
    ∑ h, (bayesianPosterior evidence state).pmf h = 1 :=
  Laws.bayesian_posterior_normalized evidence state

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
