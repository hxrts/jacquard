import Field.Model.Instance
import Field.Information.Bayesian
import Field.Information.Probabilistic

/-! # Model.Refinement — conservation and projection properties of the local belief model -/

/-
Round projections conserve support mass and do not manufacture explicit-path claims;
Bayesian posteriors remain normalised after observation.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldModelRefinement

open FieldModelAPI
open FieldInformationBayesian
open FieldInformationProbabilistic
open FieldModelInstance

/-! ## Support Conservation -/

/-- The composed round still publishes a corridor projection whose support is
subordinate to the round-updated posterior support. -/
theorem round_projection_support_conservative
    (evidence : EvidenceInput)
    (state : LocalState) :
    (FieldModelAPI.roundStep evidence state).projection.support ≤
      (FieldModelAPI.roundStep evidence state).posterior.support :=
  (FieldModelAPI.multi_layer_projection_subordinate evidence state).2.2

/-- The composed round keeps the mean-field strength aligned to the updated
posterior support. -/
theorem round_mean_field_tracks_posterior_support
    (evidence : EvidenceInput)
    (state : LocalState) :
    (FieldModelAPI.roundStep evidence state).meanField.fieldStrength =
      (FieldModelAPI.roundStep evidence state).posterior.support :=
  (FieldModelAPI.multi_layer_projection_subordinate evidence state).1

/-! ## Explicit-Path Honesty -/

/-- The stronger explicit-path claim still has to pass through the round-updated
posterior knowledge state; the projection cannot outrun that local knowledge. -/
theorem explicit_projection_requires_explicit_round_knowledge
    (evidence : EvidenceInput)
    (state : LocalState)
    (hShape :
      (FieldModelAPI.roundStep evidence state).projection.shape =
        CorridorShape.explicitPath) :
    (FieldModelAPI.roundStep evidence state).posterior.knowledge =
      ReachabilityKnowledge.explicitPath :=
  FieldModelAPI.explicit_projection_requires_explicit_knowledge evidence state hShape

/-! ## Posterior Normalisation -/

/-- The API-exposed Bayesian posterior remains normalized on every local round. -/
theorem bayesian_round_preserves_normalization
    (evidence : EvidenceInput)
    (state : LocalState) :
    ∑ h, (FieldModelAPI.bayesianPosterior evidence state).pmf h = 1 :=
  FieldModelAPI.bayesian_posterior_normalized evidence state

/-- Strong explicit-path evidence gives positive posterior mass to the
explicit-path Bayesian hypothesis because the smoothed prior always retains
some admissible mass there and the explicit-path observation likelihood is
strictly positive. -/
theorem explicit_path_evidence_supports_explicit_bayesian_hypothesis
    (state : LocalState) :
    0 <
      (FieldModelAPI.bayesianPosterior explicitPathEvidence state).pmf
        (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) := by
  simpa [FieldModelAPI.bayesianPosterior, FieldModelInstance.bayesianPosteriorImpl]
    using
      bayesianPosterior_positive_of_positive_prior_and_likelihood
        (priorBeliefOfPosteriorState state.posterior)
        (observationOfEvidence explicitPathEvidence state)
        (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath)
        (priorBeliefOfPosteriorState_positive state.posterior
          (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath))
        (by
          unfold observationLikelihood existenceLikelihood knowledgeLikelihood
            deliveryLikelihood witnessLikelihood observationOfEvidence
            defaultHypothesisOfKnowledge
          simp [explicitPathEvidence])

/-- When a strong explicit-path round also publishes an explicit-path corridor
projection, the public claim is backed by positive posterior mass for the
explicit-path Bayesian hypothesis. -/
theorem explicit_round_projection_requires_positive_bayesian_mass
    (evidence : EvidenceInput)
    (state : LocalState)
    (hShape :
      (FieldModelAPI.roundStep evidence state).projection.shape =
        CorridorShape.explicitPath) :
    0 <
      (FieldModelAPI.bayesianPosterior evidence state).pmf
        (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) := by
  have hRoundKnowledge :
      (FieldModelAPI.roundStep evidence state).posterior.knowledge =
        ReachabilityKnowledge.explicitPath :=
    explicit_projection_requires_explicit_round_knowledge evidence state hShape
  have hObservedExplicit :
      (observationOfEvidence evidence state).observedKnowledge =
        FieldHypothesis.explicitPath := by
    have hNext :
        nextKnowledge evidence.reachability state.posterior.knowledge =
          ReachabilityKnowledge.explicitPath := by
      simpa [FieldModelAPI.roundStep, roundStepImpl, updatePosteriorImpl] using hRoundKnowledge
    cases hReach : evidence.reachability <;>
      simp [observationOfEvidence, hReach] at hNext ⊢
    case unknown =>
      cases hNext
    case unreachable =>
      cases hNext
    case corridorOnly =>
      cases hNext
    case preserve =>
      cases hStateKnowledge : state.posterior.knowledge <;>
        simp [hStateKnowledge] at hNext ⊢
      case unknown =>
        cases hNext
      case unreachable =>
        cases hNext
      case corridor =>
        cases hNext
  simpa [FieldModelAPI.bayesianPosterior, FieldModelInstance.bayesianPosteriorImpl]
    using
      bayesianPosterior_positive_of_positive_prior_and_likelihood
        (priorBeliefOfPosteriorState state.posterior)
        (observationOfEvidence evidence state)
        (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath)
        (priorBeliefOfPosteriorState_positive state.posterior
          (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath))
        (by
          have hExistence :
              0 <
                existenceLikelihood
                  (observationOfEvidence evidence state)
                  (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) := by
            unfold existenceLikelihood defaultHypothesisOfKnowledge
            simp
          have hKnowledgeFactor :
              0 <
                knowledgeLikelihood
                  (observationOfEvidence evidence state)
                  (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) := by
            unfold knowledgeLikelihood defaultHypothesisOfKnowledge
            simp [hObservedExplicit]
          have hDelivery :
              0 <
                deliveryLikelihood
                  (observationOfEvidence evidence state)
                  (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) := by
            unfold deliveryLikelihood observationOfEvidence defaultHypothesisOfKnowledge
            cases evidence.refresh <;> simp
            case explicitRefresh =>
              by_cases hPressure : 500 < evidence.controllerPressure <;> simp [hPressure]
          have hWitness :
              0 <
                witnessLikelihood
                  (observationOfEvidence evidence state)
                  (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) := by
            unfold witnessLikelihood
            have hCase :
                0 <
                  (if
                      (match evidence.feedback with
                        | EvidenceFeedback.none => ObservationReliabilityBand.noisy
                        | EvidenceFeedback.weakReverse => ObservationReliabilityBand.corroborated
                        | EvidenceFeedback.strongReverse => ObservationReliabilityBand.trusted) =
                        ObservationReliabilityBand.trusted then
                    (1 : ℝ)
                  else
                    (1 / 2 : ℝ)) := by
              by_cases hTrusted :
                  (match evidence.feedback with
                    | EvidenceFeedback.none => ObservationReliabilityBand.noisy
                    | EvidenceFeedback.weakReverse => ObservationReliabilityBand.corroborated
                    | EvidenceFeedback.strongReverse => ObservationReliabilityBand.trusted) =
                    ObservationReliabilityBand.trusted
              · norm_num [hTrusted]
              · norm_num [hTrusted]
            simpa [observationOfEvidence, defaultHypothesisOfKnowledge] using hCase
          unfold observationLikelihood
          exact mul_pos (mul_pos (mul_pos hExistence hKnowledgeFactor) hDelivery) hWitness)

theorem explicit_path_projection_requires_positive_bayesian_mass
    (state : LocalState)
    (hShape :
      (FieldModelAPI.roundStep explicitPathEvidence state).projection.shape =
        CorridorShape.explicitPath) :
    0 <
      (FieldModelAPI.bayesianPosterior explicitPathEvidence state).pmf
        (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) := by
  exact explicit_round_projection_requires_positive_bayesian_mass
    explicitPathEvidence state hShape

/-- Repeating the same strong explicit-path evidence preserves positive
posterior mass for the explicit-path Bayesian hypothesis at every later local
state visited by the reduced round dynamics. -/
theorem repeated_explicit_path_evidence_preserves_positive_bayesian_mass
    (steps : Nat)
    (state : LocalState) :
    0 <
      (FieldModelAPI.bayesianPosterior explicitPathEvidence
        (runRepeatedEvidence steps explicitPathEvidence state)).pmf
          (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) := by
  exact explicit_path_evidence_supports_explicit_bayesian_hypothesis
    (runRepeatedEvidence steps explicitPathEvidence state)

/-- Once the reduced model sees explicit-path evidence twice in a row, the
shared projection stays explicit-path. -/
theorem repeated_explicit_path_rounds_stabilize
    (state : LocalState) :
    (FieldModelAPI.roundStep explicitPathEvidence
        (FieldModelAPI.roundStep explicitPathEvidence state)).projection.shape =
      CorridorShape.explicitPath := by
  simpa [FieldModelAPI.roundStep] using
    repeated_explicit_path_rounds_preserve_projection 1 state

end FieldModelRefinement
