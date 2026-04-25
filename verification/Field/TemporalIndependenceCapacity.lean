import Field.TemporalIndependenceLimits

/-
The Problem. The base temporal independence-limit file proves that raw copies,
raw transmissions, and raw reproduction do not imply effective independence.
The proof stack also needs finite certificate versions of the stronger capacity
and entropy sketches without turning the base file into an oversized source.

Solution Structure.
1. Define contact entropy, dispersion, and temporal generator-rank proxies.
2. State deterministic reconstruction and capacity certificates over those
   finite proxies.
3. Keep the stronger stochastic capacity theorem explicitly deferred unless a
   later probability surface proves it.
-/

/-! # Temporal Independence Capacity Certificates -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-! ## Contact Entropy, Dispersion, And Temporal Capacity -/

/-- Finite contact-entropy and dispersion certificate for a temporal window. -/
structure ContactEntropySummary where
  rawContacts : Nat
  rawCarriers : Nat
  forwardingEvents : Nat
  contactEntropyPermille : Permille
  dispersionPermille : Permille
  deriving Inhabited, Repr, DecidableEq, BEq

def validContactEntropySummary
    (summary : ContactEntropySummary) : Prop :=
  summary.contactEntropyPermille ≤ summary.forwardingEvents ∧
    summary.dispersionPermille ≤ summary.rawCarriers + summary.rawContacts ∧
    summary.contactEntropyPermille ≤ 1000 ∧
    summary.dispersionPermille ≤ 1000

theorem contact_entropy_and_dispersion_bounded_by_raw_activity
    (summary : ContactEntropySummary)
    (hValid : validContactEntropySummary summary) :
    summary.contactEntropyPermille ≤ summary.forwardingEvents ∧
      summary.dispersionPermille ≤ summary.rawCarriers + summary.rawContacts := by
  -- Entropy and dispersion are bounded replay certificates, not aliases for traffic.
  exact ⟨hValid.left, hValid.right.left⟩

/-- High raw activity can still have low contact entropy. -/
def highContactLowEntropySummary : ContactEntropySummary :=
  { rawContacts := 120
    rawCarriers := 50
    forwardingEvents := 200
    contactEntropyPermille := 40
    dispersionPermille := 30 }

theorem low_contact_entropy_can_coexist_with_high_transmission_count :
    100 ≤ highContactLowEntropySummary.forwardingEvents ∧
      highContactLowEntropySummary.contactEntropyPermille < 100 := by
  -- This is the finite witness behind "contact entropy, not just rate".
  decide

/-- Temporal generator-rank proxy induced by finite contact movement. -/
structure TemporalGeneratorMatrixCertificate where
  receivedRows : Nat
  independentColumns : Nat
  rankProxy : Nat
  duplicateLineageRows : Nat
  effectiveRank : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validTemporalGeneratorMatrixCertificate
    (certificate : TemporalGeneratorMatrixCertificate) : Prop :=
  certificate.rankProxy ≤ certificate.receivedRows ∧
    certificate.rankProxy ≤ certificate.independentColumns ∧
    certificate.effectiveRank ≤ certificate.rankProxy ∧
    certificate.duplicateLineageRows + certificate.rankProxy ≤
      certificate.receivedRows + certificate.duplicateLineageRows

theorem effective_rank_bounded_by_temporal_generator_rank
    (certificate : TemporalGeneratorMatrixCertificate)
    (hValid : validTemporalGeneratorMatrixCertificate certificate) :
    certificate.effectiveRank ≤ certificate.rankProxy := by
  -- Effective rank cannot exceed the temporal generator-rank proxy.
  exact hValid.right.right.left

theorem duplicate_lineage_rows_do_not_increase_rank_proxy
    (certificate : TemporalGeneratorMatrixCertificate)
    (hValid : validTemporalGeneratorMatrixCertificate certificate) :
    certificate.duplicateLineageRows + certificate.rankProxy ≤
      certificate.receivedRows + certificate.duplicateLineageRows := by
  -- Duplicate lineage rows are accounted separately from rank.
  exact hValid.right.right.right

/-- Entropy/dispersion bound for effective reconstruction. -/
structure EntropyDispersionReconstructionBound where
  window : CodingWindow
  diversity : TemporalContactDiversitySummary
  entropy : ContactEntropySummary
  maxEffectiveRankFromEntropy : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validEntropyDispersionReconstructionBound
    (bound : EntropyDispersionReconstructionBound) : Prop :=
  validTemporalContactDiversitySummary bound.diversity ∧
    validContactEntropySummary bound.entropy ∧
    bound.diversity.effectiveRank ≤ bound.maxEffectiveRankFromEntropy ∧
    bound.maxEffectiveRankFromEntropy ≤
      bound.entropy.contactEntropyPermille +
        bound.entropy.dispersionPermille +
        bound.diversity.byteBudget +
        bound.diversity.timeHorizon

theorem reconstruction_bound_from_entropy_and_dispersion
    (bound : EntropyDispersionReconstructionBound)
    (hValid : validEntropyDispersionReconstructionBound bound)
    (hRecover : effectiveReconstructable bound.window bound.diversity) :
    bound.window.k ≤ bound.maxEffectiveRankFromEntropy := by
  -- Reconstruction through the certificate is limited by the effective rank
  -- allowed by entropy, dispersion, budget, and time.
  exact Nat.le_trans hRecover hValid.right.right.left

/-! ## Temporal Contact Capacity -/

/-- Narrow finite capacity certificate for one temporal contact process. -/
structure TemporalContactCapacityCertificate where
  byteBudget : Nat
  timeHorizon : Nat
  storageBudget : Nat
  independentArrivalCapacity : Nat
  reconstructionThreshold : Nat
  commitmentThreshold : Nat
  contactDiversity : Nat
  rawContactRate : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validTemporalContactCapacityCertificate
    (certificate : TemporalContactCapacityCertificate) : Prop :=
  certificate.independentArrivalCapacity ≤ certificate.byteBudget + certificate.timeHorizon ∧
    certificate.independentArrivalCapacity ≤
      certificate.storageBudget + certificate.contactDiversity

def capacityAchievesReconstruction
    (certificate : TemporalContactCapacityCertificate) : Prop :=
  certificate.reconstructionThreshold ≤ certificate.independentArrivalCapacity

def capacityAchievesCommitment
    (certificate : TemporalContactCapacityCertificate) : Prop :=
  certificate.commitmentThreshold ≤ certificate.independentArrivalCapacity

theorem temporal_contact_capacity_bounded_by_independent_arrivals
    (certificate : TemporalContactCapacityCertificate)
    (hValid : validTemporalContactCapacityCertificate certificate) :
    certificate.independentArrivalCapacity ≤
        certificate.byteBudget + certificate.timeHorizon ∧
      certificate.independentArrivalCapacity ≤
        certificate.storageBudget + certificate.contactDiversity := by
  -- Capacity is a finite certificate over independent arrivals, not raw contact rate.
  exact hValid

theorem temporal_contact_capacity_monotone_in_budget
    (left right : TemporalContactCapacityCertificate)
    (hCapacity : left.independentArrivalCapacity ≤ right.independentArrivalCapacity)
    (hLeft : capacityAchievesReconstruction left) :
    left.reconstructionThreshold ≤ right.independentArrivalCapacity := by
  -- Increasing the certified independent-arrival capacity preserves threshold reachability.
  exact Nat.le_trans hLeft hCapacity

/-- Raw contact can increase without increasing certified capacity. -/
def rawContactIncreaseFixedCapacityWitness :
    TemporalContactCapacityCertificate × TemporalContactCapacityCertificate :=
  ( { byteBudget := 10
      timeHorizon := 10
      storageBudget := 4
      independentArrivalCapacity := 3
      reconstructionThreshold := 5
      commitmentThreshold := 3
      contactDiversity := 1
      rawContactRate := 20 }
  , { byteBudget := 10
      timeHorizon := 10
      storageBudget := 4
      independentArrivalCapacity := 3
      reconstructionThreshold := 5
      commitmentThreshold := 3
      contactDiversity := 1
      rawContactRate := 100 } )

theorem raw_contact_rate_increase_does_not_imply_capacity_increase :
    rawContactIncreaseFixedCapacityWitness.1.rawContactRate <
        rawContactIncreaseFixedCapacityWitness.2.rawContactRate ∧
      rawContactIncreaseFixedCapacityWitness.1.independentArrivalCapacity =
        rawContactIncreaseFixedCapacityWitness.2.independentArrivalCapacity := by
  -- More raw contact does not help if independent arrival capacity is unchanged.
  decide

/-! ## Triangle, Useful Reproduction, And Matched Networks -/

/-- Reliability/resource/observer-ambiguity limit triangle certificate. -/
structure ReliabilityResourceAmbiguityBoundary where
  reliabilityTarget : Nat
  resourceBudget : Nat
  observerAmbiguityTarget : Nat
  achievedReliability : Nat
  resourceSpent : Nat
  achievedObserverAmbiguity : Nat
  independenceBudget : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validReliabilityResourceAmbiguityBoundary
    (boundary : ReliabilityResourceAmbiguityBoundary) : Prop :=
  boundary.achievedReliability + boundary.achievedObserverAmbiguity ≤
      boundary.resourceSpent + boundary.independenceBudget ∧
    boundary.resourceBudget + boundary.independenceBudget <
      boundary.reliabilityTarget + boundary.observerAmbiguityTarget

theorem reliability_resource_ambiguity_triangle_incompatibility
    (boundary : ReliabilityResourceAmbiguityBoundary)
    (hValid : validReliabilityResourceAmbiguityBoundary boundary) :
    ¬ (boundary.resourceSpent ≤ boundary.resourceBudget ∧
        boundary.reliabilityTarget ≤ boundary.achievedReliability ∧
        boundary.observerAmbiguityTarget ≤ boundary.achievedObserverAmbiguity) := by
  -- With too little resource plus independence budget, high reliability and
  -- high ambiguity cannot both be certified at low cost.
  intro hAll
  have hTargets :
      boundary.reliabilityTarget + boundary.observerAmbiguityTarget ≤
        boundary.achievedReliability + boundary.achievedObserverAmbiguity :=
    Nat.add_le_add hAll.right.left hAll.right.right
  have hAchieved :
      boundary.achievedReliability + boundary.achievedObserverAmbiguity ≤
        boundary.resourceBudget + boundary.independenceBudget :=
    Nat.le_trans hValid.left (Nat.add_le_add hAll.left (Nat.le_refl _))
  have hTargetsLeBudget :
      boundary.reliabilityTarget + boundary.observerAmbiguityTarget ≤
        boundary.resourceBudget + boundary.independenceBudget :=
    Nat.le_trans hTargets hAchieved
  exact Nat.not_lt_of_ge hTargetsLeBudget hValid.right

theorem raw_reproduction_above_one_does_not_imply_effective_reproduction_above_one :
    highCopyLowIndependenceSummary.rawReproductionPermille > 1000 ∧
      ¬ highCopyLowIndependenceSummary.effectiveReproductionPermille > 1000 := by
  -- High raw reproduction can coexist with subcritical useful reproduction.
  unfold highCopyLowIndependenceSummary
  simp

theorem effective_reproduction_finite_horizon_bound
    (summary : TemporalContactDiversitySummary)
    (hValid : validTemporalContactDiversitySummary summary) :
    summary.effectiveReproductionPermille ≤ summary.rawReproductionPermille ∧
      summary.effectiveReproductionPermille ≤ 1000 ∨
        1000 ≤ summary.effectiveReproductionPermille := by
  -- The achieved useful reproduction pressure is explicit; if it exceeds 1000,
  -- that supercritical useful region is visible rather than inferred from raw R.
  exact Or.elim (Nat.le_total summary.effectiveReproductionPermille 1000)
    (fun hUseful => Or.inl ⟨hValid.right.right.right.right.left, hUseful⟩)
    (fun hSuper => Or.inr hSuper)

/-- Matched-network pair with explicit entropy and dispersion witnesses. -/
structure MatchedEntropyNetworkPair where
  pair : MatchedTemporalNetworkPair
  correlatedEntropy : ContactEntropySummary
  diverseEntropy : ContactEntropySummary
  deriving Inhabited, Repr, DecidableEq, BEq

def entropySeparatedMatchedPair
    (window : CodingWindow)
    (matched : MatchedEntropyNetworkPair) : Prop :=
  matchedOnRawSpreadAndBudget matched.pair ∧
    matched.correlatedEntropy.rawContacts = matched.diverseEntropy.rawContacts ∧
    matched.correlatedEntropy.forwardingEvents =
      matched.diverseEntropy.forwardingEvents ∧
    matched.correlatedEntropy.contactEntropyPermille <
      matched.diverseEntropy.contactEntropyPermille ∧
    matched.correlatedEntropy.dispersionPermille <
      matched.diverseEntropy.dispersionPermille ∧
    differentEffectiveRankOutcome window matched.pair

theorem matched_networks_separate_by_entropy_and_effective_rank
    (window : CodingWindow)
    (matched : MatchedEntropyNetworkPair)
    (hSeparated : entropySeparatedMatchedPair window matched) :
    matchedOnRawSpreadAndBudget matched.pair ∧
      matched.correlatedEntropy.contactEntropyPermille <
        matched.diverseEntropy.contactEntropyPermille ∧
      matched.correlatedEntropy.dispersionPermille <
        matched.diverseEntropy.dispersionPermille ∧
      ¬ effectiveReconstructable window matched.pair.correlated ∧
      effectiveReconstructable window matched.pair.diverse := by
  -- Equal raw cost and spread can still separate by entropy, dispersion, and rank.
  exact
    ⟨ hSeparated.left
    , hSeparated.right.right.right.left
    , hSeparated.right.right.right.right.left
    , hSeparated.right.right.right.right.right.left
    , hSeparated.right.right.right.right.right.right ⟩

inductive ErrorCorrectionLimitSketchStatus where
  | proved
  | replayValidated
  | deferred
  | removed
  deriving Inhabited, Repr, DecidableEq

/- The stronger stochastic capacity theorem remains deliberately separate from
the finite certificate theorem unless a later probability surface proves it. -/
def stochasticCapacitySketchStatus : ErrorCorrectionLimitSketchStatus :=
  ErrorCorrectionLimitSketchStatus.deferred

end FieldActiveBelief
