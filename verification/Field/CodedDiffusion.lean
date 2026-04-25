/-
The Problem. The coded-diffusion proof stack needs a compact proof-facing model for coded evidence
that matches the Rust reconstruction surface: independent contribution ids,
duplicate suppression, k-of-n reconstruction, recoding ledgers, observer
projection, and finite work accounting. The active theorem object is evidence
rank, not Field corridor routing or router-owned route truth.

Solution Structure.
1. Define evidence origins, contribution ids, coding windows, receiver rank,
   reconstruction quorums, and contribution-ledger records.
2. Prove duplicate non-inflation, innovative rank growth, reconstruction
   monotonicity, recoding soundness, and observer projection preservation.
3. Provide deterministic potential and finite-work recurrence lemmas, with
   explicit boundaries for probability-heavy inference results.
-/

/-! # Coded Diffusion — active theorem core -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldCodedDiffusion

/-! ## Identifiers And Evidence Origin -/

abbrev EvidenceId := Nat
abbrev ContributionId := Nat
abbrev LocalObservationId := Nat

inductive EvidenceOriginMode where
  | sourceCoded
  | locallyGenerated
  | recodedAggregated
  deriving Inhabited, Repr, DecidableEq, BEq

inductive ContributionLedgerKind where
  | sourceCodedRank
  | localObservation
  | parentLedgerUnion
  | aggregateWithLocalObservation
  deriving Inhabited, Repr, DecidableEq, BEq

/-! ## Reconstruction Vocabulary -/

structure CodingWindow where
  k : Nat
  n : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def CodingWindow.valid (window : CodingWindow) : Prop :=
  0 < window.k ∧ window.k ≤ window.n

theorem coding_window_valid_k_pos
    (window : CodingWindow)
    (hValid : window.valid) :
    0 < window.k := by
  exact hValid.left

theorem coding_window_valid_k_le_n
    (window : CodingWindow)
    (hValid : window.valid) :
    window.k ≤ window.n := by
  exact hValid.right

structure ReceiverRank where
  contributionIds : List ContributionId
  innovativeArrivals : Nat
  duplicateArrivals : Nat
  reconstructedAt? : Option Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def receiverRank (rank : ReceiverRank) : Nat :=
  rank.contributionIds.length

def reconstructable (window : CodingWindow) (rank : ReceiverRank) : Prop :=
  window.k ≤ receiverRank rank

structure ReconstructionQuorum where
  contributionIds : List ContributionId
  deriving Inhabited, Repr, DecidableEq, BEq

def validReconstructionQuorum
    (window : CodingWindow)
    (quorum : ReconstructionQuorum) : Prop :=
  window.k ≤ quorum.contributionIds.length

theorem k_of_n_reconstruction
    (window : CodingWindow)
    (rank : ReceiverRank)
    (hRank : window.k ≤ receiverRank rank) :
    reconstructable window rank := by
  exact hRank

theorem valid_quorum_implies_reconstruction
    (window : CodingWindow)
    (quorum : ReconstructionQuorum)
    (hQuorum : validReconstructionQuorum window quorum) :
    window.k ≤ quorum.contributionIds.length := by
  exact hQuorum

/-! ## Duplicate Non-Inflation -/

def duplicateArrival (rank : ReceiverRank) : ReceiverRank :=
  { rank with duplicateArrivals := rank.duplicateArrivals + 1 }

def innovativeArrivalWith
    (contributionId : ContributionId)
    (rank : ReceiverRank) : ReceiverRank :=
  { rank with
    contributionIds := rank.contributionIds ++ [contributionId]
    innovativeArrivals := rank.innovativeArrivals + 1 }

def acceptContribution
    (contributionId : ContributionId)
    (rank : ReceiverRank) : ReceiverRank :=
  if contributionId ∈ rank.contributionIds then
    duplicateArrival rank
  else
    innovativeArrivalWith contributionId rank

theorem duplicate_non_inflation (rank : ReceiverRank) :
    receiverRank (duplicateArrival rank) = receiverRank rank := by
  rfl

theorem innovative_arrival_increases_rank_by_one
    (rank : ReceiverRank)
    (contributionId : ContributionId) :
    receiverRank (innovativeArrivalWith contributionId rank) =
      receiverRank rank + 1 := by
  simp [receiverRank, innovativeArrivalWith]

theorem innovative_evidence_increases_rank_exactly_when_new
    (rank : ReceiverRank)
    (contributionId : ContributionId)
    (hNew : contributionId ∉ rank.contributionIds) :
    receiverRank (acceptContribution contributionId rank) =
      receiverRank rank + 1 := by
  -- A fresh contribution takes the innovative branch, which appends one id.
  simp [acceptContribution, hNew, innovative_arrival_increases_rank_by_one]

theorem duplicate_evidence_preserves_rank_when_present
    (rank : ReceiverRank)
    (contributionId : ContributionId)
    (hPresent : contributionId ∈ rank.contributionIds) :
    receiverRank (acceptContribution contributionId rank) = receiverRank rank := by
  -- An already-counted contribution takes the duplicate branch and leaves ids unchanged.
  simp [acceptContribution, hPresent, duplicate_non_inflation]

theorem reconstruction_monotonicity_innovative
    (window : CodingWindow)
    (rank : ReceiverRank)
    (contributionId : ContributionId)
    (hRec : reconstructable window rank) :
    reconstructable window (innovativeArrivalWith contributionId rank) := by
  -- Existing reconstruction survives because appending one contribution only grows rank.
  exact Nat.le_trans hRec (by simp [receiverRank, innovativeArrivalWith])

/-! ## Contribution-Ledger And Recoding Soundness -/

structure ContributionLedgerRecord where
  evidenceId : EvidenceId
  contributionId : ContributionId
  kind : ContributionLedgerKind
  parentContributionIds : List ContributionId
  hasLocalObservation : Bool
  deriving Inhabited, Repr, DecidableEq, BEq

def validContributionLedger (record : ContributionLedgerRecord) : Prop :=
  match record.kind with
  | .sourceCodedRank =>
      record.parentContributionIds = [] ∧ record.hasLocalObservation = false
  | .localObservation =>
      record.parentContributionIds = [] ∧ record.hasLocalObservation = true
  | .parentLedgerUnion =>
      record.parentContributionIds ≠ [] ∧
        record.contributionId ∈ record.parentContributionIds
  | .aggregateWithLocalObservation =>
      record.parentContributionIds ≠ [] ∧ record.hasLocalObservation = true

theorem recoding_soundness_parent_contribution_ledger
    (record : ContributionLedgerRecord)
    (hValid : validContributionLedger record)
    (hKind : record.kind = ContributionLedgerKind.parentLedgerUnion) :
    record.contributionId ∈ record.parentContributionIds := by
  -- Unfolding validity for a parent-ledger union exposes parent membership directly.
  simp [validContributionLedger, hKind] at hValid
  exact hValid.right

theorem aggregate_contribution_requires_local_observation
    (record : ContributionLedgerRecord)
    (hValid : validContributionLedger record)
    (hKind : record.kind = ContributionLedgerKind.aggregateWithLocalObservation) :
    record.hasLocalObservation = true := by
  -- Aggregate validity is exactly parent support plus a local-observation witness.
  simp [validContributionLedger, hKind] at hValid
  exact hValid.right

theorem recoded_duplicate_non_inflation
    (rank : ReceiverRank)
    (record : ContributionLedgerRecord)
    (hPresent : record.contributionId ∈ rank.contributionIds) :
    receiverRank (acceptContribution record.contributionId rank) =
      receiverRank rank := by
  -- Recoding cannot make an already-counted contribution innovative.
  exact duplicate_evidence_preserves_rank_when_present rank record.contributionId hPresent

theorem source_and_local_evidence_share_rank_accounting
    (rank : ReceiverRank)
    (sourceContribution localContribution : ContributionId)
    (hSourceNew : sourceContribution ∉ rank.contributionIds)
    (hLocalNew : localContribution ∉ rank.contributionIds) :
    receiverRank (acceptContribution sourceContribution rank) =
        receiverRank rank + 1 ∧
      receiverRank (acceptContribution localContribution rank) =
        receiverRank rank + 1 := by
  -- Source-coded and local observations both enter through the same contribution gate.
  exact
    ⟨ innovative_evidence_increases_rank_exactly_when_new
        rank sourceContribution hSourceNew
    , innovative_evidence_increases_rank_exactly_when_new
        rank localContribution hLocalNew ⟩

/-! ## Observer Projection -/

structure FragmentObservation where
  observedRank : Nat
  duplicateArrivals : Nat
  custodyCount : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

structure ObserverProjection where
  observedRank : Nat
  rankDeficit : Nat
  duplicateArrivals : Nat
  custodyCount : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def observerProjection
    (window : CodingWindow)
    (observation : FragmentObservation) : ObserverProjection :=
  { observedRank := observation.observedRank
    rankDeficit := window.k - observation.observedRank
    duplicateArrivals := observation.duplicateArrivals
    custodyCount := observation.custodyCount }

theorem observer_projection_preserves_rank
    (window : CodingWindow)
    (observation : FragmentObservation) :
    (observerProjection window observation).observedRank =
      observation.observedRank := by
  rfl

theorem observer_projection_preserves_duplicate_count
    (window : CodingWindow)
    (observation : FragmentObservation) :
    (observerProjection window observation).duplicateArrivals =
      observation.duplicateArrivals := by
  rfl

theorem observer_projection_preserves_custody_count
    (window : CodingWindow)
    (observation : FragmentObservation) :
    (observerProjection window observation).custodyCount =
      observation.custodyCount := by
  rfl

/-! ## Diffusion Potential Accounting -/

structure DiffusionPotential where
  rankDeficit : Nat
  duplicatePressure : Nat
  storagePressure : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def DiffusionPotential.total (potential : DiffusionPotential) : Nat :=
  potential.rankDeficit + potential.duplicatePressure + potential.storagePressure

def innovativePotentialStep (potential : DiffusionPotential) : DiffusionPotential :=
  { potential with rankDeficit := potential.rankDeficit - 1 }

def duplicatePotentialStep (potential : DiffusionPotential) : DiffusionPotential :=
  { potential with duplicatePressure := potential.duplicatePressure + 1 }

theorem innovative_step_rank_deficit_nonincreasing
    (potential : DiffusionPotential) :
    (innovativePotentialStep potential).rankDeficit ≤ potential.rankDeficit := by
  exact Nat.sub_le potential.rankDeficit 1

theorem duplicate_step_preserves_rank_deficit
    (potential : DiffusionPotential) :
    (duplicatePotentialStep potential).rankDeficit = potential.rankDeficit := by
  rfl

theorem duplicate_step_increases_duplicate_pressure
    (potential : DiffusionPotential) :
    (duplicatePotentialStep potential).duplicatePressure =
      potential.duplicatePressure + 1 := by
  rfl

theorem potential_accounting_innovative
    (potential : DiffusionPotential) :
    (innovativePotentialStep potential).rankDeficit ≤ potential.rankDeficit ∧
      (innovativePotentialStep potential).duplicatePressure =
        potential.duplicatePressure := by
  -- Innovative steps can only reduce the rank deficit and leave duplicate pressure alone.
  exact ⟨innovative_step_rank_deficit_nonincreasing potential, rfl⟩

theorem potential_accounting_duplicate
    (potential : DiffusionPotential) :
    (duplicatePotentialStep potential).rankDeficit = potential.rankDeficit ∧
      (duplicatePotentialStep potential).duplicatePressure =
        potential.duplicatePressure + 1 := by
  -- Duplicate steps do not affect rank deficit; they account for pressure explicitly.
  exact
    ⟨ duplicate_step_preserves_rank_deficit potential
    , duplicate_step_increases_duplicate_pressure potential ⟩

/-! ## Finite Deterministic Work Recurrence -/

def finiteWork (activeOpportunities : Nat → Nat) : Nat → Nat
  | 0 => activeOpportunities 0
  | t + 1 => finiteWork activeOpportunities t + activeOpportunities (t + 1)

theorem finite_work_recurrence
    (activeOpportunities : Nat → Nat)
    (t : Nat) :
    finiteWork activeOpportunities (t + 1) =
      finiteWork activeOpportunities t + activeOpportunities (t + 1) := by
  rfl

theorem finite_work_step_monotone
    (activeOpportunities : Nat → Nat)
    (t : Nat) :
    finiteWork activeOpportunities t ≤
      finiteWork activeOpportunities (t + 1) := by
  -- Finite work at the next horizon adds a nonnegative opportunity count.
  simp [finiteWork]

structure InferencePotential where
  uncertainty : Nat
  wrongBasinMass : Nat
  duplicatePressure : Nat
  storagePressure : Nat
  transmissionPressure : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def InferencePotential.total (potential : InferencePotential) : Nat :=
  potential.uncertainty +
    potential.wrongBasinMass +
    potential.duplicatePressure +
    potential.storagePressure +
    potential.transmissionPressure

def inferenceProgressStep
    (potential : InferencePotential)
    (uncertaintyProgress : Nat) : InferencePotential :=
  { potential with uncertainty := potential.uncertainty - uncertaintyProgress }

theorem inference_progress_uncertainty_nonincreasing
    (potential : InferencePotential)
    (uncertaintyProgress : Nat) :
    (inferenceProgressStep potential uncertaintyProgress).uncertainty ≤
      potential.uncertainty := by
  -- Progress can only subtract from the uncertainty component.
  exact Nat.sub_le potential.uncertainty uncertaintyProgress

theorem inference_potential_total_is_accounted_sum
    (potential : InferencePotential) :
    potential.total =
      potential.uncertainty +
        potential.wrongBasinMass +
        potential.duplicatePressure +
        potential.storagePressure +
        potential.transmissionPressure := by
  -- The Rust rows expose these named integer terms directly.
  rfl

/-! ## Majority-Threshold Mergeable Task -/

structure MajorityThresholdState where
  positiveVotes : Nat
  negativeVotes : Nat
  contributionIds : List ContributionId
  deriving Inhabited, Repr, DecidableEq, BEq

def majorityVoteCount (state : MajorityThresholdState) : Nat :=
  state.positiveVotes + state.negativeVotes

def majorityMargin (state : MajorityThresholdState) : Nat :=
  if state.positiveVotes ≥ state.negativeVotes then
    state.positiveVotes - state.negativeVotes
  else
    state.negativeVotes - state.positiveVotes

def majorityDecisionPositive (state : MajorityThresholdState) : Prop :=
  state.negativeVotes < state.positiveVotes

def acceptMajorityContribution
    (contributionId : ContributionId)
    (positiveEvidence : Bool)
    (state : MajorityThresholdState) : MajorityThresholdState :=
  if contributionId ∈ state.contributionIds then
    state
  else if positiveEvidence then
    { state with
      positiveVotes := state.positiveVotes + 1
      contributionIds := state.contributionIds ++ [contributionId] }
  else
    { state with
      negativeVotes := state.negativeVotes + 1
      contributionIds := state.contributionIds ++ [contributionId] }

theorem majority_duplicate_non_inflation
    (state : MajorityThresholdState)
    (contributionId : ContributionId)
    (positiveEvidence : Bool)
    (hPresent : contributionId ∈ state.contributionIds) :
    majorityVoteCount
        (acceptMajorityContribution contributionId positiveEvidence state) =
      majorityVoteCount state := by
  -- Already-counted contributions leave the majority statistic unchanged.
  simp [acceptMajorityContribution, hPresent, majorityVoteCount]

theorem majority_positive_innovative_increases_vote_count
    (state : MajorityThresholdState)
    (contributionId : ContributionId)
    (hNew : contributionId ∉ state.contributionIds) :
    majorityVoteCount
        (acceptMajorityContribution contributionId true state) =
      majorityVoteCount state + 1 := by
  -- A fresh positive contribution appends one ledger id and adds one vote.
  simp [
    acceptMajorityContribution,
    hNew,
    majorityVoteCount,
    Nat.add_assoc,
    Nat.add_left_comm,
    Nat.add_comm
  ]

end FieldCodedDiffusion
