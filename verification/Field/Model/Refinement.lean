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

/-! ## Reduction Preservation And Sufficiency -/

theorem reduced_summary_preserves_support_mass
    (posterior : PosteriorState)
    (belief : ProbabilisticRouteBelief) :
    (FieldModelAPI.reducePosterior posterior belief).supportMass =
      posterior.support := by
  rfl

theorem reduced_summary_preserves_uncertainty_mass
    (posterior : PosteriorState)
    (belief : ProbabilisticRouteBelief) :
    (FieldModelAPI.reducePosterior posterior belief).uncertaintyMass =
      posterior.entropy := by
  rfl

theorem reduced_summary_preserves_public_macrostate
    (posterior : PosteriorState)
    (belief : ProbabilisticRouteBelief) :
    (FieldModelAPI.reducePosterior posterior belief).publicMacrostate =
      match posterior.knowledge with
      | .explicitPath => CorridorShape.explicitPath
      | .corridor => CorridorShape.corridorEnvelope
      | .unknown => CorridorShape.opaque
      | .unreachable => CorridorShape.opaque := by
  change (FieldModelInstance.reducePosteriorImpl posterior belief).publicMacrostate =
    match posterior.knowledge with
    | .explicitPath => CorridorShape.explicitPath
    | .corridor => CorridorShape.corridorEnvelope
    | .unknown => CorridorShape.opaque
    | .unreachable => CorridorShape.opaque
  rfl

theorem extractOrderParameter_preserves_support_coordinate
    (posterior : PosteriorState)
    (belief : ProbabilisticRouteBelief) :
    (FieldModelAPI.extractOrderParameter (FieldModelAPI.reducePosterior posterior belief)).supportCoordinate =
      posterior.support := by
  change (FieldModelInstance.extractOrderParameterImpl
      (FieldModelInstance.reducePosteriorImpl posterior belief)).supportCoordinate =
    posterior.support
  rfl

theorem extractOrderParameter_preserves_uncertainty_coordinate
    (posterior : PosteriorState)
    (belief : ProbabilisticRouteBelief) :
    (FieldModelAPI.extractOrderParameter (FieldModelAPI.reducePosterior posterior belief)).uncertaintyCoordinate =
      posterior.entropy := by
  change (FieldModelInstance.extractOrderParameterImpl
      (FieldModelInstance.reducePosteriorImpl posterior belief)).uncertaintyCoordinate =
    posterior.entropy
  rfl

theorem extractOrderParameter_preserves_macrostate
    (posterior : PosteriorState)
    (belief : ProbabilisticRouteBelief) :
    (FieldModelAPI.extractOrderParameter (FieldModelAPI.reducePosterior posterior belief)).macrostate =
      (FieldModelAPI.reducePosterior posterior belief).publicMacrostate := by
  change (FieldModelInstance.extractOrderParameterImpl
      (FieldModelInstance.reducePosteriorImpl posterior belief)).macrostate =
    (FieldModelInstance.reducePosteriorImpl posterior belief).publicMacrostate
  rfl

theorem equal_reduced_summaries_yield_equal_mean_field_under_equal_pressure
    (left right : ReducedBeliefSummary)
    (leftEvidence rightEvidence : EvidenceInput)
    (hSummary : left = right)
    (hPressure : leftEvidence.controllerPressure = rightEvidence.controllerPressure) :
    FieldModelInstance.compressMeanFieldImpl leftEvidence left =
      FieldModelInstance.compressMeanFieldImpl rightEvidence right := by
  subst hSummary
  cases leftEvidence
  cases rightEvidence
  cases hPressure
  rfl

theorem equal_reduced_summaries_yield_equal_controller_updates_under_equal_pressure
    (left right : ReducedBeliefSummary)
    (leftEvidence rightEvidence : EvidenceInput)
    (controller : ControllerState)
    (hSummary : left = right)
    (hPressure : leftEvidence.controllerPressure = rightEvidence.controllerPressure) :
    FieldModelInstance.updateControllerImpl leftEvidence
        (FieldModelInstance.compressMeanFieldImpl leftEvidence left) controller =
      FieldModelInstance.updateControllerImpl rightEvidence
        (FieldModelInstance.compressMeanFieldImpl rightEvidence right) controller := by
  subst hSummary
  cases leftEvidence
  cases rightEvidence
  cases hPressure
  rfl

theorem equal_reduced_summaries_yield_equal_order_parameters
    (left right : ReducedBeliefSummary)
    (hSummary : left = right) :
    FieldModelAPI.extractOrderParameter left =
      FieldModelAPI.extractOrderParameter right := by
  subst hSummary
  rfl

theorem round_state_stores_reduced_summary
    (evidence : EvidenceInput)
    (state : LocalState) :
    (FieldModelAPI.roundStep evidence state).summary =
      FieldModelAPI.reducePosterior
        (FieldModelAPI.updatePosterior evidence state)
        (FieldModelAPI.bayesianPosterior evidence state) := by
  rfl

theorem round_state_stores_order_parameter
    (evidence : EvidenceInput)
    (state : LocalState) :
    (FieldModelAPI.roundStep evidence state).orderParameter =
      FieldModelAPI.extractOrderParameter
        (FieldModelAPI.roundStep evidence state).summary := by
  rfl

theorem round_state_mean_field_uses_stored_summary
    (evidence : EvidenceInput)
    (state : LocalState) :
    (FieldModelAPI.roundStep evidence state).meanField =
      FieldModelAPI.compressMeanField
        evidence
        (FieldModelAPI.roundStep evidence state).summary := by
  rfl

theorem round_state_regime_uses_stored_order_parameter
    (evidence : EvidenceInput)
    (state : LocalState) :
    (FieldModelAPI.roundStep evidence state).regime =
      FieldModelAPI.inferRegime
        (FieldModelAPI.roundStep evidence state).orderParameter
        (FieldModelAPI.roundStep evidence state).meanField
        (FieldModelAPI.roundStep evidence state).controller := by
  rfl

theorem round_state_stored_macrostate_chain
    (evidence : EvidenceInput)
    (state : LocalState) :
    (FieldModelAPI.roundStep evidence state).orderParameter.macrostate =
      (FieldModelAPI.roundStep evidence state).summary.publicMacrostate := by
  change
    (FieldModelInstance.extractOrderParameterImpl
      (FieldModelAPI.roundStep evidence state).summary).macrostate =
      (FieldModelAPI.roundStep evidence state).summary.publicMacrostate
  rfl

theorem reduced_summary_support_conservative
    (posterior : PosteriorState)
    (belief : ProbabilisticRouteBelief) :
    (FieldModelAPI.reducePosterior posterior belief).supportMass ≤ posterior.support := by
  simp [reduced_summary_preserves_support_mass]

theorem reduced_summary_uncertainty_conservative
    (posterior : PosteriorState)
    (belief : ProbabilisticRouteBelief) :
    (FieldModelAPI.reducePosterior posterior belief).uncertaintyMass ≤ posterior.entropy := by
  simp [reduced_summary_preserves_uncertainty_mass]

theorem reduced_summary_bounded
    (posterior : PosteriorState)
    (belief : ProbabilisticRouteBelief)
    (hPosterior : PosteriorBounded posterior) :
    ReducedBeliefSummaryBounded (FieldModelAPI.reducePosterior posterior belief) := by
  simpa [FieldModelAPI.reducePosterior] using
    FieldModelInstance.reducePosteriorImpl_bounded posterior belief hPosterior

theorem reduced_summary_support_monotone_of_posterior_support_monotone
    (left right : PosteriorState)
    (leftBelief rightBelief : ProbabilisticRouteBelief)
    (hSupport : left.support ≤ right.support) :
    (FieldModelAPI.reducePosterior left leftBelief).supportMass ≤
      (FieldModelAPI.reducePosterior right rightBelief).supportMass := by
  simpa [reduced_summary_preserves_support_mass, reduced_summary_preserves_support_mass] using hSupport

theorem reduced_summary_uncertainty_monotone_of_posterior_uncertainty_monotone
    (left right : PosteriorState)
    (leftBelief rightBelief : ProbabilisticRouteBelief)
    (hEntropy : left.entropy ≤ right.entropy) :
    (FieldModelAPI.reducePosterior left leftBelief).uncertaintyMass ≤
      (FieldModelAPI.reducePosterior right rightBelief).uncertaintyMass := by
  simpa [reduced_summary_preserves_uncertainty_mass, reduced_summary_preserves_uncertainty_mass] using hEntropy

theorem reducePosterior_preserves_support_mass_compression_boundary :
    CompressionPreserves
      (fun value : PosteriorState × ProbabilisticRouteBelief =>
        FieldModelAPI.reducePosterior value.1 value.2)
      (fun value => value.1.support)
      (fun summary => summary.supportMass) := by
  intro value
  simpa using reduced_summary_preserves_support_mass value.1 value.2

theorem reducePosterior_preserves_uncertainty_mass_compression_boundary :
    CompressionPreserves
      (fun value : PosteriorState × ProbabilisticRouteBelief =>
        FieldModelAPI.reducePosterior value.1 value.2)
      (fun value => value.1.entropy)
      (fun summary => summary.uncertaintyMass) := by
  intro value
  simpa using reduced_summary_preserves_uncertainty_mass value.1 value.2

theorem reducePosterior_preserves_public_macrostate_compression_boundary :
    CompressionPreserves
      (fun value : PosteriorState × ProbabilisticRouteBelief =>
        FieldModelAPI.reducePosterior value.1 value.2)
      (fun value =>
        match value.1.knowledge with
        | .explicitPath => CorridorShape.explicitPath
        | .corridor => CorridorShape.corridorEnvelope
        | .unknown => CorridorShape.opaque
        | .unreachable => CorridorShape.opaque)
      (fun summary => summary.publicMacrostate) := by
  intro value
  simpa using reduced_summary_preserves_public_macrostate value.1 value.2

theorem reduced_summary_is_sufficient_for_mean_field_given_evidence
    (evidence : EvidenceInput) :
    CompressionSufficientFor
      (fun value : PosteriorState × ProbabilisticRouteBelief =>
        FieldModelAPI.reducePosterior value.1 value.2)
      (fun summary => FieldModelInstance.compressMeanFieldImpl evidence summary)
      (fun value =>
        FieldModelInstance.compressMeanFieldImpl evidence
          (FieldModelAPI.reducePosterior value.1 value.2)) := by
  intro value
  rfl

theorem reduced_summary_is_sufficient_for_controller_update_given_evidence
    (evidence : EvidenceInput)
    (controller : ControllerState) :
    CompressionSufficientFor
      (fun value : PosteriorState × ProbabilisticRouteBelief =>
        FieldModelAPI.reducePosterior value.1 value.2)
      (fun summary =>
        FieldModelInstance.updateControllerImpl evidence
          (FieldModelInstance.compressMeanFieldImpl evidence summary) controller)
      (fun value =>
        FieldModelInstance.updateControllerImpl evidence
          (FieldModelInstance.compressMeanFieldImpl evidence
            (FieldModelAPI.reducePosterior value.1 value.2)) controller) := by
  intro value
  rfl

theorem reducePosterior_support_is_conservative_compression_boundary :
    CompressionConservative
      (fun value : PosteriorState × ProbabilisticRouteBelief =>
        FieldModelAPI.reducePosterior value.1 value.2)
      (fun summary => summary.supportMass)
      (fun value => value.1.support)
      Nat.le := by
  intro value
  exact reduced_summary_support_conservative value.1 value.2

theorem reducePosterior_uncertainty_is_conservative_compression_boundary :
    CompressionConservative
      (fun value : PosteriorState × ProbabilisticRouteBelief =>
        FieldModelAPI.reducePosterior value.1 value.2)
      (fun summary => summary.uncertaintyMass)
      (fun value => value.1.entropy)
      Nat.le := by
  intro value
  exact reduced_summary_uncertainty_conservative value.1 value.2

theorem current_model_support_reduction_comparison_hook :
    ReductionComparisonHook FieldModelInstance.instLaws.toModel
      (fun summary => summary.supportMass)
      PosteriorState.support := by
  intro posterior belief
  simpa using reduced_summary_support_conservative posterior belief

theorem current_model_uncertainty_reduction_comparison_hook :
    ReductionComparisonHook FieldModelInstance.instLaws.toModel
      (fun summary => summary.uncertaintyMass)
      PosteriorState.entropy := by
  intro posterior belief
  simpa using reduced_summary_uncertainty_conservative posterior belief

theorem uncertainty_burden_is_order_parameter_adjacent :
    FieldModelAPI.uncertaintyBurdenRole = .orderParameterAdjacent := by
  rfl

theorem exogenous_controller_pressure_can_change_mean_field_after_same_reduction :
    ∃ summary leftEvidence rightEvidence,
      leftEvidence.controllerPressure ≠ rightEvidence.controllerPressure ∧
        FieldModelInstance.compressMeanFieldImpl leftEvidence summary ≠
          FieldModelInstance.compressMeanFieldImpl rightEvidence summary := by
  let summary : ReducedBeliefSummary :=
    { supportMass := 400, uncertaintyMass := 200, publicMacrostate := .corridorEnvelope }
  let leftEvidence : EvidenceInput :=
    { refresh := .unchanged
      reachability := .corridorOnly
      supportSignal := 400
      entropySignal := 200
      controllerPressure := 0
      feedback := .none }
  let rightEvidence : EvidenceInput :=
    { refresh := .unchanged
      reachability := .corridorOnly
      supportSignal := 400
      entropySignal := 200
      controllerPressure := 1000
      feedback := .none }
  refine ⟨summary, leftEvidence, rightEvidence, by decide, ?_⟩
  intro hEq
  have hRelay :
      (FieldModelInstance.compressMeanFieldImpl leftEvidence summary).relayAlignment =
        (FieldModelInstance.compressMeanFieldImpl rightEvidence summary).relayAlignment := by
    exact congrArg MeanFieldState.relayAlignment hEq
  simp [FieldModelInstance.compressMeanFieldImpl, FieldModelInstance.fuseOrderParameterImpl,
    FieldModelInstance.extractOrderParameterImpl, summary, leftEvidence, rightEvidence] at hRelay
  exact (by decide : (200 : Nat) ≠ 700) hRelay

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
