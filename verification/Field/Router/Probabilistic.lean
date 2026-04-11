import Field.Information.Bayesian
import Field.Quality.API

/-!
Posterior-confidence routing layer.

Ownership note:
- this module owns posterior-based router decision semantics
- posterior confidence is not canonical route truth
- posterior confidence is not exported-view quality truth
- exported route views and support ranking do not determine posterior truth
  unless an explicit theorem says so
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterProbabilistic

open FieldInformationBayesian
open FieldInformationProbabilistic
open FieldModelAPI
open FieldNetworkAPI
open FieldQualityAPI
open FieldRouterLifecycle

structure PosteriorConfidenceThresholds where
  corridorMin : Rat
  explicitPathMin : Rat

def PosteriorConfidenceThresholdsAdmissible
    (thresholds : PosteriorConfidenceThresholds) : Prop :=
  0 ≤ thresholds.corridorMin ∧
    thresholds.corridorMin ≤ thresholds.explicitPathMin

inductive PosteriorRoutingDecision
  | abstain
  | corridorEnvelope
  | explicitPath
  deriving Inhabited, Repr, DecidableEq, BEq

def posteriorRoutingDecisionRank : PosteriorRoutingDecision → Nat
  | .abstain => 0
  | .corridorEnvelope => 1
  | .explicitPath => 2

noncomputable def posteriorConfidenceDecision
    (thresholds : PosteriorConfidenceThresholds)
    (_hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (belief : ProbabilisticRouteBelief) : PosteriorRoutingDecision :=
  if (thresholds.explicitPathMin : ℝ) ≤ probabilisticExplicitPathMass belief then
    .explicitPath
  else if (thresholds.corridorMin : ℝ) ≤ probabilisticCorridorCapableMass belief then
    .corridorEnvelope
  else
    .abstain

def PosteriorConfidenceDominates
    (stronger weaker : ProbabilisticRouteBelief) : Prop :=
  probabilisticExplicitPathMass weaker ≤ probabilisticExplicitPathMass stronger ∧
    probabilisticCorridorCapableMass weaker ≤
      probabilisticCorridorCapableMass stronger

def qualityBandUtility : RouteQualityBand → ℝ
  | .low => 0
  | .medium => 1
  | .high => 2

def qualityBandCost : RouteQualityBand → ℝ
  | .low => 2
  | .medium => 1
  | .high => 0

noncomputable def posteriorExpectedReachability
    (belief : ProbabilisticRouteBelief) : ℝ :=
  hypothesisExistenceMass belief .present

noncomputable def posteriorExpectedPathQuality
    (belief : ProbabilisticRouteBelief) : ℝ :=
  qualityBandUtility .low * hypothesisQualityMass belief .low +
    qualityBandUtility .medium * hypothesisQualityMass belief .medium +
    qualityBandUtility .high * hypothesisQualityMass belief .high

noncomputable def posteriorExpectedPathCost
    (belief : ProbabilisticRouteBelief) : ℝ :=
  qualityBandCost .low * hypothesisQualityMass belief .low +
    qualityBandCost .medium * hypothesisQualityMass belief .medium +
    qualityBandCost .high * hypothesisQualityMass belief .high

noncomputable def posteriorRiskPenalty
    (belief : ProbabilisticRouteBelief) : ℝ :=
  hypothesisTransportReliabilityMass belief .lossy +
    hypothesisObservationReliabilityMass belief .noisy

noncomputable def posteriorRiskSensitiveUtility
    (belief : ProbabilisticRouteBelief) : ℝ :=
  posteriorExpectedReachability belief +
    posteriorExpectedPathQuality belief -
    posteriorRiskPenalty belief

noncomputable def posteriorDecisionExpectedUtility
    (belief : ProbabilisticRouteBelief) :
    PosteriorRoutingDecision → ℝ
  | .abstain => 0
  | .corridorEnvelope =>
      posteriorExpectedReachability belief
  | .explicitPath =>
      posteriorExpectedReachability belief +
        posteriorExpectedPathQuality belief -
        posteriorExpectedPathCost belief

noncomputable def posteriorBestDecisionExpectedUtility
    (belief : ProbabilisticRouteBelief) : ℝ :=
  max (posteriorDecisionExpectedUtility belief .abstain)
    (max (posteriorDecisionExpectedUtility belief .corridorEnvelope)
      (posteriorDecisionExpectedUtility belief .explicitPath))

noncomputable def posteriorDecisionRegret
    (belief : ProbabilisticRouteBelief)
    (decision : PosteriorRoutingDecision) : ℝ :=
  posteriorBestDecisionExpectedUtility belief -
    posteriorDecisionExpectedUtility belief decision

noncomputable def posteriorMinRegretDecision
    (belief : ProbabilisticRouteBelief) : PosteriorRoutingDecision :=
  let abstainUtility := posteriorDecisionExpectedUtility belief .abstain
  let corridorUtility := posteriorDecisionExpectedUtility belief .corridorEnvelope
  let explicitUtility := posteriorDecisionExpectedUtility belief .explicitPath
  if explicitUtility ≥ corridorUtility ∧ explicitUtility ≥ abstainUtility then
    .explicitPath
  else if corridorUtility ≥ abstainUtility then
    .corridorEnvelope
  else
    .abstain

theorem posteriorExpectedReachability_nonneg
    (belief : ProbabilisticRouteBelief) :
    0 ≤ posteriorExpectedReachability belief := by
  exact hypothesisExistenceMass_nonneg belief .present

theorem posteriorExpectedPathQuality_nonneg
    (belief : ProbabilisticRouteBelief) :
    0 ≤ posteriorExpectedPathQuality belief := by
  unfold posteriorExpectedPathQuality qualityBandUtility
  have hLow := hypothesisQualityMass_nonneg belief .low
  have hMedium := hypothesisQualityMass_nonneg belief .medium
  have hHigh := hypothesisQualityMass_nonneg belief .high
  nlinarith

theorem posteriorExpectedPathCost_nonneg
    (belief : ProbabilisticRouteBelief) :
    0 ≤ posteriorExpectedPathCost belief := by
  unfold posteriorExpectedPathCost qualityBandCost
  have hLow := hypothesisQualityMass_nonneg belief .low
  have hMedium := hypothesisQualityMass_nonneg belief .medium
  have hHigh := hypothesisQualityMass_nonneg belief .high
  nlinarith

theorem posteriorDecisionExpectedUtility_abstain_le_best
    (belief : ProbabilisticRouteBelief) :
    posteriorDecisionExpectedUtility belief .abstain ≤
      posteriorBestDecisionExpectedUtility belief := by
  unfold posteriorBestDecisionExpectedUtility
  exact le_max_left _ _

theorem posteriorDecisionExpectedUtility_corridor_le_best
    (belief : ProbabilisticRouteBelief) :
    posteriorDecisionExpectedUtility belief .corridorEnvelope ≤
      posteriorBestDecisionExpectedUtility belief := by
  unfold posteriorBestDecisionExpectedUtility
  exact le_trans (le_max_left _ _) (le_max_right _ _)

theorem posteriorDecisionExpectedUtility_explicit_le_best
    (belief : ProbabilisticRouteBelief) :
    posteriorDecisionExpectedUtility belief .explicitPath ≤
      posteriorBestDecisionExpectedUtility belief := by
  unfold posteriorBestDecisionExpectedUtility
  exact le_trans (le_max_right _ _) (le_max_right _ _)

theorem posteriorDecisionRegret_nonneg
    (belief : ProbabilisticRouteBelief)
    (decision : PosteriorRoutingDecision) :
    0 ≤ posteriorDecisionRegret belief decision := by
  unfold posteriorDecisionRegret
  cases decision with
  | abstain =>
      linarith [posteriorDecisionExpectedUtility_abstain_le_best belief]
  | corridorEnvelope =>
      linarith [posteriorDecisionExpectedUtility_corridor_le_best belief]
  | explicitPath =>
      linarith [posteriorDecisionExpectedUtility_explicit_le_best belief]

theorem posteriorConfidenceDecision_eq_of_equal_belief
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (left right : ProbabilisticRouteBelief)
    (hEq : left = right) :
    posteriorConfidenceDecision thresholds hAdm left =
      posteriorConfidenceDecision thresholds hAdm right := by
  simp [hEq]

theorem posteriorConfidenceDecision_explicitPath_implies_threshold
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (belief : ProbabilisticRouteBelief)
    (hDecision :
      posteriorConfidenceDecision thresholds hAdm belief =
        PosteriorRoutingDecision.explicitPath) :
    (thresholds.explicitPathMin : ℝ) ≤ probabilisticExplicitPathMass belief := by
  unfold posteriorConfidenceDecision at hDecision
  by_cases hExplicit : (thresholds.explicitPathMin : ℝ) ≤ probabilisticExplicitPathMass belief
  · exact hExplicit
  · by_cases hCorridor : (thresholds.corridorMin : ℝ) ≤ probabilisticCorridorCapableMass belief
    · simp [hExplicit, hCorridor] at hDecision
    · simp [hExplicit, hCorridor] at hDecision

theorem posteriorConfidenceDecision_corridor_implies_threshold
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (belief : ProbabilisticRouteBelief)
    (hDecision :
      posteriorConfidenceDecision thresholds hAdm belief =
        PosteriorRoutingDecision.corridorEnvelope) :
    probabilisticExplicitPathMass belief < (thresholds.explicitPathMin : ℝ) ∧
      (thresholds.corridorMin : ℝ) ≤ probabilisticCorridorCapableMass belief := by
  unfold posteriorConfidenceDecision at hDecision
  by_cases hExplicit : (thresholds.explicitPathMin : ℝ) ≤ probabilisticExplicitPathMass belief
  · simp [hExplicit] at hDecision
  · by_cases hCorridor : (thresholds.corridorMin : ℝ) ≤ probabilisticCorridorCapableMass belief
    · simp [hExplicit, hCorridor] at hDecision
      exact ⟨lt_of_not_ge hExplicit, hCorridor⟩
    · simp [hExplicit, hCorridor] at hDecision

theorem posteriorConfidenceDecision_deterministic_on_equal_inputs
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (belief : ProbabilisticRouteBelief) :
    posteriorConfidenceDecision thresholds hAdm belief =
      posteriorConfidenceDecision thresholds hAdm belief := by
  rfl

theorem posteriorConfidenceDecision_explicit_of_dominating_explicit
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (stronger weaker : ProbabilisticRouteBelief)
    (hDom : PosteriorConfidenceDominates stronger weaker)
    (hDecision :
      posteriorConfidenceDecision thresholds hAdm weaker =
        PosteriorRoutingDecision.explicitPath) :
    posteriorConfidenceDecision thresholds hAdm stronger =
      PosteriorRoutingDecision.explicitPath := by
  have hWeakThreshold :
      (thresholds.explicitPathMin : ℝ) ≤ probabilisticExplicitPathMass weaker :=
    posteriorConfidenceDecision_explicitPath_implies_threshold thresholds hAdm weaker hDecision
  have hStrongThreshold :
      (thresholds.explicitPathMin : ℝ) ≤ probabilisticExplicitPathMass stronger :=
    le_trans hWeakThreshold hDom.1
  unfold posteriorConfidenceDecision
  simp [hStrongThreshold]

theorem posteriorConfidenceDecision_not_abstain_of_dominating_corridor
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (stronger weaker : ProbabilisticRouteBelief)
    (hDom : PosteriorConfidenceDominates stronger weaker)
    (hDecision :
      posteriorConfidenceDecision thresholds hAdm weaker =
        PosteriorRoutingDecision.corridorEnvelope) :
    posteriorConfidenceDecision thresholds hAdm stronger ≠
      PosteriorRoutingDecision.abstain := by
  have hStrongCorridor :
      (thresholds.corridorMin : ℝ) ≤ probabilisticCorridorCapableMass stronger := by
    rcases posteriorConfidenceDecision_corridor_implies_threshold
      thresholds hAdm weaker hDecision with ⟨_hExplicit, hCorridor⟩
    exact le_trans hCorridor hDom.2
  unfold posteriorConfidenceDecision
  by_cases hExplicit : (thresholds.explicitPathMin : ℝ) ≤ probabilisticExplicitPathMass stronger
  · simp [hExplicit]
  · simp [hExplicit, hStrongCorridor]

theorem posteriorConfidenceDecision_rank_monotone_of_dominance
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (stronger weaker : ProbabilisticRouteBelief)
    (hDom : PosteriorConfidenceDominates stronger weaker) :
    posteriorRoutingDecisionRank (posteriorConfidenceDecision thresholds hAdm weaker) ≤
      posteriorRoutingDecisionRank (posteriorConfidenceDecision thresholds hAdm stronger) := by
  cases hWeak : posteriorConfidenceDecision thresholds hAdm weaker with
  | abstain =>
      simp [posteriorRoutingDecisionRank]
  | corridorEnvelope =>
      have hNotAbstain :
          posteriorConfidenceDecision thresholds hAdm stronger ≠ .abstain :=
        posteriorConfidenceDecision_not_abstain_of_dominating_corridor
          thresholds hAdm stronger weaker hDom hWeak
      cases hStrong : posteriorConfidenceDecision thresholds hAdm stronger <;>
        simp [posteriorRoutingDecisionRank, hStrong] at hNotAbstain ⊢
  | explicitPath =>
      have hStrong :
          posteriorConfidenceDecision thresholds hAdm stronger = .explicitPath :=
        posteriorConfidenceDecision_explicit_of_dominating_explicit
          thresholds hAdm stronger weaker hDom hWeak
      simp [posteriorRoutingDecisionRank, hStrong]

def conservativePosteriorThresholds : PosteriorConfidenceThresholds :=
  { corridorMin := 1 / 2
    explicitPathMin := 3 / 4 }

theorem conservativePosteriorThresholds_admissible :
    PosteriorConfidenceThresholdsAdmissible conservativePosteriorThresholds := by
  norm_num [conservativePosteriorThresholds, PosteriorConfidenceThresholdsAdmissible]

noncomputable def explicitPosteriorBelief : ProbabilisticRouteBelief :=
  pointHypothesisBelief
    (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath)

noncomputable def corridorPosteriorBelief : ProbabilisticRouteBelief :=
  pointHypothesisBelief
    (defaultHypothesisOfKnowledge ReachabilityKnowledge.corridor)

theorem explicitPosteriorBelief_expected_reachability :
    posteriorExpectedReachability explicitPosteriorBelief = 1 := by
  simp [posteriorExpectedReachability, explicitPosteriorBelief,
    defaultHypothesisOfKnowledge, hypothesisExistenceMass_pointHypothesisBelief]

theorem explicitPosteriorBelief_expected_path_quality :
    posteriorExpectedPathQuality explicitPosteriorBelief = 2 := by
  simp [posteriorExpectedPathQuality, explicitPosteriorBelief,
    qualityBandUtility, defaultHypothesisOfKnowledge,
    hypothesisQualityMass_pointHypothesisBelief]

theorem explicitPosteriorBelief_expected_path_cost :
    posteriorExpectedPathCost explicitPosteriorBelief = 0 := by
  simp [posteriorExpectedPathCost, explicitPosteriorBelief,
    qualityBandCost, defaultHypothesisOfKnowledge,
    hypothesisQualityMass_pointHypothesisBelief]

theorem explicitPosteriorBelief_risk_sensitive_utility :
    posteriorRiskSensitiveUtility explicitPosteriorBelief = 3 := by
  simp [posteriorRiskSensitiveUtility, posteriorRiskPenalty,
    posteriorExpectedReachability, posteriorExpectedPathQuality,
    explicitPosteriorBelief, qualityBandUtility, defaultHypothesisOfKnowledge,
    hypothesisExistenceMass_pointHypothesisBelief,
    hypothesisQualityMass_pointHypothesisBelief,
    hypothesisTransportReliabilityMass_pointHypothesisBelief,
    hypothesisObservationReliabilityMass_pointHypothesisBelief]
  norm_num

def higherSupportInstalledRoute : LifecycleRoute :=
  { candidate :=
      { publisher := .alpha
        destination := .corridorA
        shape := .corridorEnvelope
        support := 8
        hopLower := 1
        hopUpper := 2 }
    status := .installed }

def lowerSupportInstalledRoute : LifecycleRoute :=
  { candidate :=
      { publisher := .beta
        destination := .corridorA
        shape := .corridorEnvelope
        support := 3
        hopLower := 1
        hopUpper := 2 }
    status := .installed }

theorem explicitPosteriorBelief_public_projection :
    probabilisticPublicProjection explicitPosteriorBelief =
      CorridorShape.explicitPath := by
  simpa [explicitPosteriorBelief] using
    probabilisticPublicProjection_pointHypothesisBelief
      (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath)

theorem explicitPosteriorBelief_decides_explicit :
    posteriorConfidenceDecision conservativePosteriorThresholds
        conservativePosteriorThresholds_admissible explicitPosteriorBelief =
      .explicitPath := by
  have hExplicitMass :
      (conservativePosteriorThresholds.explicitPathMin : ℝ) ≤
        probabilisticExplicitPathMass explicitPosteriorBelief := by
    have hMass :
        probabilisticExplicitPathMass explicitPosteriorBelief = 1 := by
      unfold probabilisticExplicitPathMass explicitPosteriorBelief
      simpa [defaultHypothesisOfKnowledge] using
        hypothesisKnowledgeMass_pointHypothesisBelief
          (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath)
          FieldHypothesis.explicitPath
    rw [hMass]
    norm_num [conservativePosteriorThresholds]
  unfold posteriorConfidenceDecision
  simp [hExplicitMass]

theorem corridorPosteriorBelief_decides_corridor :
    posteriorConfidenceDecision conservativePosteriorThresholds
        conservativePosteriorThresholds_admissible corridorPosteriorBelief =
      .corridorEnvelope := by
  have hExplicitMass :
      ¬ (conservativePosteriorThresholds.explicitPathMin : ℝ) ≤
          probabilisticExplicitPathMass corridorPosteriorBelief := by
    have hMass :
        probabilisticExplicitPathMass corridorPosteriorBelief = 0 := by
      unfold probabilisticExplicitPathMass corridorPosteriorBelief
      simpa [defaultHypothesisOfKnowledge] using
        hypothesisKnowledgeMass_pointHypothesisBelief
          (defaultHypothesisOfKnowledge ReachabilityKnowledge.corridor)
          FieldHypothesis.explicitPath
    rw [hMass]
    norm_num [conservativePosteriorThresholds]
  have hCorridorMass :
      (conservativePosteriorThresholds.corridorMin : ℝ) ≤
        probabilisticCorridorCapableMass corridorPosteriorBelief := by
    have hCorridor :
        hypothesisKnowledgeMass corridorPosteriorBelief FieldHypothesis.corridor = 1 := by
      simpa [corridorPosteriorBelief, defaultHypothesisOfKnowledge] using
        hypothesisKnowledgeMass_pointHypothesisBelief
          (defaultHypothesisOfKnowledge ReachabilityKnowledge.corridor)
          FieldHypothesis.corridor
    have hExplicit :
        hypothesisKnowledgeMass corridorPosteriorBelief FieldHypothesis.explicitPath = 0 := by
      simpa [corridorPosteriorBelief, defaultHypothesisOfKnowledge] using
        hypothesisKnowledgeMass_pointHypothesisBelief
          (defaultHypothesisOfKnowledge ReachabilityKnowledge.corridor)
          FieldHypothesis.explicitPath
    unfold probabilisticCorridorCapableMass
    rw [hCorridor, hExplicit]
    norm_num [conservativePosteriorThresholds]
  unfold posteriorConfidenceDecision
  simp [hExplicitMass, hCorridorMass]

theorem exported_route_view_does_not_determine_posterior_confidence_truth :
    routeComparisonView higherSupportInstalledRoute =
      routeComparisonView higherSupportInstalledRoute ∧
    posteriorConfidenceDecision conservativePosteriorThresholds
        conservativePosteriorThresholds_admissible corridorPosteriorBelief =
      .corridorEnvelope ∧
    posteriorConfidenceDecision conservativePosteriorThresholds
        conservativePosteriorThresholds_admissible explicitPosteriorBelief =
      .explicitPath := by
  exact ⟨rfl, corridorPosteriorBelief_decides_corridor, explicitPosteriorBelief_decides_explicit⟩

theorem support_dominance_does_not_determine_posterior_confidence_truth :
    comparisonWinner .supportDominance
        (routeComparisonView higherSupportInstalledRoute)
        (routeComparisonView lowerSupportInstalledRoute) = .left ∧
      posteriorConfidenceDecision conservativePosteriorThresholds
          conservativePosteriorThresholds_admissible corridorPosteriorBelief =
        .corridorEnvelope ∧
      posteriorConfidenceDecision conservativePosteriorThresholds
          conservativePosteriorThresholds_admissible explicitPosteriorBelief =
        .explicitPath := by
  refine ⟨?_, corridorPosteriorBelief_decides_corridor, explicitPosteriorBelief_decides_explicit⟩
  simp [comparisonWinner, routeComparisonView, RouteComparisonInputAdmissible,
    RouteViewAdmissible, routeViewIsActive, higherSupportInstalledRoute,
    lowerSupportInstalledRoute]

end FieldRouterProbabilistic
