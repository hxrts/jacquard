import Field.Adequacy.Probabilistic

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyProbabilisticFixtures

open FieldInformationBayesian
open FieldInformationCalibration
open FieldInformationProbabilistic
open FieldModelAPI
open FieldModelInstance
open FieldRouterProbabilistic

def fixtureExplicitHypothesis : ProbabilisticRouteHypothesis :=
  defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath

def fixtureExplicitObservation : ProbabilisticRouteObservation :=
  observationOfEvidence explicitPathEvidence initialState

theorem fixture_explicit_bayesian_update_gives_positive_posterior_mass :
    (bayesianPosteriorBelief
        (priorBeliefOfPosteriorState initialState.posterior)
        (observationOfEvidence explicitPathEvidence initialState)).pmf
          (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) > 0 := by
  exact trusted_explicit_observation_supports_explicit_hypothesis initialState

theorem fixture_correlated_evidence_boundary_marked :
    CorrelatedObservationRegimeOutOfScope correlatedLikelihoodPerturbation := by
  exact correlated_calibration_remains_out_of_scope

noncomputable def miscalibratedZeroLikelihood
    (_observation : ProbabilisticRouteObservation)
    (_hypothesis : ProbabilisticRouteHypothesis) : ℝ :=
  0

theorem fixture_miscalibrated_likelihood_differs_from_model :
    ∃ observation hypothesis,
      miscalibratedZeroLikelihood observation hypothesis ≠
        observationLikelihood observation hypothesis := by
  refine ⟨fixtureExplicitObservation, fixtureExplicitHypothesis, ?_⟩
  unfold miscalibratedZeroLikelihood fixtureExplicitObservation fixtureExplicitHypothesis
    observationLikelihood existenceLikelihood knowledgeLikelihood
    deliveryLikelihood witnessLikelihood observationOfEvidence
    defaultHypothesisOfKnowledge
  simp [explicitPathEvidence]

theorem fixture_sparse_evidence_guardrail_blocks_explicit_confidence :
    posteriorConfidenceDecision conservativePosteriorThresholds
        conservativePosteriorThresholds_admissible corridorPosteriorBelief ≠
      PosteriorRoutingDecision.explicitPath := by
  simp [corridorPosteriorBelief_decides_corridor]

end FieldAdequacyProbabilisticFixtures
