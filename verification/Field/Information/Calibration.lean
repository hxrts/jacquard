import Field.Model.Refinement
import Field.Router.Probabilistic

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldInformationCalibration

open FieldInformationBayesian
open FieldInformationProbabilistic
open FieldModelAPI
open FieldModelInstance
open FieldModelRefinement
open FieldRouterProbabilistic

inductive ProbabilisticCalibrationTarget
  | confidenceThresholdValidity
  | posteriorProbabilityCalibration
  | expectedUtilityCorrectness
  | regretInterpretation
  deriving Inhabited, Repr, DecidableEq, BEq

def ConfidenceThresholdValid
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (belief : ProbabilisticRouteBelief) : Prop :=
  match posteriorConfidenceDecision thresholds hAdm belief with
  | .explicitPath =>
      (thresholds.explicitPathMin : ℝ) ≤ probabilisticExplicitPathMass belief
  | .corridorEnvelope =>
      (thresholds.corridorMin : ℝ) ≤ probabilisticCorridorCapableMass belief
  | .abstain => True

noncomputable def publicProjectionDecisionRank
    (belief : ProbabilisticRouteBelief) : Nat :=
  match probabilisticPublicProjection belief with
  | .opaque => 0
  | .corridorEnvelope => 1
  | .explicitPath => 2

theorem posteriorConfidenceDecision_is_confidence_threshold_valid
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (belief : ProbabilisticRouteBelief) :
    ConfidenceThresholdValid thresholds hAdm belief := by
  unfold ConfidenceThresholdValid
  cases hDecision : posteriorConfidenceDecision thresholds hAdm belief with
  | abstain =>
      trivial
  | corridorEnvelope =>
      exact
        (posteriorConfidenceDecision_corridor_implies_threshold
          thresholds hAdm belief hDecision).2
  | explicitPath =>
      exact
        posteriorConfidenceDecision_explicitPath_implies_threshold
          thresholds hAdm belief hDecision

theorem trusted_explicit_observation_supports_explicit_hypothesis
    (state : LocalState) :
    0 <
      (bayesianPosteriorBelief
          (priorBeliefOfPosteriorState state.posterior)
          (observationOfEvidence explicitPathEvidence state)).pmf
        (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) := by
  simpa [FieldModelAPI.bayesianPosterior, FieldModelInstance.bayesianPosteriorImpl] using
    explicit_path_evidence_supports_explicit_bayesian_hypothesis state

theorem positive_confidence_threshold_decision_public_projection_bound
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (hPositive : 0 < thresholds.corridorMin)
    (belief : ProbabilisticRouteBelief) :
    ((posteriorConfidenceDecision thresholds hAdm belief = .explicitPath →
        probabilisticPublicProjection belief = CorridorShape.explicitPath) ∧
      (posteriorConfidenceDecision thresholds hAdm belief = .corridorEnvelope →
        probabilisticPublicProjection belief = CorridorShape.corridorEnvelope ∨
          probabilisticPublicProjection belief = CorridorShape.explicitPath)) := by
  constructor
  · intro hDecision
    have hExplicit :
        (thresholds.explicitPathMin : ℝ) ≤ probabilisticExplicitPathMass belief :=
      posteriorConfidenceDecision_explicitPath_implies_threshold thresholds hAdm belief hDecision
    have hThresholdPos : (0 : ℝ) < (thresholds.explicitPathMin : ℝ) := by
      exact_mod_cast (lt_of_lt_of_le hPositive hAdm.2)
    have hMassPos : 0 < probabilisticExplicitPathMass belief := by
      linarith
    unfold probabilisticPublicProjection
    rw [if_pos hMassPos]
  · intro hDecision
    rcases posteriorConfidenceDecision_corridor_implies_threshold
      thresholds hAdm belief hDecision with ⟨_hExplicit, hCorridor⟩
    have hThresholdPos : (0 : ℝ) < thresholds.corridorMin := by
      exact_mod_cast hPositive
    have hMassPos : 0 < probabilisticCorridorCapableMass belief := by
      linarith
    unfold probabilisticPublicProjection
    by_cases hExplicitMass : 0 < probabilisticExplicitPathMass belief
    · right
      rw [if_pos hExplicitMass]
    · left
      rw [if_neg hExplicitMass]
      rw [if_pos hMassPos]

noncomputable def correlatedLikelihoodPerturbation
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) : ℝ :=
  2 * observationLikelihood observation hypothesis

def PosteriorProbabilityCalibrated
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) : Prop :=
  let z := posteriorNormalizer prior observation
  if hZero : z = 0 then
    (bayesianPosteriorBelief prior observation).pmf hypothesis = prior.pmf hypothesis
  else
    (bayesianPosteriorBelief prior observation).pmf hypothesis =
      posteriorWeight prior observation hypothesis / z

def ExpectedUtilityCorrect
    (belief : ProbabilisticRouteBelief)
    (decision : PosteriorRoutingDecision) : Prop :=
  posteriorDecisionExpectedUtility belief decision ≤
    posteriorBestDecisionExpectedUtility belief

def RegretInterpretationValid
    (belief : ProbabilisticRouteBelief)
    (decision : PosteriorRoutingDecision) : Prop :=
  0 ≤ posteriorDecisionRegret belief decision ∧
    (posteriorDecisionRegret belief decision = 0 →
      posteriorDecisionExpectedUtility belief decision =
        posteriorBestDecisionExpectedUtility belief)

theorem correlated_calibration_remains_out_of_scope :
    CorrelatedObservationRegimeOutOfScope correlatedLikelihoodPerturbation := by
  refine ⟨observationOfEvidence explicitPathEvidence initialState,
    defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath, ?_⟩
  unfold correlatedLikelihoodPerturbation
  have hPos :
      0 <
        observationLikelihood
          (observationOfEvidence explicitPathEvidence initialState)
          (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) := by
    unfold observationLikelihood existenceLikelihood knowledgeLikelihood
      deliveryLikelihood witnessLikelihood observationOfEvidence
    simp [explicitPathEvidence, defaultHypothesisOfKnowledge]
  linarith

theorem posterior_probability_calibrated
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) :
    PosteriorProbabilityCalibrated prior observation hypothesis := by
  unfold PosteriorProbabilityCalibrated
  by_cases hZero : posteriorNormalizer prior observation = 0
  · simp [hZero, bayesianPosteriorBelief, ProbabilisticRouteBelief.pmf]
  · simp [hZero, bayesianPosteriorBelief, ProbabilisticRouteBelief.pmf]

theorem expected_utility_correct_for_any_decision
    (belief : ProbabilisticRouteBelief)
    (decision : PosteriorRoutingDecision) :
    ExpectedUtilityCorrect belief decision := by
  unfold ExpectedUtilityCorrect
  cases decision with
  | abstain =>
      exact posteriorDecisionExpectedUtility_abstain_le_best belief
  | corridorEnvelope =>
      exact posteriorDecisionExpectedUtility_corridor_le_best belief
  | explicitPath =>
      exact posteriorDecisionExpectedUtility_explicit_le_best belief

theorem posterior_min_regret_decision_expected_utility_correct
    (belief : ProbabilisticRouteBelief) :
    ExpectedUtilityCorrect belief (posteriorMinRegretDecision belief) := by
  exact expected_utility_correct_for_any_decision belief (posteriorMinRegretDecision belief)

theorem regret_interpretation_valid_for_any_decision
    (belief : ProbabilisticRouteBelief)
    (decision : PosteriorRoutingDecision) :
    RegretInterpretationValid belief decision := by
  constructor
  · exact posteriorDecisionRegret_nonneg belief decision
  · intro hZero
    unfold posteriorDecisionRegret at hZero
    linarith

end FieldInformationCalibration
