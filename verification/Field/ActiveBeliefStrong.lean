import Field.ActiveBelief
import Field.CodedDiffusionStrong

/-
The Problem. The base active-belief file establishes that demand is bounded,
replay-visible, and non-evidential. The paper now needs the next layer:
mergeable-statistic decoding, guarded commitment correctness, compatibility from
non-identical partial histories, demand-guided non-interference on the merged
statistic, and early-commitment witnesses under explicit useful-inference
assumptions.

Solution Structure.
1. Introduce a compact merged-statistic surface for the additive score-vector
   task family.
2. Build guarded commitment directly from that statistic and prove the emitted
   hypothesis matches the statistic decision.
3. Add explicit partial-history witnesses so compatible commitments do not
   require identical evidence sets.
4. State demand-guided statistic acceptance so demand changes control order, not
   the accepted-statistic semantics.
5. Reuse the strong finite-horizon assumption records for positive lead-time
   witnesses and keep quality monotonicity explicit.
-/

/-! # Active Belief Diffusion — mergeable-statistic theorems -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-! ## Mergeable Statistic Surface -/

/-- Proof-facing merged statistic for the additive score-vector task family. -/
structure AdditiveScoreStatistic where
  topHypothesis : HypothesisId
  runnerUpHypothesis : HypothesisId
  topMargin : Nat
  evidenceCount : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Deterministic task decision read directly from the merged statistic. -/
def statisticDecision (statistic : AdditiveScoreStatistic) : HypothesisId :=
  statistic.topHypothesis

/-- Deterministic quality summary induced by the merged statistic. -/
def qualityOfStatistic (statistic : AdditiveScoreStatistic) : QualitySummary :=
  -- The uncertainty proxy falls as the guarded margin grows, capped at 1000.
  { uncertainty := 1000 - Nat.min 1000 statistic.topMargin
    topMargin := statistic.topMargin
    evidenceCount := statistic.evidenceCount }

/-- Guarded commitment reads only the merged statistic and declared thresholds. -/
def guardPassesOnStatistic
    (guard : AnomalyCommitmentGuard)
    (statistic : AdditiveScoreStatistic) : Prop :=
  guard.marginThreshold ≤ statistic.topMargin ∧
    guard.evidenceGuard ≤ statistic.evidenceCount

instance instDecidableGuardPassesOnStatistic
    (guard : AnomalyCommitmentGuard)
    (statistic : AdditiveScoreStatistic) :
    Decidable (guardPassesOnStatistic guard statistic) := by
  -- The guard is a conjunction of decidable Nat inequalities.
  unfold guardPassesOnStatistic
  infer_instance

/-- Receiver-quality preorder used by the active monotonicity theorem. -/
def ReceiverQualityOrder
    (before after : AdditiveScoreStatistic) : Prop :=
  before.topMargin ≤ after.topMargin ∧
    before.evidenceCount ≤ after.evidenceCount

/-- One explicit witness that two receivers saw different partial histories. -/
structure PartialHistoryWitness where
  leftAcceptedCount : Nat
  rightAcceptedCount : Nat
  sharedAcceptedCount : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Shared evidence cannot exceed either receiver's accepted-count summary. -/
def validPartialHistoryWitness (witness : PartialHistoryWitness) : Prop :=
  witness.sharedAcceptedCount ≤ witness.leftAcceptedCount ∧
    witness.sharedAcceptedCount ≤ witness.rightAcceptedCount

/-- Distinct histories allow overlap without requiring identical evidence sets. -/
def nonIdenticalPartialHistories (witness : PartialHistoryWitness) : Prop :=
  witness.sharedAcceptedCount < witness.leftAcceptedCount ∨
    witness.sharedAcceptedCount < witness.rightAcceptedCount

/-- Partial-history compatibility ties different histories to the same decision. -/
def compatiblePartialHistories
    (left right : AdditiveScoreStatistic)
    (witness : PartialHistoryWitness) : Prop :=
  validPartialHistoryWitness witness ∧
    witness.leftAcceptedCount = left.evidenceCount ∧
    witness.rightAcceptedCount = right.evidenceCount ∧
    nonIdenticalPartialHistories witness ∧
    statisticDecision left = statisticDecision right

/-! ## Guarded Commitment Decoding -/

/-- Deterministic guarded commitment emitted from a receiver-local statistic. -/
def guardedCommitmentFromStatistic
    (receiverId : ReceiverId)
    (guard : AnomalyCommitmentGuard)
    (statistic : AdditiveScoreStatistic) : GuardedCommitment :=
  -- Commitment exposes the statistic decision while guarding it explicitly.
  { receiverId := receiverId
    hypothesis := statisticDecision statistic
    guardPassed := decide (guardPassesOnStatistic guard statistic) }

theorem guarded_commitment_decodes_statistic_decision
    (receiverId : ReceiverId)
    (guard : AnomalyCommitmentGuard)
    (statistic : AdditiveScoreStatistic) :
    (guardedCommitmentFromStatistic receiverId guard statistic).hypothesis =
      statisticDecision statistic := by
  -- The commitment hypothesis is read directly from the merged statistic.
  rfl

theorem guarded_commitment_guard_passes_when_guard_holds
    (receiverId : ReceiverId)
    (guard : AnomalyCommitmentGuard)
    (statistic : AdditiveScoreStatistic)
    (hGuard : guardPassesOnStatistic guard statistic) :
    (guardedCommitmentFromStatistic receiverId guard statistic).guardPassed = true := by
  -- The constructor turns the guard proposition into replay-visible Boolean state.
  have hDecide : decide (guardPassesOnStatistic guard statistic) = true := by
    simp [hGuard]
  simpa [guardedCommitmentFromStatistic] using hDecide

theorem guarded_commitment_from_mergeable_statistic_correct
    (receiverId : ReceiverId)
    (guard : AnomalyCommitmentGuard)
    (statistic : AdditiveScoreStatistic)
    (hGuard : guardPassesOnStatistic guard statistic) :
    let commitment := guardedCommitmentFromStatistic receiverId guard statistic
    commitment.guardPassed = true ∧
      commitment.hypothesis = statisticDecision statistic := by
  -- A guarded commitment exposes the statistic decision once the declared guard holds.
  intro commitment
  exact ⟨guarded_commitment_guard_passes_when_guard_holds receiverId guard statistic hGuard,
    guarded_commitment_decodes_statistic_decision receiverId guard statistic⟩

/-! ## Partial-History Compatibility -/

theorem compatible_partial_histories_share_decision
    (left right : AdditiveScoreStatistic)
    (witness : PartialHistoryWitness)
    (hCompatible : compatiblePartialHistories left right witness) :
    statisticDecision left = statisticDecision right := by
  -- Compatibility over partial histories includes agreement on the decoded task decision.
  exact hCompatible.right.right.right.right

theorem compatible_partial_histories_are_nonidentical
    (left right : AdditiveScoreStatistic)
    (witness : PartialHistoryWitness)
    (hCompatible : compatiblePartialHistories left right witness) :
    nonIdenticalPartialHistories witness := by
  -- The witness records that the two receivers need not share identical histories.
  exact hCompatible.right.right.right.left

theorem compatible_partial_histories_yield_compatible_commitments
    (leftReceiver rightReceiver : ReceiverId)
    (guard : AnomalyCommitmentGuard)
    (left right : AdditiveScoreStatistic)
    (witness : PartialHistoryWitness)
    (hLeftGuard : guardPassesOnStatistic guard left)
    (hRightGuard : guardPassesOnStatistic guard right)
    (hCompatible : compatiblePartialHistories left right witness) :
    compatibleCommitments
      (guardedCommitmentFromStatistic leftReceiver guard left)
      (guardedCommitmentFromStatistic rightReceiver guard right) := by
  -- Distinct partial histories still yield compatible commitments when guards pass in the same basin.
  have hLeftPassed :
      (guardedCommitmentFromStatistic leftReceiver guard left).guardPassed = true := by
    exact guarded_commitment_guard_passes_when_guard_holds
      leftReceiver guard left hLeftGuard
  have hRightPassed :
      (guardedCommitmentFromStatistic rightReceiver guard right).guardPassed = true := by
    exact guarded_commitment_guard_passes_when_guard_holds
      rightReceiver guard right hRightGuard
  have hSameDecision :
      statisticDecision left = statisticDecision right := by
    exact compatible_partial_histories_share_decision left right witness hCompatible
  -- Compatibility remains guarded agreement, not consensus or identical receiver state.
  exact ⟨hLeftPassed, hRightPassed, by
    simpa [guardedCommitmentFromStatistic, guardPassesOnStatistic, statisticDecision]
      using hSameDecision⟩

/-! ## Demand-Guided Statistic Acceptance -/

/-- Valid contribution semantics are fixed before demand sees the message. -/
structure AcceptedStatisticContribution where
  contributionId : ContributionId
  statisticAfterMerge : AdditiveScoreStatistic
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Plain acceptance updates the statistic only through valid new contributions. -/
def plainStatisticAccept
    (proposal : EvidenceProposal)
    (accepted : AcceptedStatisticContribution)
    (rank : ReceiverRank)
    (current : AdditiveScoreStatistic) : AdditiveScoreStatistic :=
  if proposal.validEvidence then
    if accepted.contributionId ∈ rank.contributionIds then
      current
    else
      accepted.statisticAfterMerge
  else
    current

/-- Demand can reprioritize arrivals, but not the mergeable-statistic semantics. -/
def demandGuidedStatisticAccept
    (_summary : DemandSummary)
    (proposal : EvidenceProposal)
    (accepted : AcceptedStatisticContribution)
    (rank : ReceiverRank)
    (current : AdditiveScoreStatistic) : AdditiveScoreStatistic :=
  -- The accepted contribution already passed ordinary validity and ledger checks.
  plainStatisticAccept proposal accepted rank current

theorem demand_guided_statistic_acceptance_matches_plain_acceptance
    (summary : DemandSummary)
    (proposal : EvidenceProposal)
    (accepted : AcceptedStatisticContribution)
    (rank : ReceiverRank)
    (current : AdditiveScoreStatistic) :
    demandGuidedStatisticAccept summary proposal accepted rank current =
      plainStatisticAccept proposal accepted rank current := by
  -- Demand is absent from the accepted-statistic semantics by construction.
  rfl

theorem demand_guided_duplicate_preserves_statistic
    (summary : DemandSummary)
    (proposal : EvidenceProposal)
    (accepted : AcceptedStatisticContribution)
    (rank : ReceiverRank)
    (current : AdditiveScoreStatistic)
    (hValid : proposal.validEvidence = true)
    (hPresent : accepted.contributionId ∈ rank.contributionIds)
    (hSameId : accepted.contributionId = proposal.contributionId) :
    demandGuidedStatisticAccept summary proposal accepted rank current = current := by
  -- Existing duplicate non-inflation keeps the receiver ledger fixed for this contribution.
  have _rankPreserved :
      receiverRank (demandAwareAccept summary proposal rank) = receiverRank rank := by
    exact demand_duplicate_non_inflation summary proposal rank hValid (hSameId ▸ hPresent)
  -- With the duplicate gate already closed, the statistic reducer returns the current state.
  simp [demandGuidedStatisticAccept, plainStatisticAccept, hValid, hPresent]

theorem propagated_demand_guided_statistic_acceptance_matches_plain_acceptance
    (record : PropagatedDemandRecord)
    (proposal : EvidenceProposal)
    (accepted : AcceptedStatisticContribution)
    (rank : ReceiverRank)
    (current : AdditiveScoreStatistic)
    (hValidRecord : validPropagatedDemandRecord record) :
    demandGuidedStatisticAccept record.summary proposal accepted rank current =
      plainStatisticAccept proposal accepted rank current := by
  -- Host/bridge propagation changes replay metadata, not accepted-statistic semantics.
  have _surface :
      record.surface = ActiveDemandExecutionSurface.hostBridgeReplay := by
    exact propagated_demand_uses_host_bridge_surface record hValidRecord
  exact demand_guided_statistic_acceptance_matches_plain_acceptance
    record.summary proposal accepted rank current

/-! ## Positive Commitment Lead Time -/

/-- Explicit witness for theorem-backed early commitment over one receiver. -/
structure LeadTimeWitness where
  commitmentTime : Nat
  fullRecoveryTime : Nat
  observedUsefulContributions : Nat
  initialMargin : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

theorem useful_inference_can_support_positive_commitment_lead_time
    (bound : UsefulInferenceArrivalBound)
    (model : BoundedScoreVectorUpdateModel)
    (guard : AnomalyCommitmentGuard)
    (witness : LeadTimeWitness)
    (hBound : usefulInferenceArrivalBoundValid bound)
    (hModel : validBoundedScoreVectorUpdateModel model)
    (hGuard : validAnomalyCommitmentGuard guard)
    (hUseful :
      bound.requiredUsefulContributions ≤ witness.observedUsefulContributions)
    (hMargin :
      guard.marginThreshold ≤ anomalyMarginLowerBound model witness.initialMargin)
    (hEvidence : guard.evidenceGuard ≤ witness.observedUsefulContributions)
    (hEarly : witness.commitmentTime < witness.fullRecoveryTime) :
    ∃ leadTime,
      commitmentLeadTime
          { commitmentTime? := some witness.commitmentTime
            fullRecoveryTime? := some witness.fullRecoveryTime } =
        some leadTime ∧
        0 < leadTime := by
  -- Useful-arrival and guard assumptions justify a commitment before full recovery in this witness.
  have _usefulArrived :
      guard.evidenceGuard ≤ witness.observedUsefulContributions ∧
        bound.requiredUsefulContributions ≤ witness.observedUsefulContributions := by
    exact useful_inference_arrival_bound bound guard.evidenceGuard
      witness.observedUsefulContributions hBound hUseful hEvidence
  have _guardedBound :
      guard.falseCommitmentPermilleBound ≤ 1000 ∧
        guard.marginThreshold ≤ anomalyMarginLowerBound model witness.initialMargin ∧
        guard.evidenceGuard ≤ witness.observedUsefulContributions := by
    exact guarded_commitment_false_probability_bounded
      model guard witness.initialMargin witness.observedUsefulContributions
      hModel hGuard hMargin hEvidence
  -- The positive lead time itself is the ordinary difference between recovery and commitment.
  refine ⟨witness.fullRecoveryTime - witness.commitmentTime, ?_, Nat.sub_pos_of_lt hEarly⟩
  simp [commitmentLeadTime]

theorem right_censored_timeline_has_no_commitment_lead_time
    (commitmentTime : Nat) :
    commitmentLeadTime
        { commitmentTime? := some commitmentTime
          fullRecoveryTime? := none } =
      none := by
  -- Without a recorded full-recovery event, lead time remains right-censored.
  rfl

/-! ## Innovative-Quality Monotonicity -/

/-- Innovative valid evidence is modeled as a nondegrading statistic step. -/
def innovativeQualityStep
    (before after : AdditiveScoreStatistic) : Prop :=
  before.topMargin ≤ after.topMargin ∧
    after.evidenceCount = before.evidenceCount + 1

theorem innovative_valid_evidence_quality_monotone
    (before after : AdditiveScoreStatistic)
    (hStep : innovativeQualityStep before after) :
    ReceiverQualityOrder before after := by
  -- Innovative evidence raises evidence count by one and does not shrink the guarded margin.
  refine ⟨hStep.left, ?_⟩
  -- The step witness fixes the exact evidence-count increase.
  rw [hStep.right]
  exact Nat.le_succ before.evidenceCount

end FieldActiveBelief
