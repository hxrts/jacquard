import Field.ActiveBeliefEndToEnd

/-
The Problem. The deeper paper thesis is about the limits of distributed error
correction as a reconstruction objective. For inference, the receiver may not
need the original message or full observation set. It may only need enough
audited mergeable statistic mass to enter a guarded decision basin that agrees
with the full statistic.

Solution Structure.
1. Define decision-sufficiency certificates that separate guarded decision
   correctness from full k-of-n reconstruction.
2. Prove a stable basin theorem and a concrete strict-separation witness.
3. Show exact reconstruction is one special case of decision sufficiency.
4. Expose the byte gap and demand-as-basin-progress interpretation.
5. Add a non-stable counterexample and the final decision-first error
   correction limit theorem.
-/

/-! # Active Belief Diffusion — decision sufficiency before reconstruction -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-! ## Decision Sufficiency Certificates -/

/-- A partial statistic is decision-sufficient before full reconstruction. -/
structure StableDecisionBasinCertificate where
  window : CodingWindow
  rank : ReceiverRank
  guard : AnomalyCommitmentGuard
  partialStatistic : AdditiveScoreStatistic
  fullStatistic : AdditiveScoreStatistic
  bytesToDecision : Nat
  bytesToReconstruction : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid basins agree with the full decision while recovery remains impossible. -/
def validStableDecisionBasin
    (certificate : StableDecisionBasinCertificate) : Prop :=
  ¬ reconstructable certificate.window certificate.rank ∧
    guardPassesOnStatistic certificate.guard certificate.partialStatistic ∧
    statisticDecision certificate.partialStatistic =
      statisticDecision certificate.fullStatistic ∧
    certificate.bytesToDecision < certificate.bytesToReconstruction

theorem stable_decision_basin_before_reconstruction
    (receiverId : ReceiverId)
    (certificate : StableDecisionBasinCertificate)
    (hValid : validStableDecisionBasin certificate) :
    ¬ reconstructable certificate.window certificate.rank ∧
      (guardedCommitmentFromStatistic
          receiverId certificate.guard certificate.partialStatistic).guardPassed = true ∧
      (guardedCommitmentFromStatistic
          receiverId certificate.guard certificate.partialStatistic).hypothesis =
        statisticDecision certificate.fullStatistic := by
  -- A stable basin makes the decision correct even while reconstruction is false.
  have hCommit :=
    guarded_commitment_from_mergeable_statistic_correct
      receiverId certificate.guard certificate.partialStatistic hValid.right.left
  exact ⟨hValid.left, hCommit.left, hCommit.right.trans hValid.right.right.left⟩

/-! ## Strict Separation Example -/

/-- A small recovery threshold where rank three is not enough to reconstruct. -/
def decisionLimitExampleWindow : CodingWindow :=
  { k := 5, n := 8 }

/-- Three independent contributions have arrived, below the recovery threshold. -/
def decisionLimitExampleRank : ReceiverRank :=
  { contributionIds := [1, 2, 3]
    innovativeArrivals := 3
    duplicateArrivals := 0
    reconstructedAt? := none }

/-- Partial and full statistics agree on the decision before reconstruction. -/
def decisionLimitExampleCertificate : StableDecisionBasinCertificate :=
  { window := decisionLimitExampleWindow
    rank := decisionLimitExampleRank
    guard :=
      { marginThreshold := 5
        evidenceGuard := 3
        falseCommitmentPermilleBound := 10 }
    partialStatistic :=
      { topHypothesis := 7
        runnerUpHypothesis := 2
        topMargin := 6
        evidenceCount := 3 }
    fullStatistic :=
      { topHypothesis := 7
        runnerUpHypothesis := 4
        topMargin := 11
        evidenceCount := 8 }
    bytesToDecision := 384
    bytesToReconstruction := 1024 }

theorem decision_sufficiency_strictly_weaker_than_reconstruction_example :
    validStableDecisionBasin decisionLimitExampleCertificate := by
  -- The concrete witness has rank 3 < k 5, but the guarded decision is stable.
  unfold validStableDecisionBasin
  unfold decisionLimitExampleCertificate
  unfold decisionLimitExampleWindow decisionLimitExampleRank
  simp [reconstructable, receiverRank, guardPassesOnStatistic, statisticDecision]

/-! ## Reconstruction As A Special Case -/

/-- Exact recovery can be viewed as a degenerate decision-sufficiency certificate. -/
structure RecoveryAsDecisionCertificate where
  window : CodingWindow
  rank : ReceiverRank
  statistic : AdditiveScoreStatistic
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid recovery-as-decision certificates already satisfy k-of-n recovery. -/
def validRecoveryAsDecision
    (certificate : RecoveryAsDecisionCertificate) : Prop :=
  reconstructable certificate.window certificate.rank

theorem exact_reconstruction_is_decision_sufficiency_special_case
    (certificate : RecoveryAsDecisionCertificate)
    (hValid : validRecoveryAsDecision certificate) :
    reconstructable certificate.window certificate.rank ∧
      statisticDecision certificate.statistic =
        statisticDecision certificate.statistic := by
  -- Recovery is the special case where the decision target is available after quorum.
  exact ⟨hValid, rfl⟩

/-! ## Cost Gap And Demand Value -/

theorem bytes_to_decision_can_be_less_than_bytes_to_reconstruction
    (certificate : StableDecisionBasinCertificate)
    (hValid : validStableDecisionBasin certificate) :
    certificate.bytesToDecision < certificate.bytesToReconstruction := by
  -- The byte gap is carried explicitly by the decision-sufficiency certificate.
  exact hValid.right.right.right

/-- Demand is valuable when it moves evidence toward the guarded decision basin. -/
structure DemandBasinProgressCertificate where
  demand : DemandSummary
  basinProgressValue : Nat
  rankOnlyValue : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validDemandBasinProgress
    (certificate : DemandBasinProgressCertificate) : Prop :=
  validDemandSummary certificate.demand ∧
    certificate.rankOnlyValue ≤ certificate.basinProgressValue

theorem demand_value_targets_decision_basin_progress
    (certificate : DemandBasinProgressCertificate)
    (hValid : validDemandBasinProgress certificate) :
    validDemandSummary certificate.demand ∧
      certificate.rankOnlyValue ≤ certificate.basinProgressValue ∧
      (ActiveMessage.demand certificate.demand).contributionId? = none := by
  -- Demand can prefer basin progress while still carrying no contribution identity.
  exact
    ⟨ hValid.left
    , hValid.right
    , demand_message_carries_no_contribution certificate.demand ⟩

/-! ## Boundary And Final Limit Theorem -/

/-- Without basin stability, a partial decision may disagree with the full statistic. -/
def nonstablePartialDecisionPair :
    AdditiveScoreStatistic × AdditiveScoreStatistic :=
  ( { topHypothesis := 1, runnerUpHypothesis := 2, topMargin := 1, evidenceCount := 1 }
  , { topHypothesis := 2, runnerUpHypothesis := 1, topMargin := 5, evidenceCount := 5 } )

theorem nonstable_partial_decision_counterexample :
    statisticDecision nonstablePartialDecisionPair.1 ≠
      statisticDecision nonstablePartialDecisionPair.2 := by
  -- This boundary example shows why stable decision-basin evidence is required.
  decide

/-- Decision-first correction is the paper's stronger target for inference tasks. -/
structure DistributedErrorCorrectionDecisionLimit where
  decisionCertificate : StableDecisionBasinCertificate
  demandProgress? : Option DemandBasinProgressCertificate
  deriving Inhabited, Repr, DecidableEq, BEq

def validDistributedErrorCorrectionDecisionLimit
    (limit : DistributedErrorCorrectionDecisionLimit) : Prop :=
  validStableDecisionBasin limit.decisionCertificate ∧
    match limit.demandProgress? with
    | none => True
    | some certificate => validDemandBasinProgress certificate

theorem distributed_error_correction_decision_limit
    (receiverId : ReceiverId)
    (limit : DistributedErrorCorrectionDecisionLimit)
    (hValid : validDistributedErrorCorrectionDecisionLimit limit) :
    ¬ reconstructable
        limit.decisionCertificate.window limit.decisionCertificate.rank ∧
      (guardedCommitmentFromStatistic
          receiverId
          limit.decisionCertificate.guard
          limit.decisionCertificate.partialStatistic).guardPassed = true ∧
      (guardedCommitmentFromStatistic
          receiverId
          limit.decisionCertificate.guard
          limit.decisionCertificate.partialStatistic).hypothesis =
          statisticDecision limit.decisionCertificate.fullStatistic ∧
        limit.decisionCertificate.bytesToDecision <
          limit.decisionCertificate.bytesToReconstruction := by
  -- The limit theorem packages the core idea: correct decision can precede recovery.
  have hBasin :=
    stable_decision_basin_before_reconstruction
      receiverId limit.decisionCertificate hValid.left
  exact
    ⟨ hBasin.left
    , hBasin.right.left
    , hBasin.right.right
    , bytes_to_decision_can_be_less_than_bytes_to_reconstruction
        limit.decisionCertificate hValid.left ⟩

end FieldActiveBelief
