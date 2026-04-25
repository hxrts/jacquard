import Field.ActiveBeliefStrong

/-
The Problem. The strong active-belief theorem stack proves semantic safety,
guarded statistic decoding, and positive lead-time witnesses. The paper also
needs a definitive theorem-closure layer: active demand should have a named
same-budget improvement condition, mergeable inference should be generic rather
than task-specific, early commitment should be separated from full recovery, and
resource, aggregation, stress, and observer-boundary claims should have explicit
assumption records.

Solution Structure.
1. State active-versus-passive demand improvement over the same finite budget.
2. Define a generic mergeable-statistic interface and instantiate the paper's
   main task families.
3. Add commitment-before-recovery, receiver-set compatibility, and
   near-critical control assumption records.
4. Keep aggregation efficiency, negative boundaries, bounded stress, and
   observer leakage as narrow theorem surfaces with clear non-claims.
-/

/-! # Active Belief Diffusion — definitive theorem closure -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-! ## Active Demand Improvement -/

/-- Same-budget comparison between passive and demand-guided diffusion. -/
structure DemandGuidedComparison where
  horizon : Nat
  byteBudget : Nat
  passiveUsefulArrivals : Nat
  activeUsefulArrivals : Nat
  passiveUncertainty : Nat
  activeUncertainty : Nat
  passiveCommitmentTime? : Option Nat
  activeCommitmentTime? : Option Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Demand-quality assumption: active priority exposes at least passive value. -/
def demandQualityNonWorse
    (comparison : DemandGuidedComparison) : Prop :=
  comparison.passiveUsefulArrivals ≤ comparison.activeUsefulArrivals ∧
    comparison.activeUncertainty ≤ comparison.passiveUncertainty

/-- Commitment time is non-worse when both policies commit and active is earlier. -/
def activeCommitmentTimeNonWorse
    (comparison : DemandGuidedComparison) : Prop :=
  match comparison.passiveCommitmentTime?, comparison.activeCommitmentTime? with
  | some passiveTime, some activeTime => activeTime ≤ passiveTime
  | none, some _activeTime => True
  | _, _ => True

theorem demand_guided_useful_arrivals_nonworse
    (comparison : DemandGuidedComparison)
    (hQuality : demandQualityNonWorse comparison) :
    comparison.passiveUsefulArrivals ≤ comparison.activeUsefulArrivals := by
  -- The demand-quality assumption exposes useful-arrival dominance directly.
  exact hQuality.left

theorem demand_guided_uncertainty_nonworse
    (comparison : DemandGuidedComparison)
    (hQuality : demandQualityNonWorse comparison) :
    comparison.activeUncertainty ≤ comparison.passiveUncertainty := by
  -- Demand-guided allocation is useful only under an explicit uncertainty condition.
  exact hQuality.right

theorem demand_guided_commitment_time_nonworse
    (comparison : DemandGuidedComparison)
    (hTime : activeCommitmentTimeNonWorse comparison) :
    activeCommitmentTimeNonWorse comparison := by
  -- Commitment-time improvement remains an assumption unless both event times are known.
  exact hTime

theorem demand_guided_quality_per_byte_nonworse
    (comparison : DemandGuidedComparison)
    (hQuality : demandQualityNonWorse comparison)
    (hBudget : 0 < comparison.byteBudget) :
    comparison.passiveUsefulArrivals * comparison.byteBudget ≤
      comparison.activeUsefulArrivals * comparison.byteBudget := by
  -- Equal positive byte budget lets useful-arrival dominance lift to quality per byte.
  have _budgetPositive : 0 < comparison.byteBudget := hBudget
  exact Nat.mul_le_mul_right comparison.byteBudget hQuality.left

theorem demand_guided_reaches_threshold_when_passive_reaches
    (comparison : DemandGuidedComparison)
    (threshold : Nat)
    (hQuality : demandQualityNonWorse comparison)
    (hPassive : threshold ≤ comparison.passiveUsefulArrivals) :
    threshold ≤ comparison.activeUsefulArrivals := by
  -- Under the clean value-order assumption, active demand reaches any useful-arrival
  -- threshold reached by passive control under the same comparison.
  exact Nat.le_trans hPassive hQuality.left

/-- Fixed-threshold efficiency certificate for the clean active-demand theorem. -/
structure DemandThresholdEfficiencyCertificate where
  threshold : Nat
  passiveUsefulTransmissions : Nat
  activeUsefulTransmissions : Nat
  passiveQuality : Nat
  activeQuality : Nat
  byteBudget : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validDemandThresholdEfficiencyCertificate
    (certificate : DemandThresholdEfficiencyCertificate) : Prop :=
  certificate.threshold ≤ certificate.passiveQuality ∧
    certificate.threshold ≤ certificate.activeQuality ∧
    certificate.activeUsefulTransmissions ≤ certificate.passiveUsefulTransmissions ∧
    0 < certificate.byteBudget

theorem demand_guided_threshold_with_no_more_useful_transmissions
    (certificate : DemandThresholdEfficiencyCertificate)
    (hValid : validDemandThresholdEfficiencyCertificate certificate) :
    certificate.threshold ≤ certificate.activeQuality ∧
      certificate.activeUsefulTransmissions ≤
        certificate.passiveUsefulTransmissions := by
  -- This is the narrow clean-model theorem: threshold success and transmission
  -- non-worsening are assumptions made explicit, not an optimality claim.
  exact ⟨hValid.right.left, hValid.right.right.left⟩

/-! ## Generic Mergeable-Inference Soundness -/

/-- Generic task interface for direct decoding from a mergeable statistic. -/
structure MergeableStatistic where
  Carrier : Type
  identity : Carrier
  merge : Carrier → Carrier → Carrier
  decision : Carrier → HypothesisId
  quality : Carrier → QualitySummary
  guard : Carrier → Prop

/-- Runtime state for a generic mergeable statistic plus contribution ledger. -/
structure GenericStatisticState (task : MergeableStatistic) where
  acceptedIds : List ContributionId
  statistic : task.Carrier

/-- One accepted contribution for a generic mergeable statistic. -/
structure GenericStatisticContribution (task : MergeableStatistic) where
  contributionId : ContributionId
  statisticContribution : task.Carrier

/-- Generic acceptance suppresses duplicates before merging the statistic. -/
def genericStatisticAccept
    (task : MergeableStatistic)
    (state : GenericStatisticState task)
    (contribution : GenericStatisticContribution task) :
    GenericStatisticState task :=
  if contribution.contributionId ∈ state.acceptedIds then
    state
  else
    { acceptedIds := state.acceptedIds ++ [contribution.contributionId]
      statistic := task.merge state.statistic contribution.statisticContribution }

/-- Direct decoding reads the decision map from the audited merged statistic. -/
def genericStatisticDecision
    (task : MergeableStatistic)
    (state : GenericStatisticState task) : HypothesisId :=
  task.decision state.statistic

theorem generic_duplicate_preserves_statistic_state
    (task : MergeableStatistic)
    (state : GenericStatisticState task)
    (contribution : GenericStatisticContribution task)
    (hPresent : contribution.contributionId ∈ state.acceptedIds) :
    genericStatisticAccept task state contribution = state := by
  -- Duplicate suppression happens before the merge operation can run.
  simp [genericStatisticAccept, hPresent]

theorem generic_direct_statistic_decoding
    (task : MergeableStatistic)
    (state : GenericStatisticState task) :
    genericStatisticDecision task state = task.decision state.statistic := by
  -- The generic interface makes the statistic, not raw observations, the decision object.
  rfl

/-- Generic aggregate validity: the aggregate is exactly the declared merge. -/
def validGenericAggregate
    (task : MergeableStatistic)
    (left right aggregate : task.Carrier) : Prop :=
  aggregate = task.merge left right

theorem generic_aggregate_preserves_statistic
    (task : MergeableStatistic)
    (left right aggregate : task.Carrier)
    (hValid : validGenericAggregate task left right aggregate) :
    aggregate = task.merge left right := by
  -- Aggregation soundness is explicit equality with the task merge.
  exact hValid

/-- Exact reconstruction as a set-union-style contribution-ledger task. -/
def reconstructionMergeableStatistic : MergeableStatistic :=
  { Carrier := List ContributionId
    identity := []
    merge := fun left right => left ++ right
    decision := fun ids => ids.length
    quality := fun ids =>
      { uncertainty := 0
        topMargin := ids.length
        evidenceCount := ids.length }
    guard := fun ids => 0 < ids.length }

/-- Additive anomaly localization as a mergeable-statistic task surface. -/
def additiveScoreMergeableStatistic : MergeableStatistic :=
  { Carrier := AdditiveScoreStatistic
    identity := default
    merge := fun left right =>
      { topHypothesis := left.topHypothesis
        runnerUpHypothesis := left.runnerUpHypothesis
        topMargin := left.topMargin + right.topMargin
        evidenceCount := left.evidenceCount + right.evidenceCount }
    decision := statisticDecision
    quality := qualityOfStatistic
    guard := fun statistic => 0 < statistic.evidenceCount }

/-- Majority or threshold decisions as a compact mergeable task surface. -/
def majorityThresholdMergeableStatistic : MergeableStatistic :=
  { Carrier := Nat
    identity := 0
    merge := fun left right => left + right
    decision := fun voteCount => voteCount
    quality := fun voteCount =>
      { uncertainty := 0
        topMargin := voteCount
        evidenceCount := voteCount }
    guard := fun voteCount => 0 < voteCount }

/-! ## Commitment Before Full Recovery Bound -/

/-- Finite-horizon event separating task commitment from full recovery. -/
structure CommitmentBeforeRecoveryBound where
  usefulBound : UsefulInferenceArrivalBound
  scoreModel : BoundedScoreVectorUpdateModel
  guard : AnomalyCommitmentGuard
  horizon : Nat
  commitmentPermilleFloor : Permille
  fullRecoveryPermilleFloor : Permille
  commitmentBeforeRecoveryPermilleFloor : Permille
  deriving Inhabited, Repr, DecidableEq, BEq

def validCommitmentBeforeRecoveryBound
    (bound : CommitmentBeforeRecoveryBound) : Prop :=
  usefulInferenceArrivalBoundValid bound.usefulBound ∧
    validBoundedScoreVectorUpdateModel bound.scoreModel ∧
    validAnomalyCommitmentGuard bound.guard ∧
    bound.commitmentBeforeRecoveryPermilleFloor ≤ bound.commitmentPermilleFloor ∧
    bound.commitmentBeforeRecoveryPermilleFloor ≤ 1000

theorem commitment_before_full_recovery_lower_bound
    (bound : CommitmentBeforeRecoveryBound)
    (hValid : validCommitmentBeforeRecoveryBound bound) :
    bound.commitmentBeforeRecoveryPermilleFloor ≤
      bound.commitmentPermilleFloor := by
  -- The finite-horizon lower bound is recorded separately from full recovery.
  exact hValid.right.right.right.left

theorem commitment_before_recovery_bound_is_permille
    (bound : CommitmentBeforeRecoveryBound)
    (hValid : validCommitmentBeforeRecoveryBound bound) :
    bound.commitmentBeforeRecoveryPermilleFloor ≤ 1000 := by
  -- Probability-style bounds remain explicit permille values in artifacts.
  exact hValid.right.right.right.right

/-! ## Multi-Receiver Compatibility Bound -/

/-- Receiver-set compatibility summary for guarded local commitments. -/
structure ReceiverSetCompatibility where
  receiverCount : Nat
  committedCount : Nat
  compatibleCount : Nat
  disagreementPermilleBound : Permille
  deriving Inhabited, Repr, DecidableEq, BEq

def validReceiverSetCompatibility
    (summary : ReceiverSetCompatibility) : Prop :=
  summary.compatibleCount ≤ summary.committedCount ∧
    summary.committedCount ≤ summary.receiverCount ∧
    summary.disagreementPermilleBound ≤ 1000

theorem receiver_set_compatibility_bounded
    (summary : ReceiverSetCompatibility)
    (hValid : validReceiverSetCompatibility summary) :
    summary.compatibleCount ≤ summary.receiverCount := by
  -- Compatible commitments are a subset of committed receivers, then all receivers.
  exact Nat.le_trans hValid.left hValid.right.left

theorem receiver_disagreement_permille_bounded
    (summary : ReceiverSetCompatibility)
    (hValid : validReceiverSetCompatibility summary) :
    summary.disagreementPermilleBound ≤ 1000 := by
  -- Any probabilistic disagreement claim remains a bounded permille statement.
  exact hValid.right.right

/-! ## Near-Critical Controller Stabilization -/

/-- Finite-horizon controller band over achieved reproduction pressure. -/
structure NearCriticalControllerBand where
  lowerPermille : Permille
  achievedPermille : Permille
  upperPermille : Permille
  beforePotential : Nat
  afterPotential : Nat
  progressCredit : Nat
  pressureBudget : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validNearCriticalControllerBand
    (band : NearCriticalControllerBand) : Prop :=
  band.lowerPermille ≤ band.achievedPermille ∧
    band.achievedPermille ≤ band.upperPermille ∧
    band.upperPermille ≤ 1000 ∧
    band.afterPotential + band.progressCredit ≤
      band.beforePotential + band.pressureBudget

theorem near_critical_controller_keeps_pressure_in_band
    (band : NearCriticalControllerBand)
    (hValid : validNearCriticalControllerBand band) :
    band.lowerPermille ≤ band.achievedPermille ∧
      band.achievedPermille ≤ band.upperPermille := by
  -- The controller theorem is about achieved pressure, not only target pressure.
  exact ⟨hValid.left, hValid.right.left⟩

theorem near_critical_controller_bounds_potential
    (band : NearCriticalControllerBand)
    (hValid : validNearCriticalControllerBand band) :
    band.afterPotential + band.progressCredit ≤
      band.beforePotential + band.pressureBudget := by
  -- Potential growth is bounded by the declared finite-horizon pressure budget.
  exact hValid.right.right.right

/-! ## Aggregation And Recoding Efficiency -/

/-- Cost witness for a valid aggregate versus raw contribution forwarding. -/
structure AggregateCostComparison where
  rawContributionBytes : Nat
  aggregateBytes : Nat
  custodyRawBytes : Nat
  custodyAggregateBytes : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def aggregateCostNonWorse
    (comparison : AggregateCostComparison) : Prop :=
  comparison.aggregateBytes ≤ comparison.rawContributionBytes ∧
    comparison.custodyAggregateBytes ≤ comparison.custodyRawBytes

theorem valid_aggregate_cost_nonworse
    (comparison : AggregateCostComparison)
    (hCost : aggregateCostNonWorse comparison) :
    comparison.aggregateBytes ≤ comparison.rawContributionBytes ∧
      comparison.custodyAggregateBytes ≤ comparison.custodyRawBytes := by
  -- Cost improvement is separate from semantic preservation and must be assumed explicitly.
  exact hCost

theorem aggregate_duplicate_preserves_generic_state
    (task : MergeableStatistic)
    (state : GenericStatisticState task)
    (aggregate : GenericStatisticContribution task)
    (hPresent : aggregate.contributionId ∈ state.acceptedIds) :
    genericStatisticAccept task state aggregate = state := by
  -- Aggregates use the same contribution-ledger duplicate gate as ordinary evidence.
  exact generic_duplicate_preserves_statistic_state task state aggregate hPresent

/-! ## Negative Boundary Counterexamples -/

/-- Without contribution identity, repeated arrivals can be counted twice. -/
def noLedgerDuplicateCounterexample : Nat :=
  1 + 1

theorem no_ledger_duplicate_can_change_result :
    noLedgerDuplicateCounterexample ≠ 1 := by
  -- This tiny counterexample motivates contribution ledgers for duplicate suppression.
  decide

/-- Natural-number subtraction gives a compact non-associative merge example. -/
def nonAssociativeMerge (left right : Nat) : Nat :=
  left - right

theorem non_associative_merge_order_counterexample :
    nonAssociativeMerge (nonAssociativeMerge 5 3) 1 ≠
      nonAssociativeMerge 5 (nonAssociativeMerge 3 1) := by
  -- Without associativity, merge grouping can change the decoded result.
  decide

/-! ## Bounded Adversarial Stress -/

/-- Finite stress budget for duplicate spam, stale demand, and withholding. -/
structure BoundedStressBudget where
  duplicateSpamBudget : Nat
  staleDemandBudget : Nat
  withholdingBudget : Nat
  observedDuplicateSpam : Nat
  observedStaleDemand : Nat
  observedWithholding : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validBoundedStressBudget (budget : BoundedStressBudget) : Prop :=
  budget.observedDuplicateSpam ≤ budget.duplicateSpamBudget ∧
    budget.observedStaleDemand ≤ budget.staleDemandBudget ∧
    budget.observedWithholding ≤ budget.withholdingBudget

theorem duplicate_spam_rank_safety
    (summary : DemandSummary)
    (proposal : EvidenceProposal)
    (rank : ReceiverRank)
    (budget : BoundedStressBudget)
    (hBudget : validBoundedStressBudget budget)
    (hValid : proposal.validEvidence = true)
    (hPresent : proposal.contributionId ∈ rank.contributionIds) :
    receiverRank (demandAwareAccept summary proposal rank) =
      receiverRank rank ∧
      budget.observedDuplicateSpam ≤ budget.duplicateSpamBudget := by
  -- Bounded duplicate spam cannot inflate an already-counted contribution.
  exact
    ⟨ demand_duplicate_non_inflation summary proposal rank hValid hPresent
    , hBudget.left ⟩

theorem stale_demand_stress_cannot_validate_invalid_evidence
    (summary : DemandSummary)
    (proposal : EvidenceProposal)
    (rank : ReceiverRank)
    (budget : BoundedStressBudget)
    (hBudget : validBoundedStressBudget budget)
    (hExpired : expiredDemandSummary summary)
    (hInvalid : proposal.validEvidence = false) :
    demandAwareAccept summary proposal rank = rank ∧
      budget.observedStaleDemand ≤ budget.staleDemandBudget := by
  -- Stale demand remains non-evidential even when stress rows are present.
  exact
    ⟨ expired_demand_does_not_accept_invalid_evidence
        summary proposal rank hExpired hInvalid
    , hBudget.right.left ⟩

/-! ## Observer Leakage Bound -/

/-- Explicit observer projection for the measured ambiguity frontier. -/
structure ObserverLeakageProjection where
  visibleFragments : Nat
  hiddenFragments : Nat
  linkageCandidates : Nat
  observerAdvantagePermille : Permille
  deriving Inhabited, Repr, DecidableEq, BEq

def validObserverLeakageProjection
    (projection : ObserverLeakageProjection) : Prop :=
  0 < projection.linkageCandidates ∧
    projection.observerAdvantagePermille ≤ 1000

def observerAmbiguity
    (projection : ObserverLeakageProjection) : Nat :=
  projection.hiddenFragments + projection.linkageCandidates

theorem observer_leakage_permille_bounded
    (projection : ObserverLeakageProjection)
    (hValid : validObserverLeakageProjection projection) :
    projection.observerAdvantagePermille ≤ 1000 := by
  -- The observer result is a bounded projection metric, not a privacy theorem.
  exact hValid.right

theorem observer_ambiguity_preserves_hidden_fragments
    (projection : ObserverLeakageProjection) :
    projection.hiddenFragments ≤ observerAmbiguity projection := by
  -- Ambiguity includes the hidden fragment count plus linkage candidates.
  exact Nat.le_add_right projection.hiddenFragments projection.linkageCandidates

end FieldActiveBelief
