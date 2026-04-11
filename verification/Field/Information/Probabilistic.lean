import Field.Model.API
import Mathlib.Data.Fintype.BigOperators
import Mathlib.Data.Real.Basic
import Mathlib.Tactic

/-! # Information.Probabilistic — hypothesis mass aggregation and public projection derivation -/

/-
Aggregate belief mass across knowledge, existence, quality, and reliability hypothesis dimensions
and derive the public corridor projection from local beliefs.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldInformationProbabilistic

open FieldModelAPI
open EntropyAPI
open scoped BigOperators

/-! ## Hypothesis Aggregation -/

def hypothesisKnowledgeMass
    (belief : ProbabilisticRouteBelief)
    (knowledge : FieldHypothesis) : ℝ :=
  ∑ h, if h.knowledge = knowledge then belief.pmf h else 0

def hypothesisExistenceMass
    (belief : ProbabilisticRouteBelief)
    (existence : ProbabilisticRouteExistence) : ℝ :=
  ∑ h, if h.existence = existence then belief.pmf h else 0

def hypothesisQualityMass
    (belief : ProbabilisticRouteBelief)
    (quality : RouteQualityBand) : ℝ :=
  ∑ h, if h.quality = quality then belief.pmf h else 0

def hypothesisTransportReliabilityMass
    (belief : ProbabilisticRouteBelief)
    (reliability : TransportReliabilityBand) : ℝ :=
  ∑ h, if h.transportReliability = reliability then belief.pmf h else 0

def hypothesisObservationReliabilityMass
    (belief : ProbabilisticRouteBelief)
    (reliability : ObservationReliabilityBand) : ℝ :=
  ∑ h, if h.observationReliability = reliability then belief.pmf h else 0

theorem hypothesisKnowledgeMass_nonneg
    (belief : ProbabilisticRouteBelief)
    (knowledge : FieldHypothesis) :
    0 ≤ hypothesisKnowledgeMass belief knowledge := by
  unfold hypothesisKnowledgeMass
  refine Finset.sum_nonneg ?_
  intro h _
  by_cases hEq : h.knowledge = knowledge
  · simp [hEq, ProbabilisticRouteBelief.nonneg]
  · simp [hEq]

theorem hypothesisExistenceMass_nonneg
    (belief : ProbabilisticRouteBelief)
    (existence : ProbabilisticRouteExistence) :
    0 ≤ hypothesisExistenceMass belief existence := by
  unfold hypothesisExistenceMass
  refine Finset.sum_nonneg ?_
  intro h _
  by_cases hEq : h.existence = existence
  · simp [hEq, ProbabilisticRouteBelief.nonneg]
  · simp [hEq]

theorem hypothesisQualityMass_nonneg
    (belief : ProbabilisticRouteBelief)
    (quality : RouteQualityBand) :
    0 ≤ hypothesisQualityMass belief quality := by
  unfold hypothesisQualityMass
  refine Finset.sum_nonneg ?_
  intro h _
  by_cases hEq : h.quality = quality
  · simp [hEq, ProbabilisticRouteBelief.nonneg]
  · simp [hEq]

theorem hypothesisTransportReliabilityMass_nonneg
    (belief : ProbabilisticRouteBelief)
    (reliability : TransportReliabilityBand) :
    0 ≤ hypothesisTransportReliabilityMass belief reliability := by
  unfold hypothesisTransportReliabilityMass
  refine Finset.sum_nonneg ?_
  intro h _
  by_cases hEq : h.transportReliability = reliability
  · simp [hEq, ProbabilisticRouteBelief.nonneg]
  · simp [hEq]

theorem hypothesisObservationReliabilityMass_nonneg
    (belief : ProbabilisticRouteBelief)
    (reliability : ObservationReliabilityBand) :
    0 ≤ hypothesisObservationReliabilityMass belief reliability := by
  unfold hypothesisObservationReliabilityMass
  refine Finset.sum_nonneg ?_
  intro h _
  by_cases hEq : h.observationReliability = reliability
  · simp [hEq, ProbabilisticRouteBelief.nonneg]
  · simp [hEq]

def probabilisticExplicitPathMass
    (belief : ProbabilisticRouteBelief) : ℝ :=
  hypothesisKnowledgeMass belief FieldHypothesis.explicitPath

def probabilisticCorridorCapableMass
    (belief : ProbabilisticRouteBelief) : ℝ :=
  hypothesisKnowledgeMass belief FieldHypothesis.corridor +
    hypothesisKnowledgeMass belief FieldHypothesis.explicitPath

/-! ## Public Projection -/

noncomputable def probabilisticPublicProjection
    (belief : ProbabilisticRouteBelief) : CorridorShape :=
  if probabilisticExplicitPathMass belief > 0 then
    CorridorShape.explicitPath
  else if probabilisticCorridorCapableMass belief > 0 then
    CorridorShape.corridorEnvelope
  else
    CorridorShape.opaque

noncomputable def pointHypothesisBelief
    (hypothesis : ProbabilisticRouteHypothesis) : ProbabilisticRouteBelief :=
  { distribution :=
      { pmf := fun next => if next = hypothesis then 1 else 0
        nonneg := by
          intro next
          by_cases hEq : next = hypothesis <;> simp [hEq]
        sum_one := by
          classical
          simp } }

theorem pointHypothesisBelief_pmf
    (hypothesis next : ProbabilisticRouteHypothesis) :
    (pointHypothesisBelief hypothesis).pmf next =
      (if next = hypothesis then 1 else 0) := by
  rfl

theorem hypothesisKnowledgeMass_pointHypothesisBelief
    (hypothesis : ProbabilisticRouteHypothesis)
    (knowledge : FieldHypothesis) :
    hypothesisKnowledgeMass (pointHypothesisBelief hypothesis) knowledge =
      (if hypothesis.knowledge = knowledge then 1 else 0) := by
  classical
  unfold hypothesisKnowledgeMass
  by_cases hKnowledge : hypothesis.knowledge = knowledge
  · rw [if_pos hKnowledge]
    rw [Fintype.sum_eq_single hypothesis]
    · simp [hKnowledge, pointHypothesisBelief_pmf]
    · intro x hx
      by_cases hxKnowledge : x.knowledge = knowledge
      · simp [hxKnowledge, pointHypothesisBelief_pmf, hx]
      · simp [hxKnowledge]
  · rw [if_neg hKnowledge]
    refine Finset.sum_eq_zero ?_
    intro x _
    by_cases hxKnowledge : x.knowledge = knowledge
    · have hNe : x ≠ hypothesis := by
        intro hEq
        apply hKnowledge
        simpa [hEq] using hxKnowledge
      simp [hxKnowledge, pointHypothesisBelief_pmf, hNe]
    · simp [hxKnowledge]

theorem hypothesisExistenceMass_pointHypothesisBelief
    (hypothesis : ProbabilisticRouteHypothesis)
    (existence : ProbabilisticRouteExistence) :
    hypothesisExistenceMass (pointHypothesisBelief hypothesis) existence =
      (if hypothesis.existence = existence then 1 else 0) := by
  classical
  unfold hypothesisExistenceMass
  by_cases hExistence : hypothesis.existence = existence
  · rw [if_pos hExistence]
    rw [Fintype.sum_eq_single hypothesis]
    · simp [hExistence, pointHypothesisBelief_pmf]
    · intro x hx
      by_cases hxExistence : x.existence = existence
      · simp [hxExistence, pointHypothesisBelief_pmf, hx]
      · simp [hxExistence]
  · rw [if_neg hExistence]
    refine Finset.sum_eq_zero ?_
    intro x _
    by_cases hxExistence : x.existence = existence
    · have hNe : x ≠ hypothesis := by
        intro hEq
        apply hExistence
        simpa [hEq] using hxExistence
      simp [hxExistence, pointHypothesisBelief_pmf, hNe]
    · simp [hxExistence]

theorem hypothesisQualityMass_pointHypothesisBelief
    (hypothesis : ProbabilisticRouteHypothesis)
    (quality : RouteQualityBand) :
    hypothesisQualityMass (pointHypothesisBelief hypothesis) quality =
      (if hypothesis.quality = quality then 1 else 0) := by
  classical
  unfold hypothesisQualityMass
  by_cases hQuality : hypothesis.quality = quality
  · rw [if_pos hQuality]
    rw [Fintype.sum_eq_single hypothesis]
    · simp [hQuality, pointHypothesisBelief_pmf]
    · intro x hx
      by_cases hxQuality : x.quality = quality
      · simp [hxQuality, pointHypothesisBelief_pmf, hx]
      · simp [hxQuality]
  · rw [if_neg hQuality]
    refine Finset.sum_eq_zero ?_
    intro x _
    by_cases hxQuality : x.quality = quality
    · have hNe : x ≠ hypothesis := by
        intro hEq
        apply hQuality
        simpa [hEq] using hxQuality
      simp [hxQuality, pointHypothesisBelief_pmf, hNe]
    · simp [hxQuality]

theorem hypothesisTransportReliabilityMass_pointHypothesisBelief
    (hypothesis : ProbabilisticRouteHypothesis)
    (reliability : TransportReliabilityBand) :
    hypothesisTransportReliabilityMass
        (pointHypothesisBelief hypothesis) reliability =
      (if hypothesis.transportReliability = reliability then 1 else 0) := by
  classical
  unfold hypothesisTransportReliabilityMass
  by_cases hReliability : hypothesis.transportReliability = reliability
  · rw [if_pos hReliability]
    rw [Fintype.sum_eq_single hypothesis]
    · simp [hReliability, pointHypothesisBelief_pmf]
    · intro x hx
      by_cases hxReliability : x.transportReliability = reliability
      · simp [hxReliability, pointHypothesisBelief_pmf, hx]
      · simp [hxReliability]
  · rw [if_neg hReliability]
    refine Finset.sum_eq_zero ?_
    intro x _
    by_cases hxReliability : x.transportReliability = reliability
    · have hNe : x ≠ hypothesis := by
        intro hEq
        apply hReliability
        simpa [hEq] using hxReliability
      simp [hxReliability, pointHypothesisBelief_pmf, hNe]
    · simp [hxReliability]

theorem hypothesisObservationReliabilityMass_pointHypothesisBelief
    (hypothesis : ProbabilisticRouteHypothesis)
    (reliability : ObservationReliabilityBand) :
    hypothesisObservationReliabilityMass
        (pointHypothesisBelief hypothesis) reliability =
      (if hypothesis.observationReliability = reliability then 1 else 0) := by
  classical
  unfold hypothesisObservationReliabilityMass
  by_cases hReliability : hypothesis.observationReliability = reliability
  · rw [if_pos hReliability]
    rw [Fintype.sum_eq_single hypothesis]
    · simp [hReliability, pointHypothesisBelief_pmf]
    · intro x hx
      by_cases hxReliability : x.observationReliability = reliability
      · simp [hxReliability, pointHypothesisBelief_pmf, hx]
      · simp [hxReliability]
  · rw [if_neg hReliability]
    refine Finset.sum_eq_zero ?_
    intro x _
    by_cases hxReliability : x.observationReliability = reliability
    · have hNe : x ≠ hypothesis := by
        intro hEq
        apply hReliability
        simpa [hEq] using hxReliability
      simp [hxReliability, pointHypothesisBelief_pmf, hNe]
    · simp [hxReliability]

theorem probabilisticPublicProjection_pointHypothesisBelief
    (hypothesis : ProbabilisticRouteHypothesis) :
    probabilisticPublicProjection (pointHypothesisBelief hypothesis) =
      match hypothesis.knowledge with
      | .explicitPath => CorridorShape.explicitPath
      | .corridor => CorridorShape.corridorEnvelope
      | .unknown => CorridorShape.opaque
      | .unreachable => CorridorShape.opaque := by
  cases hKnowledge : hypothesis.knowledge <;>
    simp [probabilisticPublicProjection, probabilisticExplicitPathMass,
      probabilisticCorridorCapableMass, hypothesisKnowledgeMass_pointHypothesisBelief,
      hKnowledge]

theorem probabilistic_public_projection_point_mass_ignores_quality_and_reliability
    (left right : ProbabilisticRouteHypothesis)
    (hKnowledge : left.knowledge = right.knowledge) :
    probabilisticPublicProjection (pointHypothesisBelief left) =
      probabilisticPublicProjection (pointHypothesisBelief right) := by
  rw [probabilisticPublicProjection_pointHypothesisBelief left]
  rw [probabilisticPublicProjection_pointHypothesisBelief right]
  simp [hKnowledge]

theorem probabilistic_public_projection_forgets_latent_quality_split
    (existence : ProbabilisticRouteExistence)
    (transport : TransportReliabilityBand)
    (obsReliability : ObservationReliabilityBand)
    (knowledge : FieldHypothesis) :
    probabilisticPublicProjection
        (pointHypothesisBelief
          { existence := existence
            quality := .low
            transportReliability := transport
            observationReliability := obsReliability
            knowledge := knowledge }) =
      probabilisticPublicProjection
        (pointHypothesisBelief
          { existence := existence
            quality := .high
            transportReliability := transport
            observationReliability := obsReliability
            knowledge := knowledge }) := by
  apply probabilistic_public_projection_point_mass_ignores_quality_and_reliability
  rfl

theorem probabilistic_public_projection_forgets_transport_reliability_split
    (existence : ProbabilisticRouteExistence)
    (quality : RouteQualityBand)
    (obsReliability : ObservationReliabilityBand)
    (knowledge : FieldHypothesis) :
    probabilisticPublicProjection
        (pointHypothesisBelief
          { existence := existence
            quality := quality
            transportReliability := .lossy
            observationReliability := obsReliability
            knowledge := knowledge }) =
      probabilisticPublicProjection
        (pointHypothesisBelief
          { existence := existence
            quality := quality
            transportReliability := .reliable
            observationReliability := obsReliability
            knowledge := knowledge }) := by
  apply probabilistic_public_projection_point_mass_ignores_quality_and_reliability
  rfl

end FieldInformationProbabilistic
