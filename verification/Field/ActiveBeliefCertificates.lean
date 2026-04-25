import Field.ActiveBeliefDefinitive

/-
The Problem. The definitive active-belief stack names the finite-horizon,
demand-quality, controller-band, stress, and theorem-profile assumptions used
by the paper. To make the paper result more definitive, replay artifacts need
proof-facing certificates that imply those assumption records rather than only
labeling them in prose.

Solution Structure.
1. Define compact replay certificates for receiver arrival, useful inference,
   score margins, active demand, controller bands, theorem rows, and bounded
   stress.
2. Prove each certificate exposes exactly the assumptions consumed by the
   existing theorem surfaces.
3. Keep the bridge narrow: these theorems validate certificate-to-assumption
   handoff, not arbitrary mobility, simulator correctness, privacy, or
   adversarial robustness.
-/

/-! # Active Belief Diffusion — replay certificate bridges -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-! ## Arrival And Margin Certificates -/

/-- Replay-visible certificate for receiver-rank arrival by a finite horizon. -/
structure ReceiverArrivalReplayCertificate where
  model : TemporalContactProbabilityModel
  requiredRank : Nat
  observedRankFloor : Nat
  arrivalPermilleFloor : Permille
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid receiver-arrival certificates expose all assumptions as row fields. -/
def validReceiverArrivalReplayCertificate
    (certificate : ReceiverArrivalReplayCertificate) : Prop :=
  validTemporalContactProbabilityModel certificate.model ∧
    0 < certificate.requiredRank ∧
    certificate.requiredRank ≤ certificate.observedRankFloor ∧
    certificate.arrivalPermilleFloor ≤
      finiteHorizonArrivalPermilleLowerBound certificate.model

/-- The receiver-arrival bound induced by a replay certificate. -/
def receiverArrivalBoundOfReplayCertificate
    (certificate : ReceiverArrivalReplayCertificate) :
    ReceiverArrivalBound :=
  { model := certificate.model
    requiredRank := certificate.requiredRank
    receiverArrivalPermilleFloor := certificate.arrivalPermilleFloor }

theorem replay_certificate_implies_receiver_arrival_bound
    (certificate : ReceiverArrivalReplayCertificate)
    (hValid : validReceiverArrivalReplayCertificate certificate) :
    receiverArrivalBoundValid
      (receiverArrivalBoundOfReplayCertificate certificate) ∧
      certificate.requiredRank ≤ certificate.observedRankFloor := by
  -- The certificate packages the exact fields required by the existing bound.
  exact
    ⟨ ⟨hValid.left, hValid.right.left, hValid.right.right.right⟩
    , hValid.right.right.left ⟩

/-- Replay-visible certificate for useful task-relevant contribution arrival. -/
structure UsefulInferenceReplayCertificate where
  model : TemporalContactProbabilityModel
  requiredUsefulContributions : Nat
  observedUsefulContributionFloor : Nat
  usefulArrivalPermilleFloor : Permille
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid useful-inference certificates expose contribution and probability floors. -/
def validUsefulInferenceReplayCertificate
    (certificate : UsefulInferenceReplayCertificate) : Prop :=
  validTemporalContactProbabilityModel certificate.model ∧
    0 < certificate.requiredUsefulContributions ∧
    certificate.requiredUsefulContributions ≤
      certificate.observedUsefulContributionFloor ∧
    certificate.usefulArrivalPermilleFloor ≤
      finiteHorizonArrivalPermilleLowerBound certificate.model

/-- The useful-inference bound induced by a replay certificate. -/
def usefulInferenceBoundOfReplayCertificate
    (certificate : UsefulInferenceReplayCertificate) :
    UsefulInferenceArrivalBound :=
  { model := certificate.model
    requiredUsefulContributions := certificate.requiredUsefulContributions
    usefulArrivalPermilleFloor := certificate.usefulArrivalPermilleFloor }

theorem replay_certificate_implies_useful_inference_arrival_bound
    (certificate : UsefulInferenceReplayCertificate)
    (hValid : validUsefulInferenceReplayCertificate certificate) :
    usefulInferenceArrivalBoundValid
      (usefulInferenceBoundOfReplayCertificate certificate) ∧
      certificate.requiredUsefulContributions ≤
        certificate.observedUsefulContributionFloor := by
  -- Useful-inference replay rows become proof records without changing the theorem.
  exact
    ⟨ ⟨hValid.left, hValid.right.left, hValid.right.right.right⟩
    , hValid.right.right.left ⟩

/-- Replay-visible score and guard certificate for anomaly commitment rows. -/
structure ScoreTraceCertificate where
  scoreModel : BoundedScoreVectorUpdateModel
  guard : AnomalyCommitmentGuard
  initialMargin : Nat
  observedEvidenceCount : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid score certificates expose model validity, margin, and evidence guard. -/
def validScoreTraceCertificate
    (certificate : ScoreTraceCertificate) : Prop :=
  validBoundedScoreVectorUpdateModel certificate.scoreModel ∧
    validAnomalyCommitmentGuard certificate.guard ∧
    certificate.guard.marginThreshold ≤
      anomalyMarginLowerBound certificate.scoreModel certificate.initialMargin ∧
    certificate.guard.evidenceGuard ≤ certificate.observedEvidenceCount

theorem score_trace_certificate_implies_margin_guard
    (certificate : ScoreTraceCertificate)
    (hValid : validScoreTraceCertificate certificate) :
    validBoundedScoreVectorUpdateModel certificate.scoreModel ∧
      validAnomalyCommitmentGuard certificate.guard ∧
      certificate.guard.marginThreshold ≤
        anomalyMarginLowerBound certificate.scoreModel certificate.initialMargin ∧
      certificate.guard.evidenceGuard ≤ certificate.observedEvidenceCount := by
  -- The guarded false-commitment theorem consumes exactly these obligations.
  exact hValid

/-! ## Demand And Controller Certificates -/

/-- Matched active/passive replay certificate for demand-policy comparison. -/
structure DemandPolicyReplayCertificate where
  comparison : DemandGuidedComparison
  activeDemand : DemandSummary
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid demand-policy certificates keep demand bounded and non-evidential. -/
def validDemandPolicyReplayCertificate
    (certificate : DemandPolicyReplayCertificate) : Prop :=
  validDemandSummary certificate.activeDemand ∧
    demandQualityNonWorse certificate.comparison ∧
    0 < certificate.comparison.byteBudget

theorem demand_policy_certificate_implies_useful_arrival_improvement
    (certificate : DemandPolicyReplayCertificate)
    (hValid : validDemandPolicyReplayCertificate certificate) :
    certificate.comparison.passiveUsefulArrivals ≤
      certificate.comparison.activeUsefulArrivals ∧
      certificate.comparison.activeUncertainty ≤
        certificate.comparison.passiveUncertainty ∧
      certificate.comparison.passiveUsefulArrivals *
          certificate.comparison.byteBudget ≤
        certificate.comparison.activeUsefulArrivals *
          certificate.comparison.byteBudget := by
  -- Certificate validity supplies bounded demand plus the same-budget comparison.
  exact
    ⟨ demand_guided_useful_arrivals_nonworse
        certificate.comparison hValid.right.left
    , demand_guided_uncertainty_nonworse
        certificate.comparison hValid.right.left
    , demand_guided_quality_per_byte_nonworse
        certificate.comparison hValid.right.left hValid.right.right ⟩

/-- Replay-visible opportunity certificate for near-critical controller rows. -/
structure NearCriticalControllerOpportunityCertificate where
  band : NearCriticalControllerBand
  opportunityFloor : Nat
  capacityCeiling : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid controller certificates expose opportunity, capacity, and band facts. -/
def validNearCriticalControllerOpportunityCertificate
    (certificate : NearCriticalControllerOpportunityCertificate) : Prop :=
  0 < certificate.opportunityFloor ∧
    certificate.opportunityFloor ≤ certificate.capacityCeiling ∧
    validNearCriticalControllerBand certificate.band

theorem near_critical_controller_enters_band_under_opportunity_bounds
    (certificate : NearCriticalControllerOpportunityCertificate)
    (hValid : validNearCriticalControllerOpportunityCertificate certificate) :
    validNearCriticalControllerBand certificate.band ∧
      certificate.band.lowerPermille ≤ certificate.band.achievedPermille ∧
      certificate.band.achievedPermille ≤ certificate.band.upperPermille := by
  -- Opportunity and capacity checks justify reading the achieved band theorem.
  exact
    ⟨ hValid.right.right
    , near_critical_controller_keeps_pressure_in_band
        certificate.band hValid.right.right ⟩

/-! ## Metadata And Stress Certificates -/

/-- Replay row certificate for theorem-profile metadata exported by Rust. -/
structure ActiveBeliefTheoremProfileReplayRow where
  deterministicReplay : Bool
  theoremProfileExported : Bool
  theoremAssumptionMarked : Bool
  rowSatisfiesBound : Bool
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Narrow soundness condition for theorem-profile rows, not simulator correctness. -/
def soundActiveBeliefTheoremProfileReplayRow
    (row : ActiveBeliefTheoremProfileReplayRow) : Prop :=
  row.deterministicReplay = true ∧
    row.theoremProfileExported = true ∧
    row.theoremAssumptionMarked = true ∧
    row.rowSatisfiesBound = true

theorem rust_replay_rows_sound_for_active_belief_theorem_profiles
    (row : ActiveBeliefTheoremProfileReplayRow)
    (hSound : soundActiveBeliefTheoremProfileReplayRow row) :
    row.theoremProfileExported = true ∧
      row.theoremAssumptionMarked = true ∧
      row.rowSatisfiesBound = true := by
  -- The bridge validates theorem-profile metadata only; it is not a simulator proof.
  exact ⟨hSound.right.left, hSound.right.right.left, hSound.right.right.right⟩

/-- Replay-visible bounded-stress certificate for guarded commitment rows. -/
structure BoundedStressReplayCertificate where
  budget : BoundedStressBudget
  falseCommitmentPermilleBound : Permille
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid stress certificates expose bounded counts and a permille guard. -/
def validBoundedStressReplayCertificate
    (certificate : BoundedStressReplayCertificate) : Prop :=
  validBoundedStressBudget certificate.budget ∧
    certificate.falseCommitmentPermilleBound ≤ 1000

theorem bounded_stress_certificate_implies_guarded_commitment_bound
    (certificate : BoundedStressReplayCertificate)
    (hValid : validBoundedStressReplayCertificate certificate) :
    validBoundedStressBudget certificate.budget ∧
      certificate.falseCommitmentPermilleBound ≤ 1000 := by
  -- Stress replay rows prove only bounded modeled stress, not arbitrary robustness.
  exact hValid

end FieldActiveBelief
