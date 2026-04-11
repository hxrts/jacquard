import Field.Information.Probabilistic
import Mathlib.Data.Fintype.BigOperators
import Mathlib.Data.Real.Basic
import Mathlib.Tactic

/-! # Information.Bayesian — observation classes, probabilistic route observations, and prior belief operations -/

/-
Define how raw observations are classified by delivery mode and how uniform/averaged priors
are constructed over route hypothesis spaces.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldInformationBayesian

open FieldInformationProbabilistic
open FieldModelAPI
open EntropyAPI
open scoped BigOperators

/-! ## Observation Classes -/

inductive ObservationDeliveryClass
  | missing
  | delayed
  | delivered
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

structure ProbabilisticRouteObservation where
  observedKnowledge : FieldHypothesis
  delivery : ObservationDeliveryClass
  witnessReliability : ObservationReliabilityBand
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

/-! ## Prior Construction -/

noncomputable def uniformPriorBelief : ProbabilisticRouteBelief :=
  { distribution :=
      { pmf := fun _ => 1 / (Fintype.card ProbabilisticRouteHypothesis : ℝ)
        nonneg := by
          intro _
          positivity
        sum_one := by
          have hCardNe : (Fintype.card ProbabilisticRouteHypothesis : ℝ) ≠ 0 := by
            positivity
          calc
            ∑ h, (1 / (Fintype.card ProbabilisticRouteHypothesis : ℝ))
                =
              (Fintype.card ProbabilisticRouteHypothesis : ℝ) *
                (1 / (Fintype.card ProbabilisticRouteHypothesis : ℝ)) := by
                  simp
            _ = 1 := by
                  field_simp [hCardNe] } }

theorem uniformPriorBelief_positive
    (hypothesis : ProbabilisticRouteHypothesis) :
    0 < uniformPriorBelief.pmf hypothesis := by
  unfold uniformPriorBelief ProbabilisticRouteBelief.pmf
  positivity

noncomputable def deterministicPriorBelief
    (hypothesis : ProbabilisticRouteHypothesis) : ProbabilisticRouteBelief :=
  pointHypothesisBelief hypothesis

noncomputable def averagedBelief
    (left right : ProbabilisticRouteBelief) : ProbabilisticRouteBelief :=
  { distribution :=
      { pmf := fun hypothesis => (left.pmf hypothesis + right.pmf hypothesis) / 2
        nonneg := by
          intro hypothesis
          have hLeft := ProbabilisticRouteBelief.nonneg left hypothesis
          have hRight := ProbabilisticRouteBelief.nonneg right hypothesis
          positivity
        sum_one := by
          calc
            ∑ h, (left.pmf h + right.pmf h) / 2
                = (∑ h, (left.pmf h + right.pmf h)) / 2 := by
                    rw [Finset.sum_div]
            _ = ((∑ h, left.pmf h) + (∑ h, right.pmf h)) / 2 := by
                    rw [Finset.sum_add_distrib]
            _ = (1 + 1) / 2 := by
                    simp [ProbabilisticRouteBelief.sum_one]
            _ = 1 := by norm_num } }

theorem averagedBelief_positive_of_right_positive
    (left right : ProbabilisticRouteBelief)
    (hypothesis : ProbabilisticRouteHypothesis)
    (hRight : 0 < right.pmf hypothesis) :
    0 < (averagedBelief left right).pmf hypothesis := by
  unfold averagedBelief ProbabilisticRouteBelief.pmf
  have hLeftNonneg : 0 ≤ left.pmf hypothesis := ProbabilisticRouteBelief.nonneg left hypothesis
  have hNumPos : 0 < left.pmf hypothesis + right.pmf hypothesis := by
    linarith
  have hTwoPos : 0 < (2 : ℝ) := by norm_num
  exact div_pos hNumPos hTwoPos

def defaultHypothesisOfKnowledge
    (knowledge : ReachabilityKnowledge) : ProbabilisticRouteHypothesis :=
  { existence := .present
    quality :=
      match knowledge with
      | .explicitPath => .high
      | .corridor => .medium
      | .unknown => .low
      | .unreachable => .low
    transportReliability :=
      match knowledge with
      | .explicitPath => .reliable
      | .corridor => .delayed
      | .unknown => .lossy
      | .unreachable => .lossy
    observationReliability :=
      match knowledge with
      | .explicitPath => .trusted
      | .corridor => .corroborated
      | .unknown => .noisy
      | .unreachable => .trusted
    knowledge :=
      match knowledge with
      | .unknown => .unknown
      | .unreachable => .unreachable
      | .corridor => .corridor
      | .explicitPath => .explicitPath }

noncomputable def priorBeliefOfPosteriorState
    (state : PosteriorState) : ProbabilisticRouteBelief :=
  averagedBelief
    (deterministicPriorBelief (defaultHypothesisOfKnowledge state.knowledge))
    uniformPriorBelief

theorem priorBeliefOfPosteriorState_positive
    (state : PosteriorState)
    (hypothesis : ProbabilisticRouteHypothesis) :
    0 < (priorBeliefOfPosteriorState state).pmf hypothesis := by
  unfold priorBeliefOfPosteriorState
  exact averagedBelief_positive_of_right_positive
    (deterministicPriorBelief (defaultHypothesisOfKnowledge state.knowledge))
    uniformPriorBelief
    hypothesis
    (uniformPriorBelief_positive hypothesis)

def observationOfEvidence
    (evidence : EvidenceInput)
    (state : LocalState) : ProbabilisticRouteObservation :=
  { observedKnowledge :=
      match evidence.reachability with
      | .preserve =>
          match state.posterior.knowledge with
          | .unknown => .unknown
          | .unreachable => .unreachable
          | .corridor => .corridor
          | .explicitPath => .explicitPath
      | .unknown => .unknown
      | .unreachable => .unreachable
      | .corridorOnly => .corridor
      | .explicitPath => .explicitPath
    delivery :=
      match evidence.refresh with
      | .unchanged => .missing
      | .explicitRefresh =>
          if evidence.controllerPressure > 500 then .delayed else .delivered
    witnessReliability :=
      match evidence.feedback with
      | .none => .noisy
      | .weakReverse => .corroborated
      | .strongReverse => .trusted }

noncomputable def knowledgeLikelihood
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) : ℝ :=
  if observation.observedKnowledge = hypothesis.knowledge then
    1
  else if observation.witnessReliability = .trusted then
    0
  else if observation.witnessReliability = .corroborated then
    (1 / 4 : ℝ)
  else
    (1 / 2 : ℝ)

theorem knowledgeLikelihood_nonneg
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) :
    0 ≤ knowledgeLikelihood observation hypothesis := by
  unfold knowledgeLikelihood
  by_cases hMatch : observation.observedKnowledge = hypothesis.knowledge
  · simp [hMatch]
  · by_cases hTrusted : observation.witnessReliability = .trusted
    · simp [hMatch, hTrusted]
    · by_cases hCorroborated : observation.witnessReliability = .corroborated
      · simp [hMatch, hCorroborated]
      · simp [hMatch, hTrusted, hCorroborated]

def existenceLikelihood
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) : ℝ :=
  match hypothesis.existence, observation.observedKnowledge with
  | .absent, .unknown => 1
  | .absent, _ => 0
  | .present, _ => 1

theorem existenceLikelihood_nonneg
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) :
    0 ≤ existenceLikelihood observation hypothesis := by
  cases observation with
  | mk observedKnowledge delivery witnessReliability =>
      cases hypothesis with
      | mk existence quality transportReliability observationReliability knowledge =>
          cases existence <;> cases observedKnowledge <;>
            norm_num [existenceLikelihood]

noncomputable def deliveryLikelihood
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) : ℝ :=
  match observation.delivery, hypothesis.transportReliability with
  | .delivered, .reliable => 1
  | .delivered, .delayed => (3 / 4 : ℝ)
  | .delivered, .lossy => (1 / 4 : ℝ)
  | .delayed, .reliable => (1 / 4 : ℝ)
  | .delayed, .delayed => 1
  | .delayed, .lossy => (1 / 2 : ℝ)
  | .missing, .reliable => (1 / 4 : ℝ)
  | .missing, .delayed => (1 / 2 : ℝ)
  | .missing, .lossy => 1

theorem deliveryLikelihood_nonneg
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) :
    0 ≤ deliveryLikelihood observation hypothesis := by
  cases observation with
  | mk observedKnowledge delivery witnessReliability =>
      cases hypothesis with
      | mk existence quality transportReliability observationReliability knowledge =>
          cases delivery <;> cases transportReliability <;>
            norm_num [deliveryLikelihood]

noncomputable def witnessLikelihood
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) : ℝ :=
  if observation.witnessReliability = hypothesis.observationReliability then
    1
  else
    (1 / 2 : ℝ)

theorem witnessLikelihood_nonneg
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) :
    0 ≤ witnessLikelihood observation hypothesis := by
  unfold witnessLikelihood
  by_cases hEq : observation.witnessReliability = hypothesis.observationReliability
  · simp [hEq]
  · simp [hEq]

noncomputable def observationLikelihood
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) : ℝ :=
  existenceLikelihood observation hypothesis *
    knowledgeLikelihood observation hypothesis *
    deliveryLikelihood observation hypothesis *
    witnessLikelihood observation hypothesis

abbrev FactorizedLikelihoodModel : Prop :=
  ∀ observation hypothesis,
    observationLikelihood observation hypothesis =
      existenceLikelihood observation hypothesis *
        knowledgeLikelihood observation hypothesis *
        deliveryLikelihood observation hypothesis *
        witnessLikelihood observation hypothesis

theorem observationLikelihood_factorized : FactorizedLikelihoodModel := by
  intro observation hypothesis
  rfl

def CorrelatedObservationRegimeOutOfScope
    (jointLikelihood : ProbabilisticRouteObservation → ProbabilisticRouteHypothesis → ℝ) : Prop :=
  ∃ observation hypothesis,
    jointLikelihood observation hypothesis ≠ observationLikelihood observation hypothesis

theorem observationLikelihood_nonneg
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) :
    0 ≤ observationLikelihood observation hypothesis := by
  unfold observationLikelihood
  exact mul_nonneg
    (mul_nonneg
      (mul_nonneg
        (existenceLikelihood_nonneg observation hypothesis)
        (knowledgeLikelihood_nonneg observation hypothesis))
      (deliveryLikelihood_nonneg observation hypothesis))
    (witnessLikelihood_nonneg observation hypothesis)

noncomputable def posteriorWeight
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) : ℝ :=
  prior.pmf hypothesis * observationLikelihood observation hypothesis

theorem posteriorWeight_nonneg
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis) :
    0 ≤ posteriorWeight prior observation hypothesis := by
  unfold posteriorWeight
  exact mul_nonneg (ProbabilisticRouteBelief.nonneg prior hypothesis)
    (observationLikelihood_nonneg observation hypothesis)

noncomputable def posteriorNormalizer
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation) : ℝ :=
  ∑ h, posteriorWeight prior observation h

theorem posteriorNormalizer_nonneg
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation) :
    0 ≤ posteriorNormalizer prior observation := by
  unfold posteriorNormalizer
  exact Finset.sum_nonneg (fun h _ => posteriorWeight_nonneg prior observation h)

theorem posteriorNormalizer_pos_of_positive_weight
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis)
    (hWeight : 0 < posteriorWeight prior observation hypothesis) :
    0 < posteriorNormalizer prior observation := by
  unfold posteriorNormalizer
  have hLe :
      posteriorWeight prior observation hypothesis ≤
        ∑ h, posteriorWeight prior observation h := by
    exact Finset.single_le_sum
      (fun h _ => posteriorWeight_nonneg prior observation h)
      (by simp)
  linarith

noncomputable def bayesianPosteriorBelief
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation) : ProbabilisticRouteBelief :=
  let z := posteriorNormalizer prior observation
  { distribution :=
      { pmf := fun hypothesis =>
          if hZero : z = 0 then
            prior.pmf hypothesis
          else
            posteriorWeight prior observation hypothesis / z
        nonneg := by
          intro hypothesis
          by_cases hZero : z = 0
          · simp [hZero, z]
            exact ProbabilisticRouteBelief.nonneg prior hypothesis
          · simp [hZero, z]
            exact div_nonneg
              (posteriorWeight_nonneg prior observation hypothesis)
              (posteriorNormalizer_nonneg prior observation)
        sum_one := by
          by_cases hZero : z = 0
          · simpa [hZero, z] using ProbabilisticRouteBelief.sum_one prior
          · have hz : z ≠ 0 := hZero
            calc
              ∑ h,
                  (if hZero : z = 0 then prior.pmf h else posteriorWeight prior observation h / z)
                    =
                ∑ h, posteriorWeight prior observation h / z := by
                  simp [hZero, z]
              _ = (∑ h, posteriorWeight prior observation h) / z := by
                  rw [Finset.sum_div]
              _ = z / z := by
                  rfl
              _ = 1 := by
                  field_simp [hz] } }

theorem bayesianPosteriorBelief_sum_one
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation) :
    ∑ h, (bayesianPosteriorBelief prior observation).pmf h = 1 := by
  exact ProbabilisticRouteBelief.sum_one (bayesianPosteriorBelief prior observation)

theorem bayesianPosterior_positive_of_positive_prior_and_likelihood
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis)
    (hPrior : 0 < prior.pmf hypothesis)
    (hLikelihood : 0 < observationLikelihood observation hypothesis) :
    0 < (bayesianPosteriorBelief prior observation).pmf hypothesis := by
  have hWeightPos : 0 < posteriorWeight prior observation hypothesis := by
    unfold posteriorWeight
    nlinarith
  have hNormPos : 0 < posteriorNormalizer prior observation :=
    posteriorNormalizer_pos_of_positive_weight prior observation hypothesis hWeightPos
  have hNormNe : posteriorNormalizer prior observation ≠ 0 := ne_of_gt hNormPos
  unfold bayesianPosteriorBelief
  simp [hNormNe, posteriorWeight]
  exact div_pos hWeightPos hNormPos

theorem bayesianPosterior_support_requires_prior_and_likelihood
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation)
    (hypothesis : ProbabilisticRouteHypothesis)
    (hPos : 0 < (bayesianPosteriorBelief prior observation).pmf hypothesis) :
    0 < prior.pmf hypothesis ∧
      (posteriorNormalizer prior observation = 0 ∨
        0 < observationLikelihood observation hypothesis) := by
  by_cases hZero : posteriorNormalizer prior observation = 0
  · simpa [bayesianPosteriorBelief, hZero] using hPos
  · have hzPos : 0 < posteriorNormalizer prior observation := by
      exact lt_of_le_of_ne
        (posteriorNormalizer_nonneg prior observation)
        (Ne.symm hZero)
    have hDivPos : 0 < posteriorWeight prior observation hypothesis /
        posteriorNormalizer prior observation := by
      simpa [bayesianPosteriorBelief, hZero] using hPos
    have hWeightPos : 0 < posteriorWeight prior observation hypothesis := by
      by_contra hWeightNotPos
      have hWeightLe : posteriorWeight prior observation hypothesis ≤ 0 :=
        le_of_not_gt hWeightNotPos
      have hDivLe :
          posteriorWeight prior observation hypothesis /
              posteriorNormalizer prior observation ≤ 0 := by
        exact div_nonpos_of_nonpos_of_nonneg hWeightLe hzPos.le
      linarith
    have hPriorPos : 0 < prior.pmf hypothesis := by
      by_contra hPriorNotPos
      have hPriorLe : prior.pmf hypothesis ≤ 0 := le_of_not_gt hPriorNotPos
      have hWeightLe : posteriorWeight prior observation hypothesis ≤ 0 := by
        unfold posteriorWeight
        exact mul_nonpos_of_nonpos_of_nonneg hPriorLe
          (observationLikelihood_nonneg observation hypothesis)
      linarith
    have hLikelihoodPos : 0 < observationLikelihood observation hypothesis := by
      by_contra hLikeNotPos
      have hLikeLe : observationLikelihood observation hypothesis ≤ 0 := le_of_not_gt hLikeNotPos
      have hWeightLe : posteriorWeight prior observation hypothesis ≤ 0 := by
        unfold posteriorWeight
        exact mul_nonpos_of_nonneg_of_nonpos
          (ProbabilisticRouteBelief.nonneg prior hypothesis) hLikeLe
      linarith
    exact ⟨hPriorPos, Or.inr hLikelihoodPos⟩

theorem bayesianPosterior_impossible_observation_falls_back_to_prior
    (prior : ProbabilisticRouteBelief)
    (observation : ProbabilisticRouteObservation)
    (hImpossible :
      ∀ hypothesis, observationLikelihood observation hypothesis = 0) :
    ∀ hypothesis,
      (bayesianPosteriorBelief prior observation).pmf hypothesis = prior.pmf hypothesis := by
  intro hypothesis
  have hZero :
      posteriorNormalizer prior observation = 0 := by
    unfold posteriorNormalizer
    simp [posteriorWeight, hImpossible]
  unfold bayesianPosteriorBelief
  simp [hZero]
  rfl

theorem bayesianPosterior_ofEvidence_updates_from_posterior_state
    (evidence : EvidenceInput)
    (state : LocalState) :
    bayesianPosteriorBelief
        (priorBeliefOfPosteriorState state.posterior)
        (observationOfEvidence evidence state) =
      bayesianPosteriorBelief
        (priorBeliefOfPosteriorState state.posterior)
        (observationOfEvidence evidence state) := by
  rfl

end FieldInformationBayesian
