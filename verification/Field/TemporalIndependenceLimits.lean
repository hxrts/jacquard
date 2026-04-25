import Field.ActiveBeliefDecisionSufficiency

/-
The Problem. The deeper distributed error-correction claim is not just that a
decision may be possible before full reconstruction. It is that reconstruction
in temporal decentralized networks is limited by the effective independence
created by contact, mobility, forwarding policy, and resource budgets. Raw
copies, raw transmissions, and an apparent reproduction number do not by
themselves certify recoverable rank.

Solution Structure.
1. Define proof-facing contact-diversity and effective-rank certificates.
2. Prove reconstruction requires effective rank and recovery probability is
   bounded by the certified probability of reaching effective rank.
3. Add concrete counterexamples for many copies and high raw reproduction.
4. Compare matched networks whose raw spread is equal but effective rank differs.
5. State the cost-time-independence triangle and effective-rank control target.
6. Package the final distributed error-correction independence-limit theorem.
-/

/-! # Temporal Independence Limits For Distributed Error Correction -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-! ## Effective Rank And Contact Diversity -/

/-- Replay-facing summary of how much independent rank a contact process produced. -/
structure TemporalContactDiversitySummary where
  nodeCount : Nat
  contactRatePermille : Permille
  rawTransmissions : Nat
  rawCopies : Nat
  effectiveRank : Nat
  contactEntropyPermille : Permille
  distinctCarrierLineages : Nat
  distinctBridgeCrossings : Nat
  timeHorizon : Nat
  byteBudget : Nat
  storageBudget : Nat
  observabilityCost : Nat
  rawReproductionPermille : Permille
  effectiveReproductionPermille : Permille
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid summaries keep effective rank bounded by raw spread and artifact caps. -/
def validTemporalContactDiversitySummary
    (summary : TemporalContactDiversitySummary) : Prop :=
  summary.effectiveRank ≤ summary.rawCopies ∧
    summary.effectiveRank ≤ summary.rawTransmissions ∧
    summary.contactEntropyPermille ≤ 1000 ∧
    summary.contactRatePermille ≤ 1000 ∧
    summary.effectiveReproductionPermille ≤ summary.rawReproductionPermille ∧
    0 < summary.timeHorizon

theorem effective_rank_bounded_by_raw_copies
    (summary : TemporalContactDiversitySummary)
    (hValid : validTemporalContactDiversitySummary summary) :
    summary.effectiveRank ≤ summary.rawCopies := by
  -- Effective rank is a filtered independence measure, never raw copy count.
  exact hValid.left

theorem effective_rank_bounded_by_raw_transmissions
    (summary : TemporalContactDiversitySummary)
    (hValid : validTemporalContactDiversitySummary summary) :
    summary.effectiveRank ≤ summary.rawTransmissions := by
  -- Transmission volume can upper-bound, but does not define, recoverable rank.
  exact hValid.right.left

/-! ## Reconstruction Requires Effective Rank -/

/-- Effective reconstruction is k-of-n reconstruction over contact-generated rank. -/
def effectiveReconstructable
    (window : CodingWindow)
    (summary : TemporalContactDiversitySummary) : Prop :=
  window.k ≤ summary.effectiveRank

theorem reconstruction_requires_effective_fragment_rank
    (window : CodingWindow)
    (rank : ReceiverRank)
    (summary : TemporalContactDiversitySummary)
    (hRank : receiverRank rank = summary.effectiveRank)
    (hReconstructable : reconstructable window rank) :
    effectiveReconstructable window summary := by
  -- Ordinary k-of-n recovery becomes an effective-rank obligation once the
  -- receiver rank is identified with contact-generated independent rank.
  unfold effectiveReconstructable
  unfold reconstructable at hReconstructable
  rw [← hRank]
  exact hReconstructable

theorem effective_rank_reconstruction_suffices
    (window : CodingWindow)
    (summary : TemporalContactDiversitySummary)
    (hEffective : effectiveReconstructable window summary) :
    window.k ≤ summary.effectiveRank := by
  -- The sufficiency direction is exactly the exposed effective-rank threshold.
  exact hEffective

/-! ## Inference-Facing Effective Independence -/

/-- Effective independence for direct statistic decoding, not only fragment recovery. -/
structure InferenceEffectiveIndependenceCertificate where
  rawCopies : Nat
  rawTransmissions : Nat
  acceptedLedgerCount : Nat
  receiverHistoryCount : Nat
  lineageDiversity : Nat
  contactDiversity : Nat
  duplicateDiscount : Nat
  effectiveTaskIndependence : Nat
  evidenceGuard : Nat
  timeHorizon : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid task-independence certificates expose all replay-facing audit caps. -/
def validInferenceEffectiveIndependenceCertificate
    (certificate : InferenceEffectiveIndependenceCertificate) : Prop :=
  certificate.effectiveTaskIndependence ≤ certificate.rawCopies ∧
    certificate.effectiveTaskIndependence ≤ certificate.rawTransmissions ∧
    certificate.effectiveTaskIndependence ≤ certificate.acceptedLedgerCount ∧
    certificate.effectiveTaskIndependence ≤ certificate.receiverHistoryCount ∧
    certificate.effectiveTaskIndependence + certificate.duplicateDiscount ≤
      certificate.lineageDiversity + certificate.contactDiversity ∧
    0 < certificate.timeHorizon

/-- A task guard is satisfied only by effective task independence. -/
def taskEffectiveEvidenceGuard
    (certificate : InferenceEffectiveIndependenceCertificate) : Prop :=
  certificate.evidenceGuard ≤ certificate.effectiveTaskIndependence

theorem effective_task_independence_bounded_by_raw_copies
    (certificate : InferenceEffectiveIndependenceCertificate)
    (hValid : validInferenceEffectiveIndependenceCertificate certificate) :
    certificate.effectiveTaskIndependence ≤ certificate.rawCopies := by
  -- Raw copies can upper-bound useful task independence, but do not define it.
  exact hValid.left

theorem effective_task_independence_bounded_by_raw_transmissions
    (certificate : InferenceEffectiveIndependenceCertificate)
    (hValid : validInferenceEffectiveIndependenceCertificate certificate) :
    certificate.effectiveTaskIndependence ≤ certificate.rawTransmissions := by
  -- Raw transmissions can upper-bound useful task independence, but do not define it.
  exact hValid.right.left

theorem effective_task_independence_connected_to_audit_fields
    (certificate : InferenceEffectiveIndependenceCertificate)
    (hValid : validInferenceEffectiveIndependenceCertificate certificate) :
    certificate.effectiveTaskIndependence ≤ certificate.acceptedLedgerCount ∧
      certificate.effectiveTaskIndependence ≤ certificate.receiverHistoryCount ∧
      certificate.effectiveTaskIndependence + certificate.duplicateDiscount ≤
        certificate.lineageDiversity + certificate.contactDiversity := by
  -- The inference-facing object is tied to ledgers, histories, lineage diversity,
  -- contact diversity, and duplicate discounting.
  exact ⟨hValid.right.right.left, hValid.right.right.right.left,
    hValid.right.right.right.right.left⟩

/-- Guarded statistic commitment paired with the effective task-evidence guard. -/
def effectiveGuardedStatisticCommitment
    (certificate : InferenceEffectiveIndependenceCertificate)
    (guard : AnomalyCommitmentGuard)
    (statistic : AdditiveScoreStatistic) : Prop :=
  guardPassesOnStatistic guard statistic ∧
    guard.evidenceGuard ≤ certificate.effectiveTaskIndependence

theorem direct_statistic_commitment_requires_task_effective_guard
    (certificate : InferenceEffectiveIndependenceCertificate)
    (guard : AnomalyCommitmentGuard)
    (statistic : AdditiveScoreStatistic)
    (hCommit : effectiveGuardedStatisticCommitment certificate guard statistic) :
    guard.evidenceGuard ≤ certificate.effectiveTaskIndependence := by
  -- Direct statistic commitment is guarded by effective task evidence, not raw copies.
  exact hCommit.right

/-- High raw spread but too little effective task independence for a guard. -/
def highRawLowTaskIndependenceCertificate :
    InferenceEffectiveIndependenceCertificate :=
  { rawCopies := 80
    rawTransmissions := 120
    acceptedLedgerCount := 1
    receiverHistoryCount := 1
    lineageDiversity := 1
    contactDiversity := 1
    duplicateDiscount := 1
    effectiveTaskIndependence := 1
    evidenceGuard := 3
    timeHorizon := 20 }

theorem high_raw_spread_does_not_imply_task_effective_independence :
    highRawLowTaskIndependenceCertificate.evidenceGuard ≤
        highRawLowTaskIndependenceCertificate.rawCopies ∧
      highRawLowTaskIndependenceCertificate.evidenceGuard ≤
        highRawLowTaskIndependenceCertificate.rawTransmissions ∧
      ¬ taskEffectiveEvidenceGuard highRawLowTaskIndependenceCertificate := by
  -- Raw copies and transmissions exceed the guard, but effective task evidence does not.
  unfold highRawLowTaskIndependenceCertificate taskEffectiveEvidenceGuard
  simp

/-- Exact k-of-n recovery as an inference effective-independence instance. -/
def exactReconstructionEffectiveIndependence
    (window : CodingWindow)
    (summary : TemporalContactDiversitySummary) :
    InferenceEffectiveIndependenceCertificate :=
  { rawCopies := summary.rawCopies
    rawTransmissions := summary.rawTransmissions
    acceptedLedgerCount := summary.effectiveRank
    receiverHistoryCount := summary.effectiveRank
    lineageDiversity := summary.distinctCarrierLineages
    contactDiversity := summary.distinctBridgeCrossings
    duplicateDiscount := summary.rawCopies - summary.effectiveRank
    effectiveTaskIndependence := summary.effectiveRank
    evidenceGuard := window.k
    timeHorizon := summary.timeHorizon }

theorem exact_k_of_n_effective_guard_is_reconstruction_threshold
    (window : CodingWindow)
    (summary : TemporalContactDiversitySummary) :
    (exactReconstructionEffectiveIndependence window summary).evidenceGuard =
      window.k := by
  -- Exact reconstruction remains the set-union threshold instance.
  rfl

/-- Additive anomaly localization as an inference effective-independence instance. -/
def additiveAnomalyEffectiveIndependence
    (guard : AnomalyCommitmentGuard)
    (statistic : AdditiveScoreStatistic)
    (summary : TemporalContactDiversitySummary) :
    InferenceEffectiveIndependenceCertificate :=
  { rawCopies := summary.rawCopies
    rawTransmissions := summary.rawTransmissions
    acceptedLedgerCount := statistic.evidenceCount
    receiverHistoryCount := statistic.evidenceCount
    lineageDiversity := summary.distinctCarrierLineages
    contactDiversity := summary.distinctBridgeCrossings
    duplicateDiscount := summary.rawCopies - statistic.evidenceCount
    effectiveTaskIndependence := Nat.min statistic.evidenceCount summary.effectiveRank
    evidenceGuard := guard.evidenceGuard
    timeHorizon := summary.timeHorizon }

theorem additive_anomaly_effective_guard_matches_commitment_guard
    (guard : AnomalyCommitmentGuard)
    (statistic : AdditiveScoreStatistic)
    (summary : TemporalContactDiversitySummary) :
    (additiveAnomalyEffectiveIndependence guard statistic summary).evidenceGuard =
      guard.evidenceGuard := by
  -- The anomaly task uses the declared commitment evidence guard.
  rfl

/-- Majority or threshold task as a compact mergeable effective-independence instance. -/
def majorityThresholdEffectiveIndependence
    (voteCount guardThreshold : Nat)
    (summary : TemporalContactDiversitySummary) :
    InferenceEffectiveIndependenceCertificate :=
  { rawCopies := summary.rawCopies
    rawTransmissions := summary.rawTransmissions
    acceptedLedgerCount := voteCount
    receiverHistoryCount := voteCount
    lineageDiversity := summary.distinctCarrierLineages
    contactDiversity := summary.distinctBridgeCrossings
    duplicateDiscount := summary.rawCopies - voteCount
    effectiveTaskIndependence := Nat.min voteCount summary.effectiveRank
    evidenceGuard := guardThreshold
    timeHorizon := summary.timeHorizon }

theorem majority_threshold_effective_guard_matches_task_threshold
    (voteCount guardThreshold : Nat)
    (summary : TemporalContactDiversitySummary) :
    (majorityThresholdEffectiveIndependence voteCount guardThreshold summary).evidenceGuard =
      guardThreshold := by
  -- The second compact mergeable task uses its own declared evidence threshold.
  rfl

/-! ## Independence-Limited Recovery Bound -/

/-- Permille certificate for `P_recover(T) <= P(I_T >= k)`. -/
structure IndependenceLimitedRecoveryBound where
  window : CodingWindow
  summary : TemporalContactDiversitySummary
  recoverPermille : Permille
  effectiveRankAtLeastKPermille : Permille
  deriving Inhabited, Repr, DecidableEq, BEq

def validIndependenceLimitedRecoveryBound
    (bound : IndependenceLimitedRecoveryBound) : Prop :=
  validTemporalContactDiversitySummary bound.summary ∧
    bound.recoverPermille ≤ bound.effectiveRankAtLeastKPermille ∧
    bound.effectiveRankAtLeastKPermille ≤ 1000

theorem recovery_probability_bounded_by_effective_independence
    (bound : IndependenceLimitedRecoveryBound)
    (hValid : validIndependenceLimitedRecoveryBound bound) :
    bound.recoverPermille ≤ bound.effectiveRankAtLeastKPermille ∧
      bound.effectiveRankAtLeastKPermille ≤ 1000 := by
  -- The stochastic-looking paper claim is represented as an audited permille bound.
  exact ⟨hValid.right.left, hValid.right.right⟩

/-! ## Raw Spread Counterexamples -/

/-- A trace with many copies but only one effectively independent fragment. -/
def highCopyLowIndependenceSummary : TemporalContactDiversitySummary :=
  { nodeCount := 25
    contactRatePermille := 700
    rawTransmissions := 120
    rawCopies := 80
    effectiveRank := 1
    contactEntropyPermille := 90
    distinctCarrierLineages := 1
    distinctBridgeCrossings := 0
    timeHorizon := 20
    byteBudget := 4096
    storageBudget := 2048
    observabilityCost := 80
    rawReproductionPermille := 1800
    effectiveReproductionPermille := 300 }

def highCopyLowIndependenceWindow : CodingWindow :=
  { k := 3, n := 8 }

theorem many_copies_do_not_imply_many_independent_fragments :
    highCopyLowIndependenceWindow.k ≤ highCopyLowIndependenceSummary.rawCopies ∧
      ¬ effectiveReconstructable
          highCopyLowIndependenceWindow highCopyLowIndependenceSummary := by
  -- Raw copies exceed k, but effective rank is one and cannot reconstruct k = 3.
  unfold highCopyLowIndependenceWindow highCopyLowIndependenceSummary
  simp [effectiveReconstructable]

/-! ## Raw Reproduction Versus Effective Reproduction -/

/-- Control-facing split between raw copies and independent useful fragments. -/
structure EffectiveReproductionSummary where
  existingActiveFragments : Nat
  newRawCopies : Nat
  newIndependentUsefulFragments : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validEffectiveReproductionSummary
    (summary : EffectiveReproductionSummary) : Prop :=
  summary.newIndependentUsefulFragments ≤ summary.newRawCopies

theorem raw_reproduction_above_one_does_not_imply_reconstruction :
    highCopyLowIndependenceSummary.rawReproductionPermille > 1000 ∧
      ¬ effectiveReconstructable
          highCopyLowIndependenceWindow highCopyLowIndependenceSummary := by
  -- Apparent R > 1 can be trapped in correlated circulation with low rank_eff.
  unfold highCopyLowIndependenceSummary highCopyLowIndependenceWindow
  simp [effectiveReconstructable]

/-! ## Matched Temporal Networks -/

/-- Two traces matched on raw spread and budget but separated by contact diversity. -/
structure MatchedTemporalNetworkPair where
  correlated : TemporalContactDiversitySummary
  diverse : TemporalContactDiversitySummary
  deriving Inhabited, Repr, DecidableEq, BEq

def matchedOnRawSpreadAndBudget
    (pair : MatchedTemporalNetworkPair) : Prop :=
  pair.correlated.nodeCount = pair.diverse.nodeCount ∧
    pair.correlated.contactRatePermille = pair.diverse.contactRatePermille ∧
    pair.correlated.byteBudget = pair.diverse.byteBudget ∧
    pair.correlated.rawTransmissions = pair.diverse.rawTransmissions ∧
    pair.correlated.rawReproductionPermille =
      pair.diverse.rawReproductionPermille

def differentEffectiveRankOutcome
    (window : CodingWindow)
    (pair : MatchedTemporalNetworkPair) : Prop :=
  ¬ effectiveReconstructable window pair.correlated ∧
    effectiveReconstructable window pair.diverse

theorem same_budget_and_raw_spread_can_have_different_reconstruction
    (window : CodingWindow)
    (pair : MatchedTemporalNetworkPair)
    (hMatched : matchedOnRawSpreadAndBudget pair)
    (hOutcome : differentEffectiveRankOutcome window pair) :
    matchedOnRawSpreadAndBudget pair ∧
      ¬ effectiveReconstructable window pair.correlated ∧
      effectiveReconstructable window pair.diverse := by
  -- Equal raw spread does not force equal reconstruction once effective rank differs.
  exact ⟨hMatched, hOutcome.left, hOutcome.right⟩

/-! ## Limit Triangle And Effective-Rank Control -/

/-- Abstract cost-time-independence boundary for finite replay certificates. -/
structure CostTimeIndependenceBoundary where
  costBudget : Nat
  timeOpportunity : Nat
  independenceTarget : Nat
  costSpent : Nat
  timeUsed : Nat
  achievedEffectiveRank : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validCostTimeIndependenceBoundary
    (boundary : CostTimeIndependenceBoundary) : Prop :=
  boundary.achievedEffectiveRank ≤ boundary.costSpent + boundary.timeUsed ∧
    boundary.costBudget + boundary.timeOpportunity <
      boundary.independenceTarget

theorem cost_time_independence_triangle_incompatibility
    (boundary : CostTimeIndependenceBoundary)
    (hValid : validCostTimeIndependenceBoundary boundary) :
    ¬ (boundary.costSpent ≤ boundary.costBudget ∧
        boundary.timeUsed ≤ boundary.timeOpportunity ∧
        boundary.independenceTarget ≤ boundary.achievedEffectiveRank) := by
  -- Low cost and fast time together cannot certify more independence than the
  -- combined opportunity budget permits.
  intro hAll
  have hSpent :
      boundary.costSpent + boundary.timeUsed ≤
        boundary.costBudget + boundary.timeOpportunity :=
    Nat.add_le_add hAll.left hAll.right.left
  have hTargetLeBudget :
      boundary.independenceTarget ≤
        boundary.costBudget + boundary.timeOpportunity :=
    Nat.le_trans hAll.right.right (Nat.le_trans hValid.left hSpent)
  exact Nat.not_lt_of_ge hTargetLeBudget hValid.right

theorem effective_reproduction_tracks_independent_useful_fragments
    (summary : EffectiveReproductionSummary)
    (hValid : validEffectiveReproductionSummary summary) :
    summary.newIndependentUsefulFragments ≤ summary.newRawCopies := by
  -- Effective reproduction is the independent useful numerator, not raw copying.
  exact hValid

/-! ## Final Independence-Limit Certificate -/

/-- Final certificate packaging the temporal independence limit. -/
structure DistributedErrorCorrectionIndependenceLimit where
  window : CodingWindow
  diversity : TemporalContactDiversitySummary
  recoveryBound : IndependenceLimitedRecoveryBound
  reproduction : EffectiveReproductionSummary
  boundary : CostTimeIndependenceBoundary
  deriving Inhabited, Repr, DecidableEq, BEq

def validDistributedErrorCorrectionIndependenceLimit
    (limit : DistributedErrorCorrectionIndependenceLimit) : Prop :=
  validTemporalContactDiversitySummary limit.diversity ∧
    validIndependenceLimitedRecoveryBound limit.recoveryBound ∧
    limit.recoveryBound.window = limit.window ∧
    limit.recoveryBound.summary = limit.diversity ∧
    validEffectiveReproductionSummary limit.reproduction ∧
    validCostTimeIndependenceBoundary limit.boundary

theorem distributed_error_correction_independence_limit
    (limit : DistributedErrorCorrectionIndependenceLimit)
    (hValid : validDistributedErrorCorrectionIndependenceLimit limit) :
    limit.diversity.effectiveRank ≤ limit.diversity.rawTransmissions ∧
      limit.recoveryBound.recoverPermille ≤
        limit.recoveryBound.effectiveRankAtLeastKPermille ∧
      limit.reproduction.newIndependentUsefulFragments ≤
        limit.reproduction.newRawCopies ∧
      ¬ (limit.boundary.costSpent ≤ limit.boundary.costBudget ∧
          limit.boundary.timeUsed ≤ limit.boundary.timeOpportunity ∧
          limit.boundary.independenceTarget ≤
            limit.boundary.achievedEffectiveRank) := by
  -- The packaged theorem states the limit: temporal error correction is
  -- bounded by certified effective independence, not raw redundancy alone.
  exact
    ⟨ effective_rank_bounded_by_raw_transmissions limit.diversity hValid.left
    , hValid.right.left.right.left
    , effective_reproduction_tracks_independent_useful_fragments
        limit.reproduction hValid.right.right.right.right.left
    , cost_time_independence_triangle_incompatibility
        limit.boundary hValid.right.right.right.right.right ⟩

end FieldActiveBelief
