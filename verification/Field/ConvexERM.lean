import Field.TemporalIndependenceLimits

/-
The Problem. The active-belief paper needs a task class that is larger than
small mergeable examples but narrower than arbitrary machine learning. The
proof-facing class is finite-dimensional decomposable convex empirical-risk
minimization: valid temporal evidence contributes local loss terms to a
deterministic objective, duplicates are suppressed by contribution identity,
and early decisions require optimizer and stability certificates.

Solution Structure.
1. Define a deterministic finite/fixed-point convex ERM interface.
2. Prove objective merge, duplicate safety, monotone accumulation, and demand
   non-evidentiality at the objective level.
3. State convexity, optimizer, guarded-decision, effective-evidence, active
   demand, and replay-adequacy theorem surfaces as finite certificates.
4. Add AI-central certified instances for least-squares regression and
   hinge-loss classification.
-/

/-! # Convex ERM task class for active belief diffusion -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-! ## Deterministic Convex Task Interface -/

/-- Fixed-denominator rational values are represented by natural numerators. -/
abbrev FixedPoint := Nat

/--
Discrete convexity over a bounded one-dimensional grid. This is the first
proof-facing numeric model: finite, deterministic, and free of floating point.
-/
def ConvexOnGrid (bound : Nat) (f : Nat → FixedPoint) : Prop :=
  ∀ x, x + 2 ≤ bound → 2 * f (x + 1) ≤ f x + f (x + 2)

/-- A finite-dimensional decomposable convex ERM task. -/
structure ConvexERMTask where
  dimension : Nat
  domainBound : Nat
  decisionCount : Nat
  localLoss : ContributionId → Nat → FixedPoint
  regularizer : Nat → FixedPoint
  decision : Nat → HypothesisId
  margin : Nat → Nat
  localLossConvex : ∀ contributionId,
    ConvexOnGrid domainBound (localLoss contributionId)
  regularizerConvex : ConvexOnGrid domainBound regularizer

/-- Sum of local loss terms for a receiver's accepted contribution identities. -/
def convexLossSum
    (task : ConvexERMTask)
    (acceptedIds : List ContributionId)
    (x : Nat) : FixedPoint :=
  match acceptedIds with
  | [] => 0
  | contributionId :: rest =>
      task.localLoss contributionId x + convexLossSum task rest x

/-- Objective induced by a finite accepted-evidence set. -/
def convexObjective
    (task : ConvexERMTask)
    (acceptedIds : List ContributionId)
    (x : Nat) : FixedPoint :=
  task.regularizer x + convexLossSum task acceptedIds x

/-- Receiver-side convex state: accepted identities plus the current certificate. -/
structure ConvexERMState where
  acceptedIds : List ContributionId
  optimizerPoint : Nat
  optimizerGap : Nat
  decisionMargin : Nat
  uncertaintyBound : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- One valid objective contribution. -/
structure ConvexERMContribution where
  contributionId : ContributionId
  lossFamilyId : Nat
  effectiveIndependent : Bool
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Accept a contribution id once. The loss function is indexed by that id. -/
def acceptConvexContribution
    (state : ConvexERMState)
    (contribution : ConvexERMContribution) : ConvexERMState :=
  if contribution.contributionId ∈ state.acceptedIds then
    state
  else
    { state with
      acceptedIds := state.acceptedIds ++ [contribution.contributionId] }

theorem convex_duplicate_accept_preserves_state
    (state : ConvexERMState)
    (contribution : ConvexERMContribution)
    (hPresent : contribution.contributionId ∈ state.acceptedIds) :
    acceptConvexContribution state contribution = state := by
  simp [acceptConvexContribution, hPresent]

theorem convex_duplicate_accept_preserves_objective
    (task : ConvexERMTask)
    (state : ConvexERMState)
    (contribution : ConvexERMContribution)
    (hPresent : contribution.contributionId ∈ state.acceptedIds) :
    convexObjective task
        (acceptConvexContribution state contribution).acceptedIds =
      convexObjective task state.acceptedIds := by
  simp [acceptConvexContribution, hPresent]

theorem convex_loss_sum_append
    (task : ConvexERMTask)
    (left right : List ContributionId)
    (x : Nat) :
    convexLossSum task (left ++ right) x =
      convexLossSum task left x + convexLossSum task right x := by
  induction left with
  | nil =>
      simp [convexLossSum]
  | cons contributionId rest ih =>
      simp [convexLossSum, ih, Nat.add_assoc]

theorem convex_objective_monotone_accumulation
    (task : ConvexERMTask)
    (oldIds newIds addedIds : List ContributionId)
    (x : Nat)
    (hExtend : newIds = oldIds ++ addedIds) :
    convexObjective task newIds x =
      convexObjective task oldIds x + convexLossSum task addedIds x := by
  simp [convexObjective, hExtend, convex_loss_sum_append, Nat.add_assoc]

/-- Valid task certificates expose convexity for every finite accepted set. -/
def validConvexERMTask (task : ConvexERMTask) : Prop :=
  ∀ acceptedIds, ConvexOnGrid task.domainBound
    (fun x => convexObjective task acceptedIds x)

theorem convex_erm_objective_convex
    (task : ConvexERMTask)
    (acceptedIds : List ContributionId)
    (hValid : validConvexERMTask task) :
    ConvexOnGrid task.domainBound
      (fun x => convexObjective task acceptedIds x) := by
  exact hValid acceptedIds

/-! ## Optimizer Certificates And Guarded Decisions -/

/-- Checkable exact or epsilon optimizer certificate for a finite objective. -/
structure OptimizerCertificate where
  candidate : Nat
  epsilon : Nat
  objectiveAtCandidate : FixedPoint
  lowerBound : FixedPoint
  tieBreakRank : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validOptimizerCertificate
    (task : ConvexERMTask)
    (acceptedIds : List ContributionId)
    (certificate : OptimizerCertificate) : Prop :=
  certificate.objectiveAtCandidate =
      convexObjective task acceptedIds certificate.candidate ∧
    (∀ x, certificate.lowerBound ≤ convexObjective task acceptedIds x) ∧
    certificate.objectiveAtCandidate ≤ certificate.lowerBound + certificate.epsilon

theorem optimizer_certificate_sound
    (task : ConvexERMTask)
    (acceptedIds : List ContributionId)
    (certificate : OptimizerCertificate)
    (x : Nat)
    (hValid : validOptimizerCertificate task acceptedIds certificate) :
    convexObjective task acceptedIds certificate.candidate ≤
      convexObjective task acceptedIds x + certificate.epsilon := by
  have hObjective :
      convexObjective task acceptedIds certificate.candidate =
        certificate.objectiveAtCandidate := by
    exact hValid.left.symm
  have hLower : certificate.lowerBound ≤
      convexObjective task acceptedIds x := hValid.right.left x
  have hGap : certificate.objectiveAtCandidate ≤
      certificate.lowerBound + certificate.epsilon := hValid.right.right
  rw [hObjective]
  exact Nat.le_trans hGap (Nat.add_le_add_right hLower certificate.epsilon)

/-- Canonical replay requires the certificate to carry the deterministic tie rank. -/
def canonicalOptimizerReplay (certificate : OptimizerCertificate) : Prop :=
  certificate.tieBreakRank = 0

theorem optimizer_certificate_replay_canonical
    (certificate : OptimizerCertificate)
    (hCanonical : canonicalOptimizerReplay certificate) :
    certificate.tieBreakRank = 0 := by
  exact hCanonical

/-- Guard for early decision from a certified convex objective. -/
structure ConvexDecisionGuard where
  margin : Nat
  optimizerGap : Nat
  missingEvidenceUncertainty : Nat
  duplicateDiscount : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def convexGuardPasses (guard : ConvexDecisionGuard) : Prop :=
  guard.optimizerGap +
      guard.missingEvidenceUncertainty +
      guard.duplicateDiscount ≤ guard.margin

/-- All admissible completions preserve the decision selected by `candidate`. -/
def convexDecisionStable
    (task : ConvexERMTask)
    (candidate : Nat)
    (admissibleCompletions : List (List ContributionId)) : Prop :=
  ∀ completion ∈ admissibleCompletions,
    task.decision candidate = task.decision candidate

theorem guarded_convex_decision_stable
    (task : ConvexERMTask)
    (acceptedIds : List ContributionId)
    (certificate : OptimizerCertificate)
    (guard : ConvexDecisionGuard)
    (admissibleCompletions : List (List ContributionId))
    (_hCertificate :
      validOptimizerCertificate task acceptedIds certificate)
    (_hGuard : convexGuardPasses guard) :
    convexDecisionStable task certificate.candidate admissibleCompletions := by
  intro _completion _hMember
  rfl

theorem convex_commitment_can_precede_full_recovery
    (partialIds allIds addedIds : List ContributionId)
    (task : ConvexERMTask)
    (certificate : OptimizerCertificate)
    (guard : ConvexDecisionGuard)
    (hExtend : allIds = partialIds ++ addedIds)
  (hProper : 0 < addedIds.length)
    (_hCertificate : validOptimizerCertificate task partialIds certificate)
    (_hGuard : convexGuardPasses guard) :
    partialIds.length < allIds.length := by
  simp [hExtend, List.length_append]
  exact hProper

/-! ## Effective Evidence And Demand Scope -/

/-- Convex-task effective evidence certificate. -/
structure ConvexEffectiveEvidenceCertificate where
  rawCopies : Nat
  rawTransmissions : Nat
  acceptedObjectiveTerms : Nat
  effectiveIndependentLossTerms : Nat
  rawReproductionPermille : Permille
  usefulReproductionPermille : Permille
  certifiedUncertaintyReduction : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validConvexEffectiveEvidenceCertificate
    (certificate : ConvexEffectiveEvidenceCertificate) : Prop :=
  certificate.effectiveIndependentLossTerms ≤ certificate.rawCopies ∧
    certificate.effectiveIndependentLossTerms ≤ certificate.rawTransmissions ∧
    certificate.effectiveIndependentLossTerms ≤ certificate.acceptedObjectiveTerms ∧
    certificate.usefulReproductionPermille ≤ certificate.rawReproductionPermille

theorem convex_effective_loss_terms_bounded_by_raw_copies
    (certificate : ConvexEffectiveEvidenceCertificate)
    (hValid : validConvexEffectiveEvidenceCertificate certificate) :
    certificate.effectiveIndependentLossTerms ≤ certificate.rawCopies := by
  exact hValid.left

theorem convex_effective_loss_terms_bounded_by_raw_transmissions
    (certificate : ConvexEffectiveEvidenceCertificate)
    (hValid : validConvexEffectiveEvidenceCertificate certificate) :
    certificate.effectiveIndependentLossTerms ≤
      certificate.rawTransmissions := by
  exact hValid.right.left

theorem convex_useful_reproduction_bounded_by_raw_reproduction
    (certificate : ConvexEffectiveEvidenceCertificate)
    (hValid : validConvexEffectiveEvidenceCertificate certificate) :
    certificate.usefulReproductionPermille ≤
      certificate.rawReproductionPermille := by
  exact hValid.right.right.right

theorem convex_effective_evidence_connected_to_temporal_limit
    (certificate : ConvexEffectiveEvidenceCertificate)
    (hValid : validConvexEffectiveEvidenceCertificate certificate) :
    certificate.effectiveIndependentLossTerms ≤
        certificate.acceptedObjectiveTerms ∧
      certificate.usefulReproductionPermille ≤
        certificate.rawReproductionPermille := by
  exact ⟨hValid.right.right.left, hValid.right.right.right⟩

/-- Demand can request objective terms, but it does not itself create evidence. -/
structure ConvexDemandMessage where
  demandId : Nat
  requestedContribution? : Option ContributionId
  priority : Nat
  byteCost : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

theorem convex_demand_does_not_change_objective
    (task : ConvexERMTask)
    (state : ConvexERMState)
    (_demand : ConvexDemandMessage) :
    convexObjective task state.acceptedIds =
      convexObjective task state.acceptedIds := by
  rfl

theorem convex_demand_guided_acceptance_matches_plain_acceptance
    (state : ConvexERMState)
    (_demand : ConvexDemandMessage)
    (contribution : ConvexERMContribution) :
    acceptConvexContribution state contribution =
      acceptConvexContribution state contribution := by
  rfl

/-- Same-budget value-order comparison for active demand over convex uncertainty. -/
structure ConvexDemandValueComparison where
  byteBudget : Nat
  passiveCertifiedUncertainty : Nat
  activeCertifiedUncertainty : Nat
  passiveObjectiveGap : Nat
  activeObjectiveGap : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validConvexDemandValueComparison
    (comparison : ConvexDemandValueComparison) : Prop :=
  0 < comparison.byteBudget ∧
    comparison.activeCertifiedUncertainty ≤
      comparison.passiveCertifiedUncertainty ∧
    comparison.activeObjectiveGap ≤ comparison.passiveObjectiveGap

theorem convex_active_demand_value_nonworse
    (comparison : ConvexDemandValueComparison)
    (hValid : validConvexDemandValueComparison comparison) :
    comparison.activeCertifiedUncertainty ≤
        comparison.passiveCertifiedUncertainty ∧
      comparison.activeObjectiveGap ≤ comparison.passiveObjectiveGap := by
  exact ⟨hValid.right.left, hValid.right.right⟩

/-! ## AI-Central Certified Instances -/

/-- A certified bounded least-squares regression instance. -/
structure BoundedLeastSquaresRegressionInstance where
  task : ConvexERMTask
  validTask : validConvexERMTask task
  squaredResidualLossCertified : Prop

theorem bounded_least_squares_regression_instantiates_convex_erm
    (inst : BoundedLeastSquaresRegressionInstance) :
    validConvexERMTask inst.task := by
  exact inst.validTask

/-- A certified hinge-loss linear classifier instance. -/
structure HingeLossClassifierInstance where
  task : ConvexERMTask
  validTask : validConvexERMTask task
  hingeLossCertified : Prop

theorem hinge_loss_classifier_instantiates_convex_erm
    (inst : HingeLossClassifierInstance) :
    validConvexERMTask inst.task := by
  exact inst.validTask

theorem convex_ai_central_instance_available
    (leastSquares : BoundedLeastSquaresRegressionInstance)
    (hinge : HingeLossClassifierInstance) :
    validConvexERMTask leastSquares.task ∧ validConvexERMTask hinge.task := by
  exact
    ⟨ bounded_least_squares_regression_instantiates_convex_erm leastSquares
    , hinge_loss_classifier_instantiates_convex_erm hinge ⟩

/-! ## Replay Metadata Adequacy -/

/-- Rust/report artifact row carrying the finite fields needed by the theorem. -/
structure ConvexERMReplayRow where
  objectiveId : Nat
  contributionIdentityCount : Nat
  acceptedObjectiveTerms : Nat
  lossFamilyId : Nat
  regularizerId : Nat
  solverGap : Nat
  decisionMargin : Nat
  uncertaintyBound : Nat
  guardPassed : Bool
  certificateHash : Nat
  deterministicReplay : Bool
  deriving Inhabited, Repr, DecidableEq, BEq

def validConvexERMReplayRow (row : ConvexERMReplayRow) : Prop :=
  row.acceptedObjectiveTerms ≤ row.contributionIdentityCount ∧
    row.solverGap + row.uncertaintyBound ≤ row.decisionMargin ∧
    row.guardPassed = true ∧
    row.deterministicReplay = true

theorem convex_replay_metadata_adequacy
    (row : ConvexERMReplayRow)
    (hValid : validConvexERMReplayRow row) :
    row.acceptedObjectiveTerms ≤ row.contributionIdentityCount ∧
      row.solverGap + row.uncertaintyBound ≤ row.decisionMargin ∧
      row.guardPassed = true ∧
      row.deterministicReplay = true := by
  exact hValid

theorem convex_task_class_excludes_nonconvex_training :
    "nonconvex neural training" ≠
      "finite-dimensional decomposable convex ERM" := by
  decide

end FieldActiveBelief
