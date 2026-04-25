import Field.ActiveBeliefCertificates

/-
The Problem. The paper's central claim is end-to-end: controlled temporal
diffusion of evidence and demand should produce receiver-side inference before
full information transit. Earlier theorem packs prove the local pieces. This
file connects those pieces over a reduced finite trace so the core thesis has a
single proof-facing theorem surface.

Solution Structure.
1. Define a finite active-belief trace whose operational state is obtained by
   folding replay-visible evidence events.
2. Prove trace soundness: the final receiver state is exactly that fold, demand
   remains non-evidential, and any commitment is decoded from the audited
   merged statistic.
3. Prove active demand improves under a value model that orders passive
   forwarding value below demand-guided useful value under equal budget.
4. Prove Rust replay validator adequacy for the narrow theorem-profile and
   trace-certificate metadata consumed by the Lean theorem surfaces.
-/

/-! # Active Belief Diffusion — end-to-end theorem surface -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-! ## Finite Trace Semantics -/

/-- Receiver-local proof state carried by the reduced finite trace semantics. -/
structure ActiveBeliefTraceState where
  rank : ReceiverRank
  statistic : AdditiveScoreStatistic
  deriving Inhabited, Repr, DecidableEq, BEq

/-- One replay-visible evidence event and its ordinary accepted contribution. -/
structure ActiveBeliefTraceEvent where
  round : Nat
  summary : DemandSummary
  proposal : EvidenceProposal
  accepted : AcceptedStatisticContribution
  deriving Inhabited, Repr, DecidableEq, BEq

/-- A trace step updates rank and statistic through the ordinary evidence gate. -/
def activeBeliefTraceStep
    (state : ActiveBeliefTraceState)
    (event : ActiveBeliefTraceEvent) :
    ActiveBeliefTraceState :=
  { rank := demandAwareAccept event.summary event.proposal state.rank
    statistic :=
      demandGuidedStatisticAccept
        event.summary event.proposal event.accepted state.rank state.statistic }

/-- Folded receiver state after replaying all finite trace events. -/
def activeBeliefTraceFinalState
    (initial : ActiveBeliefTraceState)
    (events : List ActiveBeliefTraceEvent) :
    ActiveBeliefTraceState :=
  events.foldl activeBeliefTraceStep initial

/-- Reduced finite trace for the paper's active-belief core claim. -/
structure ActiveBeliefFiniteTrace where
  receiverId : ReceiverId
  initial : ActiveBeliefTraceState
  events : List ActiveBeliefTraceEvent
  guard : AnomalyCommitmentGuard
  noStaticPathInCoreWindow : Bool
  commitmentBeforeFullRecovery : Bool
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid traces expose the path-free and early-commitment checks explicitly. -/
def validActiveBeliefFiniteTrace
    (trace : ActiveBeliefFiniteTrace) : Prop :=
  trace.noStaticPathInCoreWindow = true ∧
    trace.commitmentBeforeFullRecovery = true ∧
    guardPassesOnStatistic trace.guard
      (activeBeliefTraceFinalState trace.initial trace.events).statistic

theorem active_belief_trace_step_matches_plain_acceptance
    (state : ActiveBeliefTraceState)
    (event : ActiveBeliefTraceEvent) :
    (activeBeliefTraceStep state event).statistic =
      plainStatisticAccept
        event.proposal event.accepted state.rank state.statistic := by
  -- Demand-guided trace execution uses the same accepted-statistic reducer.
  simp [activeBeliefTraceStep,
    demand_guided_statistic_acceptance_matches_plain_acceptance]

theorem active_belief_trace_soundness
    (trace : ActiveBeliefFiniteTrace)
    (hValid : validActiveBeliefFiniteTrace trace) :
    let finalState := activeBeliefTraceFinalState trace.initial trace.events
    let commitment :=
      guardedCommitmentFromStatistic trace.receiverId trace.guard finalState.statistic
    finalState = activeBeliefTraceFinalState trace.initial trace.events ∧
      commitment.guardPassed = true ∧
      commitment.hypothesis = statisticDecision finalState.statistic ∧
      trace.noStaticPathInCoreWindow = true ∧
      trace.commitmentBeforeFullRecovery = true := by
  -- The final receiver state is the trace fold, and commitment decodes from it.
  intro finalState commitment
  have hCommit :=
    guarded_commitment_from_mergeable_statistic_correct
      trace.receiverId trace.guard finalState.statistic hValid.right.right
  exact ⟨rfl, hCommit.left, hCommit.right, hValid.left, hValid.right.left⟩

/-! ## Active Demand Value Model -/

/-- Value-order model comparing passive forwarding with demand-guided forwarding. -/
structure ActiveDemandPolicyValueModel where
  passiveUsefulArrivals : Nat
  activeUsefulArrivals : Nat
  passiveSelectedValue : Nat
  activeDemandValue : Nat
  passiveUncertainty : Nat
  activeUncertainty : Nat
  byteBudget : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid value models order passive value below active useful demand value. -/
def validActiveDemandPolicyValueModel
    (model : ActiveDemandPolicyValueModel) : Prop :=
  model.passiveUsefulArrivals ≤ model.passiveSelectedValue ∧
    model.passiveSelectedValue ≤ model.activeDemandValue ∧
    model.activeDemandValue ≤ model.activeUsefulArrivals ∧
    model.activeUncertainty ≤ model.passiveUncertainty ∧
    0 < model.byteBudget

/-- Same-budget comparison induced by a value-order model. -/
def demandComparisonOfValueModel
    (model : ActiveDemandPolicyValueModel) :
    DemandGuidedComparison :=
  { horizon := 0
    byteBudget := model.byteBudget
    passiveUsefulArrivals := model.passiveUsefulArrivals
    activeUsefulArrivals := model.activeUsefulArrivals
    passiveUncertainty := model.passiveUncertainty
    activeUncertainty := model.activeUncertainty
    passiveCommitmentTime? := none
    activeCommitmentTime? := none }

theorem active_demand_policy_improves_under_value_model
    (model : ActiveDemandPolicyValueModel)
    (hValid : validActiveDemandPolicyValueModel model) :
    demandQualityNonWorse (demandComparisonOfValueModel model) ∧
      model.passiveUsefulArrivals * model.byteBudget ≤
        model.activeUsefulArrivals * model.byteBudget := by
  -- Chaining the value order derives useful-arrival dominance under equal budget.
  have hUseful :
      model.passiveUsefulArrivals ≤ model.activeUsefulArrivals :=
    Nat.le_trans hValid.left
      (Nat.le_trans hValid.right.left hValid.right.right.left)
  have hQuality :
      demandQualityNonWorse (demandComparisonOfValueModel model) :=
    ⟨hUseful, hValid.right.right.right.left⟩
  exact
    ⟨ hQuality
    , demand_guided_quality_per_byte_nonworse
        (demandComparisonOfValueModel model) hQuality hValid.right.right.right.right ⟩

/-! ## Rust Replay Validator Adequacy -/

/-- Narrow Rust replay validator certificate for exported theorem rows. -/
structure RustReplayValidatorRow where
  canonicalPreprocessing : Bool
  deterministicReplay : Bool
  theoremProfileExported : Bool
  theoremAssumptionMarked : Bool
  traceFoldChecked : Bool
  noStaticPathChecked : Bool
  guardChecked : Bool
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Validator validity means the exported row exposes every proof-facing check. -/
def validRustReplayValidatorRow
    (row : RustReplayValidatorRow) : Prop :=
  row.canonicalPreprocessing = true ∧
    row.deterministicReplay = true ∧
    row.theoremProfileExported = true ∧
    row.theoremAssumptionMarked = true ∧
    row.traceFoldChecked = true ∧
    row.noStaticPathChecked = true ∧
    row.guardChecked = true

/-- The theorem-profile replay row induced by the validator surface. -/
def theoremProfileRowOfValidator
    (row : RustReplayValidatorRow) :
    ActiveBeliefTheoremProfileReplayRow :=
  { deterministicReplay := row.deterministicReplay
    theoremProfileExported := row.theoremProfileExported
    theoremAssumptionMarked := row.theoremAssumptionMarked
    rowSatisfiesBound := row.guardChecked }

theorem trace_validator_adequacy
    (row : RustReplayValidatorRow)
    (trace : ActiveBeliefFiniteTrace)
    (hRow : validRustReplayValidatorRow row)
    (hTrace : validActiveBeliefFiniteTrace trace) :
    soundActiveBeliefTheoremProfileReplayRow
      (theoremProfileRowOfValidator row) ∧
      row.traceFoldChecked = true ∧
      trace.noStaticPathInCoreWindow = true ∧
      trace.commitmentBeforeFullRecovery = true := by
  -- Adequacy links exported validator bits to the narrow Lean theorem inputs.
  exact
    ⟨ ⟨hRow.right.left, hRow.right.right.left,
        hRow.right.right.right.left, hRow.right.right.right.right.right.right⟩
    , hRow.right.right.right.right.left
    , hTrace.left
    , hTrace.right.left ⟩

end FieldActiveBelief
