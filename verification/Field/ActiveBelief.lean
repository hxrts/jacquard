import Field.CodedDiffusion

/-
The Problem. Active belief diffusion adds receiver-indexed belief landscapes
and first-class bounded demand summaries exchanged alongside coded evidence.
Evidence and demand are symmetric as replay-visible messages, but not in
semantics: evidence carries audited contributions, while demand describes what
would most reduce uncertainty. Demand must steer forwarding, custody, and
allocation without becoming evidence, hidden global state, or a way to inflate
rank.

Solution Structure.
1. Define proof-facing receiver, evidence-message, demand-message, and
   commitment objects.
2. Prove that demand is a bounded exchanged message but cannot validate
   evidence or carry contribution identity.
3. Prove duplicate non-inflation under demand-driven forwarding.
4. State multi-receiver compatibility as guarded local decisions, not
   consensus or identical beliefs.
-/

/-! # Active Belief Diffusion — demand soundness core -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-! ## Receiver-Indexed Belief State -/

abbrev ReceiverId := Nat
abbrev HypothesisId := Nat
abbrev DemandEntryId := Nat

/-- A receiver-local quality summary used to derive bounded demand. -/
structure QualitySummary where
  uncertainty : Nat
  topMargin : Nat
  evidenceCount : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Receiver-indexed audited state: rank plus quality, not route truth. -/
structure ReceiverBeliefState where
  receiverId : ReceiverId
  rank : ReceiverRank
  quality : QualitySummary
  topHypothesis? : Option HypothesisId
  deriving Inhabited, Repr, DecidableEq, BEq

/-- A guarded local decision emitted from an audited receiver statistic. -/
structure GuardedCommitment where
  receiverId : ReceiverId
  hypothesis : HypothesisId
  guardPassed : Bool
  deriving Inhabited, Repr, DecidableEq, BEq

/-! ## Bounded Demand Summaries -/

/-- One bounded request for evidence that may improve a receiver landscape. -/
structure DemandEntry where
  entryId : DemandEntryId
  hypothesis : HypothesisId
  requestedContribution? : Option ContributionId
  priority : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Demand is advisory control data with explicit size and lifetime caps. -/
structure DemandSummary where
  receiverId : ReceiverId
  entries : List DemandEntry
  entryCap : Nat
  byteCap : Nat
  ttl : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid demand is bounded and live; it still does not count as evidence. -/
def validDemandSummary (summary : DemandSummary) : Prop :=
  summary.entries.length ≤ summary.entryCap ∧
    summary.entries.length ≤ summary.byteCap ∧
    0 < summary.ttl

/-- Expired demand is ignored by policy and never justifies evidence acceptance. -/
def expiredDemandSummary (summary : DemandSummary) : Prop :=
  summary.ttl = 0

theorem demand_bounded_by_entry_cap
    (summary : DemandSummary)
    (hValid : validDemandSummary summary) :
    summary.entries.length ≤ summary.entryCap := by
  -- Valid demand exposes the entry cap as its first boundedness obligation.
  exact hValid.left

theorem demand_bounded_by_byte_cap
    (summary : DemandSummary)
    (hValid : validDemandSummary summary) :
    summary.entries.length ≤ summary.byteCap := by
  -- The byte cap is modeled as a proof-facing entry budget.
  exact hValid.right.left

theorem valid_demand_is_live
    (summary : DemandSummary)
    (hValid : validDemandSummary summary) :
    0 < summary.ttl := by
  -- Live demand carries a positive time-to-live.
  exact hValid.right.right

/-- Deterministic proof-facing demand derived from audited receiver state. -/
def demandSummaryFromReceiverState
    (state : ReceiverBeliefState)
    (entryCap byteCap ttl : Nat) : DemandSummary :=
  { receiverId := state.receiverId
    entries :=
      [ { entryId := state.receiverId
          hypothesis := state.topHypothesis?.getD 0
          requestedContribution? := none
          priority := state.quality.uncertainty + state.quality.topMargin } ]
    entryCap := entryCap
    byteCap := byteCap
    ttl := ttl }

theorem demand_summary_from_receiver_state_valid
    (state : ReceiverBeliefState)
    (entryCap byteCap ttl : Nat)
    (hEntryCap : 1 ≤ entryCap)
    (hByteCap : 1 ≤ byteCap)
    (hTtl : 0 < ttl) :
    validDemandSummary
      (demandSummaryFromReceiverState state entryCap byteCap ttl) := by
  -- Receiver-local demand is a deterministic bounded function of audited state.
  simp [validDemandSummary, demandSummaryFromReceiverState, hEntryCap, hByteCap, hTtl]

theorem demand_summary_from_receiver_state_has_canonical_singleton_order
    (state : ReceiverBeliefState)
    (entryCap byteCap ttl : Nat) :
    (demandSummaryFromReceiverState state entryCap byteCap ttl).entries =
      [ { entryId := state.receiverId
          hypothesis := state.topHypothesis?.getD 0
          requestedContribution? := none
          priority := state.quality.uncertainty + state.quality.topMargin } ] := by
  -- The proof-facing constructor emits exactly one canonically positioned entry.
  rfl

/-! ## First-Class Active Messages -/

/-- Evidence proposed to a receiver under a demand-aware policy. -/
structure EvidenceProposal where
  contributionId : ContributionId
  validEvidence : Bool
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Active belief diffusion exchanges evidence messages and demand messages. -/
inductive ActiveMessage where
  | evidence (proposal : EvidenceProposal)
  | demand (summary : DemandSummary)
  deriving Inhabited, Repr, DecidableEq

/-- Only evidence messages carry audited contribution identity. -/
def ActiveMessage.contributionId? : ActiveMessage → Option ContributionId
  | .evidence proposal => some proposal.contributionId
  | .demand _summary => none

theorem demand_message_carries_no_contribution
    (summary : DemandSummary) :
    (ActiveMessage.demand summary).contributionId? = none := by
  -- Demand is first-class in communication, but non-evidential in semantics.
  rfl

theorem evidence_message_carries_contribution
    (proposal : EvidenceProposal) :
    (ActiveMessage.evidence proposal).contributionId? =
      some proposal.contributionId := by
  -- Evidence messages expose the contribution id checked by the receiver.
  rfl

/-! ## Demand-Aware Evidence Acceptance -/

/-- Only evidence validity gates acceptance; demand is intentionally absent. -/
def evidenceAcceptancePermitted (proposal : EvidenceProposal) : Prop :=
  proposal.validEvidence = true

/-- Demand can affect priority, but invalid evidence leaves rank unchanged. -/
def demandAwareAccept
    (_summary : DemandSummary)
    (proposal : EvidenceProposal)
    (rank : ReceiverRank) : ReceiverRank :=
  if proposal.validEvidence then
    acceptContribution proposal.contributionId rank
  else
    rank

theorem demand_cannot_validate_invalid_evidence
    (summary : DemandSummary)
    (proposal : EvidenceProposal)
    (rank : ReceiverRank)
    (hInvalid : proposal.validEvidence = false) :
    demandAwareAccept summary proposal rank = rank := by
  -- Invalid evidence is rejected regardless of demand contents or priority.
  simp [demandAwareAccept, hInvalid]

theorem demand_accepts_only_through_valid_evidence
    (summary : DemandSummary)
    (proposal : EvidenceProposal)
    (rank : ReceiverRank)
    (hValid : proposal.validEvidence = true) :
    demandAwareAccept summary proposal rank =
      acceptContribution proposal.contributionId rank := by
  -- Valid evidence follows the ordinary contribution-ledger acceptance gate.
  simp [demandAwareAccept, hValid]

theorem demand_duplicate_non_inflation
    (summary : DemandSummary)
    (proposal : EvidenceProposal)
    (rank : ReceiverRank)
    (hValid : proposal.validEvidence = true)
    (hPresent : proposal.contributionId ∈ rank.contributionIds) :
    receiverRank (demandAwareAccept summary proposal rank) =
      receiverRank rank := by
  -- Demand may route duplicate evidence, but the receiver counts it once.
  rw [demand_accepts_only_through_valid_evidence summary proposal rank hValid]
  exact duplicate_evidence_preserves_rank_when_present
    rank proposal.contributionId hPresent

theorem expired_demand_does_not_accept_invalid_evidence
    (summary : DemandSummary)
    (proposal : EvidenceProposal)
    (rank : ReceiverRank)
    (_hExpired : expiredDemandSummary summary)
    (hInvalid : proposal.validEvidence = false) :
    demandAwareAccept summary proposal rank = rank := by
  -- Expiration is policy metadata; invalid evidence is still rejected directly.
  exact demand_cannot_validate_invalid_evidence summary proposal rank hInvalid

/-! ## Commitment Lead Time -/

/-- Logged commitment and full-recovery times for one receiver. -/
structure CommitmentTimeline where
  commitmentTime? : Option Nat
  fullRecoveryTime? : Option Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Positive values mean commitment happened before full recovery. -/
def commitmentLeadTime (timeline : CommitmentTimeline) : Option Nat :=
  match timeline.commitmentTime?, timeline.fullRecoveryTime? with
  | some commitmentTime, some fullRecoveryTime =>
      some (fullRecoveryTime - commitmentTime)
  | _, _ => none

theorem commitment_lead_time_soundness
    (commitmentTime fullRecoveryTime : Nat) :
    commitmentLeadTime
        { commitmentTime? := some commitmentTime
          fullRecoveryTime? := some fullRecoveryTime } =
      some (fullRecoveryTime - commitmentTime) := by
  -- Lead time is derived only from logged commitment and recovery events.
  rfl

/-! ## Multi-Receiver Compatibility -/

/-- Compatibility is agreement on the guarded hypothesis, not consensus. -/
def compatibleCommitments
    (left right : GuardedCommitment) : Prop :=
  left.guardPassed = true ∧
    right.guardPassed = true ∧
    left.hypothesis = right.hypothesis

theorem same_guarded_basin_compatible
    (left right : GuardedCommitment)
    (hLeft : left.guardPassed = true)
    (hRight : right.guardPassed = true)
    (hSame : left.hypothesis = right.hypothesis) :
    compatibleCommitments left right := by
  -- Compatibility records same guarded basin; it does not assert identical state.
  exact ⟨hLeft, hRight, hSame⟩

theorem compatible_commitments_have_same_hypothesis
    (left right : GuardedCommitment)
    (hCompatible : compatibleCommitments left right) :
    left.hypothesis = right.hypothesis := by
  -- The third component of compatibility is the shared committed hypothesis.
  exact hCompatible.right.right

/-! ## Demand Is Control, Not Evidence -/

/-- A policy score can use first-class demand, but only to rank candidates. -/
def demandPriorityScore
    (summary : DemandSummary)
    (entry : DemandEntry) : Nat :=
  summary.ttl + entry.priority

theorem demand_priority_does_not_change_acceptance
    (summary : DemandSummary)
    (entry : DemandEntry)
    (proposal : EvidenceProposal)
    (rank : ReceiverRank) :
    demandAwareAccept summary proposal rank =
      demandAwareAccept
        { summary with
          entries := entry :: summary.entries
          ttl := demandPriorityScore summary entry }
        proposal
        rank := by
  -- Acceptance ignores demand contents; changing priority metadata is harmless.
  cases proposal.validEvidence <;> rfl

/-! ## Propagated Host/Bridge Demand Soundness -/

inductive ActiveDemandExecutionSurface where
  | simulatorLocal
  | hostBridgeReplay
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Replay-visible host/bridge demand carries custody metadata, not evidence. -/
structure PropagatedDemandRecord where
  surface : ActiveDemandExecutionSurface
  summary : DemandSummary
  bridgeBatchId : Nat
  ingressRound : Nat
  replayVisible : Bool
  deriving Inhabited, Repr, DecidableEq, BEq

def validPropagatedDemandRecord
    (record : PropagatedDemandRecord) : Prop :=
  record.surface = ActiveDemandExecutionSurface.hostBridgeReplay ∧
    validDemandSummary record.summary ∧
    record.replayVisible = true

theorem propagated_demand_is_replay_visible
    (record : PropagatedDemandRecord)
    (hValid : validPropagatedDemandRecord record) :
    record.replayVisible = true := by
  -- Host/bridge demand must be visible to replay to remain auditable.
  exact hValid.right.right

theorem propagated_demand_uses_host_bridge_surface
    (record : PropagatedDemandRecord)
    (hValid : validPropagatedDemandRecord record) :
    record.surface = ActiveDemandExecutionSurface.hostBridgeReplay := by
  -- The replay surface distinguishes host/bridge demand from simulator-local demand.
  exact hValid.left

theorem propagated_demand_carries_no_contribution
    (record : PropagatedDemandRecord) :
    (ActiveMessage.demand record.summary).contributionId? = none := by
  -- Propagation changes custody and replay metadata, not demand semantics.
  exact demand_message_carries_no_contribution record.summary

theorem propagated_demand_cannot_validate_invalid_evidence
    (record : PropagatedDemandRecord)
    (proposal : EvidenceProposal)
    (rank : ReceiverRank)
    (hValidRecord : validPropagatedDemandRecord record)
    (hInvalid : proposal.validEvidence = false) :
    demandAwareAccept record.summary proposal rank = rank := by
  -- Even on the host/bridge path, invalid evidence is rejected by evidence validity.
  have _live : 0 < record.summary.ttl :=
    valid_demand_is_live record.summary hValidRecord.right.left
  exact demand_cannot_validate_invalid_evidence
    record.summary proposal rank hInvalid

theorem propagated_demand_duplicate_non_inflation
    (record : PropagatedDemandRecord)
    (proposal : EvidenceProposal)
    (rank : ReceiverRank)
    (hValidRecord : validPropagatedDemandRecord record)
    (hValidEvidence : proposal.validEvidence = true)
    (hPresent : proposal.contributionId ∈ rank.contributionIds) :
    receiverRank (demandAwareAccept record.summary proposal rank) =
      receiverRank rank := by
  -- Propagated demand cannot turn an already-seen contribution into new rank.
  have _visible : record.replayVisible = true :=
    propagated_demand_is_replay_visible record hValidRecord
  exact demand_duplicate_non_inflation
    record.summary proposal rank hValidEvidence hPresent

end FieldActiveBelief
