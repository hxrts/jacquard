import Field.Adequacy.Instance
import Field.Information.Calibration

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyProbabilistic

open FieldAdequacyAPI
open FieldInformationCalibration
open FieldModelAPI
open FieldModelInstance
open FieldRouterProbabilistic

def runtimeLeadingEvidence
    (artifacts : List RuntimeRoundArtifact) : EvidenceInput :=
  match FieldAdequacyAPI.runtimeEvidence artifacts with
  | evidence :: _ => evidence
  | [] => unknownEvidence

def traceLeadingEvidence
    (trace : FieldProtocolAPI.ProtocolTrace) : EvidenceInput :=
  match FieldBoundary.controllerEvidenceFromTrace trace with
  | evidence :: _ => evidence
  | [] => unknownEvidence

noncomputable def runtimePosteriorOfArtifacts
    (state : LocalState)
    (artifacts : List RuntimeRoundArtifact) : FieldModelAPI.ProbabilisticRouteBelief :=
  FieldModelAPI.bayesianPosterior (runtimeLeadingEvidence artifacts) state

noncomputable def runtimePosteriorDecision
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (state : LocalState)
    (artifacts : List RuntimeRoundArtifact) : PosteriorRoutingDecision :=
  posteriorConfidenceDecision thresholds hAdm (runtimePosteriorOfArtifacts state artifacts)

noncomputable def runtimePosteriorMinRegretDecision
    (state : LocalState)
    (artifacts : List RuntimeRoundArtifact) : PosteriorRoutingDecision :=
  posteriorMinRegretDecision (runtimePosteriorOfArtifacts state artifacts)

noncomputable def runtimePosteriorDecisionExpectedUtility
    (state : LocalState)
    (decision : PosteriorRoutingDecision)
    (artifacts : List RuntimeRoundArtifact) : ℝ :=
  posteriorDecisionExpectedUtility (runtimePosteriorOfArtifacts state artifacts) decision

noncomputable def tracePosteriorDecisionExpectedUtility
    (state : LocalState)
    (decision : PosteriorRoutingDecision)
    (artifacts : List RuntimeRoundArtifact) : ℝ :=
  posteriorDecisionExpectedUtility
    (FieldModelAPI.bayesianPosterior
      (traceLeadingEvidence (FieldAdequacyAPI.extractTrace artifacts))
      state)
    decision

theorem runtimeLeadingEvidence_eq_traceLeadingEvidence
    (artifacts : List RuntimeRoundArtifact) :
    runtimeLeadingEvidence artifacts =
      traceLeadingEvidence (FieldAdequacyAPI.extractTrace artifacts) := by
  unfold runtimeLeadingEvidence traceLeadingEvidence
  rw [FieldAdequacyAPI.runtime_evidence_agrees_with_semantic_trace]

theorem runtime_probabilistic_projection_complete_for_posterior_decision
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (state : LocalState)
    (left right : List RuntimeRoundArtifact)
    (hEvidence : runtimeLeadingEvidence left = runtimeLeadingEvidence right) :
    runtimePosteriorDecision thresholds hAdm state left =
      runtimePosteriorDecision thresholds hAdm state right := by
  unfold runtimePosteriorDecision runtimePosteriorOfArtifacts
  simp [hEvidence]

theorem runtime_trace_confidence_threshold_preservation
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (state : LocalState)
    (artifacts : List RuntimeRoundArtifact) :
    runtimePosteriorDecision thresholds hAdm state artifacts =
      posteriorConfidenceDecision thresholds hAdm
        (FieldModelAPI.bayesianPosterior
          (traceLeadingEvidence (FieldAdequacyAPI.extractTrace artifacts))
          state) := by
  unfold runtimePosteriorDecision runtimePosteriorOfArtifacts
  rw [runtimeLeadingEvidence_eq_traceLeadingEvidence]

theorem runtime_trace_posterior_min_regret_decision_preservation
    (state : LocalState)
    (artifacts : List RuntimeRoundArtifact) :
    runtimePosteriorMinRegretDecision state artifacts =
      posteriorMinRegretDecision
        (FieldModelAPI.bayesianPosterior
          (traceLeadingEvidence (FieldAdequacyAPI.extractTrace artifacts))
          state) := by
  unfold runtimePosteriorMinRegretDecision runtimePosteriorOfArtifacts
  rw [runtimeLeadingEvidence_eq_traceLeadingEvidence]

theorem runtime_trace_expected_utility_preservation
    (state : LocalState)
    (decision : PosteriorRoutingDecision)
    (artifacts : List RuntimeRoundArtifact) :
    runtimePosteriorDecisionExpectedUtility state decision artifacts =
      tracePosteriorDecisionExpectedUtility state decision artifacts := by
  unfold runtimePosteriorDecisionExpectedUtility
    tracePosteriorDecisionExpectedUtility runtimePosteriorOfArtifacts
  rw [runtimeLeadingEvidence_eq_traceLeadingEvidence]

theorem runtime_trace_expected_utility_order_preservation
    (state : LocalState)
    (decision : PosteriorRoutingDecision)
    (left right : List RuntimeRoundArtifact)
    (hOrder :
      runtimePosteriorDecisionExpectedUtility state decision left ≤
        runtimePosteriorDecisionExpectedUtility state decision right) :
    tracePosteriorDecisionExpectedUtility state decision left ≤
      tracePosteriorDecisionExpectedUtility state decision right := by
  rw [← runtime_trace_expected_utility_preservation state decision left]
  rw [← runtime_trace_expected_utility_preservation state decision right]
  exact hOrder

theorem runtime_posterior_decision_ignores_erased_tail_artifacts
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (state : LocalState)
    (head : RuntimeRoundArtifact)
    (tail₁ tail₂ : List RuntimeRoundArtifact)
    (hTail :
      runtimeLeadingEvidence (head :: tail₁) =
        runtimeLeadingEvidence (head :: tail₂)) :
    runtimePosteriorDecision thresholds hAdm state (head :: tail₁) =
      runtimePosteriorDecision thresholds hAdm state (head :: tail₂) := by
  exact runtime_probabilistic_projection_complete_for_posterior_decision
    thresholds hAdm state (head :: tail₁) (head :: tail₂) hTail

end FieldAdequacyProbabilistic
