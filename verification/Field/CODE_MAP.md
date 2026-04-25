# Field Verification Code Map

This map describes the current organization of `verification/Field`.

## Proof Audit Status

The verification tree contains no `sorry`, `admit`, or `axiom` declarations in
the Lean sources. The active research theorem path is fully checked by Lean, but
not every theorem is unconditional. Performance-oriented statements in
`Field/CodedDiffusionStrong.lean`, `Field/ActiveBeliefDefinitive.lean`,
`Field/ActiveBeliefDefinitiveClosure.lean`,
`Field/ActiveBeliefCertificates.lean`, `Field/ActiveBeliefEndToEnd.lean`,
`Field/TemporalIndependenceLimits.lean`, and
`Field/TemporalIndependenceCapacity.lean` are assumption-boundary or finite
certificate lemmas: they prove consequences of explicit finite-horizon, margin,
demand-quality, controller-band, cost, stress, observer-projection,
effective-independence, entropy/dispersion, temporal-capacity, or
replay-certificate records. They must not be cited as deriving those
assumptions from arbitrary temporal traces.

Unconditional theorem surfaces are the algebraic, safety, and accounting
results: duplicate non-inflation, k-of-n reconstruction from rank, recoding and
aggregate preservation under declared ledgers, demand non-interference,
guarded-statistic decoding, right-censoring, generic duplicate suppression,
effective-rank upper bounds, negative boundary counterexamples, and bounded
replay/projection facts.

## Top-Level Theorem Packs

- `Field/CodedDiffusion.lean`
  - active coded-diffusion theorem path for evidence-origin modes, contribution ledgers, k-of-n reconstruction, duplicate non-inflation, recoding soundness, observer projection, diffusion-potential accounting, and finite deterministic work recurrence
- `Field/CodedDiffusionStrong.lean`
  - strong coded-diffusion theorem path for finite-horizon probability assumptions, receiver-arrival bounds, useful-inference arrival bounds, anomaly-margin bounds, guarded false-commitment bounds, and inference-potential drift
- `Field/ActiveBelief.lean`
  - active belief diffusion theorem path for receiver-indexed belief state, first-class bounded demand messages, evidence messages, demand soundness, duplicate non-inflation under demand-driven forwarding, commitment lead-time accounting, stale-demand safety, multi-receiver compatibility, and propagated host/bridge demand soundness
- `Field/ActiveBeliefStrong.lean`
  - strong active belief theorem path for mergeable-statistic decoding,
    guarded commitment correctness, compatibility from non-identical partial
    histories, demand-guided non-interference on the merged statistic,
    positive commitment lead time under explicit useful-inference assumptions,
    and innovative-quality monotonicity
- `Field/ActiveBeliefDefinitive.lean`
  - definitive theorem-closure path for active-demand improvement, generic
    mergeable-inference soundness, commitment-before-full-recovery bounds,
    receiver-set compatibility bounds, near-critical controller stabilization,
    aggregation efficiency, negative task-class boundaries, bounded stress, and
    observer projection metrics
- `Field/ActiveBeliefDefinitiveClosure.lean`
  - small extension pack for replay-visible receiver-disagreement
    explanations, raw/useful reproduction pressure, and
    effective-independence potential drift
- `Field/ActiveBeliefCertificates.lean`
  - replay-certificate bridge path proving that explicit artifact-facing
    certificate records imply receiver-arrival, useful-inference, score-margin,
    demand-policy, near-critical controller, theorem-profile metadata, and
    bounded-stress assumption records
- `Field/ActiveBeliefEndToEnd.lean`
  - end-to-end reduced finite-trace theorem path connecting replay-visible
    evidence events, active demand, folded receiver state, guarded commitment,
    demand-policy value ordering, and Rust replay-validator metadata adequacy
- `Field/ActiveBeliefDecisionSufficiency.lean`
  - decision-sufficiency theorem path for the distributed error-correction
    limit: stable guarded decision basins can be reached before full k-of-n
    reconstruction, exact recovery is a special case, demand can target basin
    progress, and non-stable partial decisions form an explicit boundary
- `Field/TemporalIndependenceLimits.lean`
  - temporal independence-limit theorem path for distributed error correction:
    effective rank is separated from raw copies/transmissions, reconstruction
    requires contact-generated independence, recovery probability is bounded by
    effective-rank probability, raw reproduction is insufficient, matched
    networks separate by contact diversity, and the cost-time-independence
    triangle is explicit
- `Field/TemporalIndependenceCapacity.lean`
  - extension pack for contact entropy and dispersion, temporal
    generator-rank proxies, entropy/dispersion reconstruction bounds, narrow
    temporal-contact capacity certificates, limit-triangle certificates, and
    matched entropy-separated networks
- `Field/Architecture.lean`
  - shared enum vocabulary for projection kinds, refinement-ladder stages, lineage stages, and semantic-versus-proof-artifact roles
- `Field/CostAPI.lean`
  - shared work-unit and budget vocabulary reused by router, system, and adequacy cost packs

- `Field/LocalModel.lean`
  - imports the local observer-controller model, the probabilistic information layer, the local refinement theorems, and the first decision procedure
- `Field/Information.lean`
  - imports the information API, concrete probabilistic realization, Bayesian update layer, calibration/blindness packs, and quantitative difference lemmas
- `Field/PrivateProtocol.lean`
  - imports the reduced private choreography/runtime layer, conservation/coherence packs, concrete fixtures, the protocol-closure theorem pack, and the Telltale-family bridge
- `Field/Boundary.lean`
  - imports the observational controller-boundary theorems
- `Field/Adequacy.lean`
  - imports the Rust-runtime adequacy bridge, low-level runtime-to-canonical alignment theorems, search-aware adequacy closure, stronger projected runtime/system refinement theorems, runtime-state execution refinement theorems, runtime/system safety-preservation results, probabilistic preservation theorems, first budgeted-optimality preservation theorems, and proof-facing fixture cases
- `Field/Network.lean`
  - imports the reduced finite network layer and its first safety theorems
- `Field/Router.lean`
  - imports the reduced publication, admission, installation, lifecycle, canonical-selection, and posterior-decision layers
- `Field/Search.lean`
  - imports the proof-facing reduced field search boundary
- `Field/Async.lean`
  - imports the reduced async delivery semantics, transport lifecycle lemmas, and first async safety theorems
- `Field/Retention.lean`
  - imports the reduced payload-retention policy/custody layer, executable bounded retention instance, separation/refinement theorems, and proof-facing fixtures
- `Field/System.lean`
  - imports system-level summaries, reduced end-to-end semantics, probabilistic evidence-flow theorems, refinement to router-owned canonical selection above the async layer, and the first budgeted/reduced-context optimality theorems
- `Field/Quality.lean`
  - imports the reduced routing-quality / comparison, reference-best, and support-only refinement layer above the router and system boundaries
- `Field/Assumptions.lean`
  - imports the proof-contract vocabulary and theorem-packaging surface used across the field stack
- `Field/Field.lean`
  - umbrella import for the current field verification stack; imports
    `Field/CodedDiffusion.lean`, `Field/CodedDiffusionStrong.lean`,
    `Field/ActiveBelief.lean`, `Field/ActiveBeliefStrong.lean`,
    `Field/ActiveBeliefDefinitive.lean`,
    `Field/ActiveBeliefDefinitiveClosure.lean`, and
    `Field/ActiveBeliefCertificates.lean`, `Field/ActiveBeliefEndToEnd.lean`,
    `Field/ActiveBeliefDecisionSufficiency.lean`, and
    `Field/TemporalIndependenceLimits.lean`, and
    `Field/TemporalIndependenceCapacity.lean` as the active research theorem
    path and keeps older route/corridor packs as legacy baseline context

## Active Coded-Diffusion Path

- `Field/CodedDiffusion.lean`
  - owns the active coded-diffusion proof vocabulary:
    - `EvidenceOriginMode` for source-coded, locally generated, and recoded/aggregated evidence
    - `EvidenceId`, `ContributionId`, and `LocalObservationId` for proof-facing ids
    - `CodingWindow` for k-of-n reconstruction requirements
    - `ReceiverRank` for independent receiver contribution ids and duplicate/innovative arrival accounting
    - `ReconstructionQuorum` for valid k-of-n reconstruction quorums
    - `ContributionLedgerKind` and `ContributionLedgerRecord` for source, local, parent-ledger-union, and aggregate-with-local-observation contribution validity
    - `FragmentObservation` and `ObserverProjection` for observer-visible fragment/rank/custody projection
    - `DiffusionPotential` for rank-deficit, duplicate-pressure, and storage-pressure accounting
    - `finiteWork` for deterministic finite-horizon work recurrence support
  - completed theorem names:
    - `coding_window_valid_k_pos`
    - `coding_window_valid_k_le_n`
    - `k_of_n_reconstruction`
    - `valid_quorum_implies_reconstruction`
    - `duplicate_non_inflation`
    - `innovative_arrival_increases_rank_by_one`
    - `innovative_evidence_increases_rank_exactly_when_new`
    - `duplicate_evidence_preserves_rank_when_present`
    - `reconstruction_monotonicity_innovative`
    - `recoding_soundness_parent_contribution_ledger`
    - `aggregate_contribution_requires_local_observation`
    - `recoded_duplicate_non_inflation`
    - `source_and_local_evidence_share_rank_accounting`
    - `observer_projection_preserves_rank`
    - `observer_projection_preserves_duplicate_count`
    - `observer_projection_preserves_custody_count`
    - `innovative_step_rank_deficit_nonincreasing`
    - `duplicate_step_preserves_rank_deficit`
    - `duplicate_step_increases_duplicate_pressure`
    - `potential_accounting_innovative`
    - `potential_accounting_duplicate`
    - `finite_work_recurrence`
    - `finite_work_step_monotone`
    - `inference_progress_uncertainty_nonincreasing`
    - `inference_potential_total_is_accounted_sum`
    - `majority_duplicate_non_inflation`
    - `majority_positive_innovative_increases_vote_count`
  - Rust alignment:
    - `EvidenceOriginMode`, `ContributionLedgerKind`, `ContributionLedgerRecord`, `CodingWindow`, `ReceiverRank`, and reconstruction/recoding theorem names intentionally mirror `crates/field/src/research.rs`.

## Strong Coded-Diffusion Path

- `Field/CodedDiffusionStrong.lean`
  - owns the strong proof vocabulary:
    - `TemporalContactProbabilityModel`, `ContactDependenceAssumption`, and
      `ReceiverArrivalBound` for the finite-horizon receiver-arrival
      assumption surface
    - `UsefulInferenceArrivalBound` for task-relevant contribution arrival
      before full recovery
    - `BoundedScoreVectorUpdateModel` and `AnomalyCommitmentGuard` for
      anomaly-margin and false-commitment theorem assumptions
    - `InferenceDriftAssumption` for the strong progress/drift potential
      statement
  - completed theorem names:
    - `receiver_arrival_reconstruction_bound`
    - `useful_inference_arrival_bound`
    - `anomaly_margin_lower_tail_bound`
    - `guarded_commitment_false_probability_bounded`
    - `inference_potential_drift_progress`
  - strong proposal boundary:
    - receiver-arrival reconstruction, useful-inference arrival, and
      anomaly-margin conclusions are theorem-backed under explicit
      finite-horizon assumption records. The proofs do not hide mobility,
      independence, or lower-tail assumptions; experiment rows must report
      which regimes satisfy them.
    - `InferencePotential` mirrors the implemented uncertainty, wrong-basin,
      duplicate, storage, and transmission pressure terms.
    - `InferenceDriftAssumption` upgrades accounting into a progress/drift
      statement under explicit controller assumptions.
    - `MajorityThresholdState` supplies the stronger second mergeable task
      boundary beyond set-union reconstruction.
  - Telltale-family mapping:
    - Reuses conceptually, but does not import directly in the local coded-diffusion model, `Distributed/Families/DataAvailability.*` for reconstruction quorum and retrievability vocabulary.
    - Emulates locally the finite, deterministic subset of `Runtime/Proofs/Lyapunov.lean`, `Runtime/Proofs/ProtocolMachinePotential.lean`, and `Classical/Families/FosterLyapunovHarris.lean` through `DiffusionPotential`, `potential_accounting_*`, and `finiteWork`.
    - Reuses conceptually `Runtime/Proofs/ObserverProjection.lean`, `Protocol/InformationCost.lean`, and `Protocol/Noninterference*.lean` for the observer projection/erasure story; only local projection preservation is proved here.
    - Keeps probability-heavy concentration support local to the strong
      assumption records rather than importing a broader probability framework.
  - Rust alignment:
    - `ReceiverArrivalBound`, `UsefulInferenceArrivalBound`,
      `BoundedScoreVectorUpdateModel`, `AnomalyCommitmentGuard`, and
      `InferenceDriftAssumption` map to strong experiment theorem-assumption
      metadata rows added in the simulator artifact surface.

## Active Belief Diffusion Path

- `Field/ActiveBelief.lean`
  - owns the active belief diffusion proof vocabulary:
    - `ReceiverId`, `HypothesisId`, and `DemandEntryId` for receiver-indexed active belief objects
    - `QualitySummary` for proof-facing uncertainty, margin, and evidence-count summaries
    - `ReceiverBeliefState` for audited receiver state over `ReceiverRank`
    - `DemandEntry` and `DemandSummary` for bounded advisory demand control data
    - `validDemandSummary` and `expiredDemandSummary` for demand caps and lifetime semantics
    - `EvidenceProposal` and `demandAwareAccept` for demand-aware forwarding through the ordinary contribution gate
    - `ActiveMessage` for the first-class exchanged-message surface covering evidence and demand
    - `CommitmentTimeline` and `commitmentLeadTime` for logged lead-time accounting
    - `GuardedCommitment` and `compatibleCommitments` for multi-receiver compatibility without consensus
    - `demandPriorityScore` for proof-facing priority metadata that does not affect evidence acceptance
    - `ActiveDemandExecutionSurface` and `PropagatedDemandRecord` for
      host/bridge replay-visible active demand
  - completed theorem names:
    - `demand_bounded_by_entry_cap`
    - `demand_bounded_by_byte_cap`
    - `valid_demand_is_live`
    - `demand_summary_from_receiver_state_valid`
    - `demand_summary_from_receiver_state_has_canonical_singleton_order`
    - `demand_message_carries_no_contribution`
    - `evidence_message_carries_contribution`
    - `demand_cannot_validate_invalid_evidence`
    - `demand_accepts_only_through_valid_evidence`
    - `demand_duplicate_non_inflation`
    - `expired_demand_does_not_accept_invalid_evidence`
    - `commitment_lead_time_soundness`
    - `same_guarded_basin_compatible`
    - `compatible_commitments_have_same_hypothesis`
    - `demand_priority_does_not_change_acceptance`
    - `propagated_demand_is_replay_visible`
    - `propagated_demand_uses_host_bridge_surface`
    - `propagated_demand_carries_no_contribution`
    - `propagated_demand_cannot_validate_invalid_evidence`
    - `propagated_demand_duplicate_non_inflation`
  - non-claims:
    - demand is first-class replay-visible communication data, but it is not evidence
    - receiver compatibility is agreement on a guarded local decision, not consensus, common knowledge, or globally identical beliefs
    - active demand is not claimed optimal under arbitrary mobility or adversarial traces
    - propagated host/bridge demand remains non-evidential; bridge custody and
      replay metadata do not change contribution identity or evidence validity
  - Rust alignment target:
    - `DemandSummary`, `DemandEntry`, receiver-indexed belief summaries,
      commitment lead-time rows, receiver agreement rows, demand satisfaction
      rows, and stale-demand rejection counters are mirrored by the Rust/replay
      artifacts listed below.
    - `PropagatedDemandRecord` maps to the strong host/bridge replay artifact
      surface that distinguishes simulator-local demand from host/bridge
      demand.
    - `AnomalyLandscapeSummary`, `DecisionCommitmentState`,
      `ReceiverInferenceQualitySummary`, and
      `ReceiverBeliefCompatibilitySummary` are the Rust mirrors for the
      implemented merged-statistic, guarded-commitment, quality-order, and
      compatibility theorem surfaces.

- `Field/ActiveBeliefStrong.lean`
  - owns the strong active belief proof vocabulary:
    - `AdditiveScoreStatistic` as the proof-facing merged statistic for the
      additive score-vector task
    - `guardPassesOnStatistic` and `guardedCommitmentFromStatistic` for
      guarded decision construction directly from the merged statistic
    - `ReceiverQualityOrder` and `innovativeQualityStep` for theorem-facing
      sharpening claims
    - `PartialHistoryWitness`, `validPartialHistoryWitness`,
      `nonIdenticalPartialHistories`, and `compatiblePartialHistories` for
      multi-receiver compatibility without identical evidence histories
    - `AcceptedStatisticContribution`, `plainStatisticAccept`, and
      `demandGuidedStatisticAccept` for theorem-facing statistic semantics under
      active control
    - `LeadTimeWitness` for positive lead-time existence over explicit useful
      inference and guard assumptions
  - completed theorem names:
    - `guarded_commitment_decodes_statistic_decision`
    - `guarded_commitment_guard_passes_when_guard_holds`
    - `guarded_commitment_from_mergeable_statistic_correct`
    - `compatible_partial_histories_share_decision`
    - `compatible_partial_histories_are_nonidentical`
    - `compatible_partial_histories_yield_compatible_commitments`
    - `demand_guided_statistic_acceptance_matches_plain_acceptance`
    - `demand_guided_duplicate_preserves_statistic`
    - `propagated_demand_guided_statistic_acceptance_matches_plain_acceptance`
    - `useful_inference_can_support_positive_commitment_lead_time`
    - `right_censored_timeline_has_no_commitment_lead_time`
    - `innovative_valid_evidence_quality_monotone`
  - theorem boundary:
    - guarded commitment correctness, partial-history compatibility,
      propagated-demand non-interference, right-censoring, and innovative
      monotonicity are deterministic theorem-backed statements
    - positive commitment lead time is theorem-backed only under the explicit
      useful-inference, bounded-score, and guarded false-commitment assumption
      records reused from `Field/CodedDiffusionStrong.lean`
    - none of these theorems imply consensus, globally identical beliefs,
      arbitrary mobility optimality, or demand-created evidence
  - Rust alignment:
    - `crates/field/src/research.rs`
      - `AdditiveScoreStatistic`, `PartialHistoryWitness`,
        `commitment_lead_time_rounds`, and
        `ReceiverBeliefCompatibilitySummary::has_compatible_guarded_commitments`
        mirror the strong active belief theorem surface

- `Field/ActiveBeliefDefinitive.lean`
  - owns the definitive active belief theorem-closure vocabulary:
    - `DemandGuidedComparison` for same-budget active-versus-passive theorem
      surfaces
    - `DemandThresholdEfficiencyCertificate` for restricted useful-arrival
      threshold efficiency under explicit clean-model assumptions
    - `MergeableStatistic`, `GenericStatisticState`, and
      `GenericStatisticContribution` for generic mergeable-inference soundness
    - `CommitmentBeforeRecoveryBound` for finite-horizon commitment-before-full-recovery
      claims
    - `ReceiverSetCompatibility` for receiver-set compatibility and
      disagreement-bound metadata
    - `NearCriticalControllerBand` for achieved-pressure and potential-bound
      resource control
    - `AggregateCostComparison` for explicit aggregation cost assumptions
    - `BoundedStressBudget` for bounded duplicate, stale-demand, and withholding
      stress surfaces
    - `ObserverLeakageProjection` for measured observer-ambiguity bounds
  - completed theorem names:
    - `demand_guided_useful_arrivals_nonworse`
    - `demand_guided_uncertainty_nonworse`
    - `demand_guided_commitment_time_nonworse`
    - `demand_guided_quality_per_byte_nonworse`
    - `demand_guided_reaches_threshold_when_passive_reaches`
    - `demand_guided_threshold_with_no_more_useful_transmissions`
    - `generic_duplicate_preserves_statistic_state`
    - `generic_direct_statistic_decoding`
    - `generic_aggregate_preserves_statistic`
    - `commitment_before_full_recovery_lower_bound`
    - `commitment_before_recovery_bound_is_permille`
    - `receiver_set_compatibility_bounded`
    - `receiver_disagreement_permille_bounded`
    - `near_critical_controller_keeps_pressure_in_band`
    - `near_critical_controller_bounds_potential`
    - `valid_aggregate_cost_nonworse`
    - `aggregate_duplicate_preserves_generic_state`
    - `no_ledger_duplicate_can_change_result`
    - `non_associative_merge_order_counterexample`
    - `duplicate_spam_rank_safety`
    - `stale_demand_stress_cannot_validate_invalid_evidence`
    - `observer_leakage_permille_bounded`
    - `observer_ambiguity_preserves_hidden_fragments`
  - theorem boundary:
    - active demand improvement, commitment-before-full-recovery, receiver-set
      disagreement, near-critical control, aggregation cost, bounded stress, and
      observer leakage are theorem-backed only under their explicit assumption
      records
    - generic mergeable-statistic theorems provide the class boundary; small
      counterexamples show why contribution identity and merge laws are required
    - none of these theorems imply consensus, globally identical beliefs,
      arbitrary mobility optimality, demand-created evidence, formal privacy, or
      robustness against arbitrary adaptive adversaries
  - `crates/simulator/src/diffusion/core_experiment.rs`
    - theorem-assumption rows now include both the finite-horizon bound
      theorems, the deterministic strong active belief theorems, and the
      definitive theorem-closure rows for replay certificates, demand
      improvement, generic mergeable inference, commitment-before-recovery,
      compatibility, near-critical control, aggregation, stress, theorem-profile
      row soundness, and observer ambiguity.
      The report generator uses theorem-profile and bound-summary labels to
      distinguish deterministic rows from assumption-bearing rows.

- `Field/ActiveBeliefDefinitiveClosure.lean`
  - owns the definitive closure extension vocabulary:
    - `ReceiverDisagreementCause` and `ReceiverDisagreementExplanation` for
      replay-visible bounded disagreement causes
    - `RawUsefulReproductionPressure` for separate raw and independently useful
      reproduction pressure
    - `EffectiveIndependenceInferencePotential` and
      `EffectiveIndependencePotentialDrift` for useful-control potential with
      an explicit effective-independence deficit
  - completed theorem names:
    - `receiver_disagreement_has_replay_visible_cause`
    - `useful_reproduction_pressure_is_bounded_by_raw_pressure`
    - `useful_reproduction_pressure_in_achieved_band`
    - `effective_independence_potential_total_is_accounted_sum`
    - `effective_independence_potential_drift_bounded`
  - theorem boundary:
    - these are finite certificate theorems over replay-visible fields; they do
      not prove consensus, optimal active control, or arbitrary-trace
      convergence

- `Field/ActiveBeliefCertificates.lean`
  - owns the replay-certificate bridge vocabulary:
    - `ReceiverArrivalReplayCertificate`,
      `UsefulInferenceReplayCertificate`, and `ScoreTraceCertificate` for
      replay-checkable certificate bridges into finite-horizon assumption
      records
    - `DemandPolicyReplayCertificate` for matched active/passive replay rows
      that justify active-demand improvement assumptions
    - `NearCriticalControllerOpportunityCertificate` for replay-visible
      opportunity and capacity checks that justify achieved-band rows
    - `ActiveBeliefTheoremProfileReplayRow` for narrow Rust replay row
      theorem-profile metadata soundness
    - `BoundedStressReplayCertificate` for replay-visible bounded-stress
      inputs to guarded commitment bounds
  - completed theorem names:
    - `replay_certificate_implies_receiver_arrival_bound`
    - `replay_certificate_implies_useful_inference_arrival_bound`
    - `score_trace_certificate_implies_margin_guard`
    - `demand_policy_certificate_implies_useful_arrival_improvement`
    - `near_critical_controller_enters_band_under_opportunity_bounds`
    - `rust_replay_rows_sound_for_active_belief_theorem_profiles`
    - `bounded_stress_certificate_implies_guarded_commitment_bound`
  - theorem boundary:
    - replay-certificate bridge theorems prove that explicit artifact-facing
      certificate records imply existing assumption records; they do not prove
      arbitrary-trace mobility, margin, controller convergence, simulator
      correctness, privacy, or arbitrary-adversary robustness

- `Field/ActiveBeliefEndToEnd.lean`
  - owns the reduced end-to-end theorem vocabulary:
    - `ActiveBeliefTraceState`, `ActiveBeliefTraceEvent`, and
      `ActiveBeliefFiniteTrace` for finite replay semantics over accepted
      receiver rank and merged statistic state
    - `activeBeliefTraceStep` and `activeBeliefTraceFinalState` for the
      operational fold of replay-visible evidence events
    - `ActiveDemandPolicyValueModel` for equal-budget value-order assumptions
      that derive active-demand useful-arrival improvement
    - `RustReplayValidatorRow` for narrow validator metadata consumed by the
      theorem-profile and trace-certificate surfaces
  - completed theorem names:
    - `active_belief_trace_step_matches_plain_acceptance`
    - `active_belief_trace_soundness`
    - `active_demand_policy_improves_under_value_model`
    - `trace_validator_adequacy`
  - theorem boundary:
    - the end-to-end theorem proves the finite receiver state is the fold of the
      reduced replay events and that guarded commitment decodes from that
      audited folded statistic before full recovery when the finite trace is
      valid
    - the value-model theorem derives active-demand improvement from explicit
      value-order and equal-budget assumptions rather than assuming useful
      arrivals directly
    - validator adequacy proves exported validator metadata is sound input to
      the Lean theorem-profile surface; it is still not a proof of arbitrary
      simulator correctness or arbitrary temporal-network success

- `Field/ActiveBeliefDecisionSufficiency.lean`
  - owns the decision-first distributed error-correction vocabulary:
    - `StableDecisionBasinCertificate` for partial statistics that pass a guard,
      agree with the full statistic decision, and remain below reconstruction
      rank
    - `RecoveryAsDecisionCertificate` for exact k-of-n recovery as a special
      decision-sufficiency endpoint
    - `DemandBasinProgressCertificate` for demand value measured as progress
      toward a guarded decision basin rather than rank alone
    - `DistributedErrorCorrectionDecisionLimit` for the packaged decision-first
      correction limit
  - completed theorem names:
    - `stable_decision_basin_before_reconstruction`
    - `decision_sufficiency_strictly_weaker_than_reconstruction_example`
    - `exact_reconstruction_is_decision_sufficiency_special_case`
    - `bytes_to_decision_can_be_less_than_bytes_to_reconstruction`
    - `demand_value_targets_decision_basin_progress`
    - `nonstable_partial_decision_counterexample`
    - `distributed_error_correction_decision_limit`
  - theorem boundary:
    - the decision-limit theorem proves a scoped statement over audited
      mergeable statistics and explicit stable-basin certificates; it does not
      claim every partial statistic is safe, and the counterexample shows why
      basin stability is necessary

- `Field/TemporalIndependenceLimits.lean`
  - owns the temporal independence-limit vocabulary:
    - `TemporalContactDiversitySummary` for raw transmissions, raw copies,
      effective rank, contact entropy, lineages, bridge crossings, budgets,
      observability cost, raw reproduction, and effective reproduction
    - `effectiveReconstructable` for k-of-n reconstruction over effective rank
      rather than raw copy count
    - `InferenceEffectiveIndependenceCertificate` for direct-statistic and
      task-level effective independence over audited ledgers, receiver
      histories, lineage/contact diversity, and duplicate discounting
    - `IndependenceLimitedRecoveryBound` for the replay-facing permille bound
      corresponding to `P_recover(T) <= P(I_T >= k)`
    - `EffectiveReproductionSummary` for control over independent useful
      fragments rather than raw copies
    - `MatchedTemporalNetworkPair` for same-budget/same-raw-spread traces that
      separate by contact diversity
    - `CostTimeIndependenceBoundary` for the cost-time-independence triangle
    - `DistributedErrorCorrectionIndependenceLimit` for the final packaged
      temporal independence limit
  - completed theorem names:
    - `effective_rank_bounded_by_raw_copies`
    - `effective_rank_bounded_by_raw_transmissions`
    - `reconstruction_requires_effective_fragment_rank`
    - `effective_rank_reconstruction_suffices`
    - `effective_task_independence_bounded_by_raw_copies`
    - `effective_task_independence_bounded_by_raw_transmissions`
    - `effective_task_independence_connected_to_audit_fields`
    - `direct_statistic_commitment_requires_task_effective_guard`
    - `high_raw_spread_does_not_imply_task_effective_independence`
    - `exact_k_of_n_effective_guard_is_reconstruction_threshold`
    - `additive_anomaly_effective_guard_matches_commitment_guard`
    - `majority_threshold_effective_guard_matches_task_threshold`
    - `recovery_probability_bounded_by_effective_independence`
    - `many_copies_do_not_imply_many_independent_fragments`
    - `raw_reproduction_above_one_does_not_imply_reconstruction`
    - `same_budget_and_raw_spread_can_have_different_reconstruction`
    - `cost_time_independence_triangle_incompatibility`
    - `effective_reproduction_tracks_independent_useful_fragments`
    - `distributed_error_correction_independence_limit`
  - theorem boundary:
    - the independence-limit theorem proves a finite replay/certificate result:
      raw redundancy and raw reproduction are not enough unless the contact
      process certifies effective rank and task-effective independence. It does
      not prove an unconstrained temporal-network capacity theorem or derive
      stochastic contact entropy from arbitrary mobility traces.

- `Field/TemporalIndependenceCapacity.lean`
  - owns the temporal-capacity extension vocabulary:
    - `ContactEntropySummary` for finite contact-entropy and dispersion
      certificates
    - `TemporalGeneratorMatrixCertificate` for the finite generator-rank proxy
      induced by contact movement
    - `EntropyDispersionReconstructionBound` for deterministic
      entropy/dispersion reconstruction bounds
    - `TemporalContactCapacityCertificate` for narrow finite contact-process
      capacity over trace, budget, TTL, storage, and validity assumptions
    - `capacityAchievesReconstruction` and `capacityAchievesCommitment` for
      reconstruction and commitment reachability inside that certificate
    - `ReliabilityResourceAmbiguityBoundary` for the limit-triangle feasibility
      certificate
    - `MatchedEntropyNetworkPair` for same-raw-spread traces separated by
      entropy, dispersion, effective rank, and reconstruction outcome
    - `ErrorCorrectionLimitSketchStatus` for marking stronger stochastic
      sketches as proved, replay-validated, deferred, or removed
    - `stochasticCapacitySketchStatus` for the explicit deferred status of the
      stronger stochastic temporal-capacity theorem
  - completed theorem names:
    - `contact_entropy_and_dispersion_bounded_by_raw_activity`
    - `low_contact_entropy_can_coexist_with_high_transmission_count`
    - `effective_rank_bounded_by_temporal_generator_rank`
    - `duplicate_lineage_rows_do_not_increase_rank_proxy`
    - `reconstruction_bound_from_entropy_and_dispersion`
    - `temporal_contact_capacity_bounded_by_independent_arrivals`
    - `temporal_contact_capacity_monotone_in_budget`
    - `raw_contact_rate_increase_does_not_imply_capacity_increase`
    - `reliability_resource_ambiguity_triangle_incompatibility`
    - `raw_reproduction_above_one_does_not_imply_effective_reproduction_above_one`
    - `effective_reproduction_finite_horizon_bound`
    - `matched_networks_separate_by_entropy_and_effective_rank`
  - theorem boundary:
    - the capacity extension is a finite certificate surface. It does not
      assert stochastic capacity for arbitrary mobility processes, full
      network-coding linear algebra over unbounded temporal networks, or formal
      privacy from observer ambiguity.

### Rust Correspondence Freeze

The active belief theorem statements intended for the paper are frozen to the
following non-optimality, non-consensus claims:

- bounded first-class demand communication:
  `demand_bounded_by_entry_cap`, `demand_bounded_by_byte_cap`,
  `valid_demand_is_live`
- semantic separation of demand from evidence:
  `demand_message_carries_no_contribution`,
  `evidence_message_carries_contribution`,
  `demand_cannot_validate_invalid_evidence`,
  `demand_accepts_only_through_valid_evidence`,
  `demand_priority_does_not_change_acceptance`
- duplicate and stale-demand safety:
  `demand_duplicate_non_inflation`,
  `expired_demand_does_not_accept_invalid_evidence`
- replay metrics and compatibility:
  `commitment_lead_time_soundness`,
  `same_guarded_basin_compatible`,
  `compatible_commitments_have_same_hypothesis`
- mergeable-statistic decoding and strong active belief:
  `guarded_commitment_from_mergeable_statistic_correct`,
  `compatible_partial_histories_yield_compatible_commitments`,
  `demand_guided_statistic_acceptance_matches_plain_acceptance`,
  `propagated_demand_guided_statistic_acceptance_matches_plain_acceptance`,
  `useful_inference_can_support_positive_commitment_lead_time`,
  `right_censored_timeline_has_no_commitment_lead_time`,
  `innovative_valid_evidence_quality_monotone`

Rust/replay correspondence:

- `crates/field/src/research.rs`
  - `ActiveBeliefMessage` mirrors `ActiveMessage`; `DemandSummary` variants
    return an empty contribution slice, while `CodedEvidence` exposes
    `contribution_ledger_ids`.
  - `ActiveDemandSummary`, `ActiveDemandEntry`, `ActiveDemandSummaryInput`,
    and `ACTIVE_DEMAND_ENTRY_COUNT_MAX` mirror `DemandSummary`,
    `DemandEntry`, and boundedness obligations. Rust uses explicit
    `encoded_bytes`, `byte_cap`, `DurationMs`, `issued_at_tick`, and
    `expires_at_tick`; Lean abstracts byte accounting to the proof-facing cap.
  - `ReceiverIndexedBeliefState` and `ReceiverInferenceQualitySummary` mirror
    `ReceiverBeliefState` and `QualitySummary` for replay-visible
    receiver-indexed statistics.
  - `AdditiveScoreStatistic`, `PartialHistoryWitness`, and
    `commitment_lead_time_rounds` mirror the strong active belief theorem
    vocabulary over merged statistics, partial histories, and right-censored
    lead time.
  - `generate_active_demand_summary` mirrors demand generation from
    uncertainty, margin, missing contribution ids, and coverage gap. It only
    builds demand summaries; it does not mutate rank or evidence validity.
  - `record_active_evidence_arrival` mirrors `demandAwareAccept` through the
    ordinary contribution gate and preserves duplicate non-inflation.
  - `ActiveDemandPropagationMode::{None, LocalOnly, PiggybackedPeerDemand}`
    names the active-demand communication modes without adding transport or router
    semantics.
- `crates/simulator/src/diffusion/coded_inference.rs`
  - `CodedDemandSummaryEvent` is the replay row for demand emitted, demand
    received, demand satisfied, demand response lag, and stale-demand ignore
    counters.
  - `CodedInferenceReadinessSummary` exports commitment lead time, receiver
    agreement, belief divergence, collective uncertainty, evidence overlap,
    demand satisfaction, and active reproduction metrics.
- `crates/simulator/src/diffusion/local_policy/*.rs`
  - `demand_value` is an explicit priority term in `LocalPolicyScoreInput` and
    `LocalPolicyScoreBreakdown`. The `NoDemandValue` ablation demonstrates
    demand affects allocation priority, not evidence validity or contribution
    identity.
- `crates/simulator/src/diffusion/core_experiment.rs`
  - `ActiveBeliefExperimentArtifacts` exports the active belief grid,
    active-versus-passive rows, no-central-encoder panel, second compact
    mergeable task row, recoding frontier, and bounded robustness rows.
  - `ActiveRobustnessRow.false_confidence_permille` is the replay-visible
    stress-test rejection counter. It is an experiment metric, not a Lean
    theorem of adversarial robustness.

Paper non-claims remain in force: no theorem here asserts consensus, common
knowledge, optimal active policy, privacy, globally identical receiver beliefs,
or robustness against arbitrary adaptive adversaries. Telltale is an
implementation/protocol-support dependency for Jacquard; the active belief
diffusion result is stated over the reduced proof-facing objects above.

### Definitive Closure Correspondence

The closure pass added the following proof-facing objects and replay
correspondence entries. Objects without a direct Rust type are proof-only
certificates that bind existing replay fields into a theorem boundary.

| Lean object | Rust/replay counterpart | Role |
| --- | --- | --- |
| `InferenceEffectiveIndependenceCertificate` | Existing effective-rank, receiver-rank, contribution-overlap, duplicate, contact-diversity, and theorem-profile rows in `ActiveBeliefExperimentArtifacts` | Proof-facing task-independence certificate for direct statistic decoding. |
| `validInferenceEffectiveIndependenceCertificate` | Artifact sanity and theorem-profile rows that bound rank, raw copies, raw transmissions, and duplicate pressure | Validity boundary for task-level effective independence. |
| `taskEffectiveEvidenceGuard` | Commitment guard fields and receiver evidence-count rows | Evidence guard over effective task independence, not raw copy count. |
| `effective_task_independence_bounded_by_raw_copies` | Raw-copy and effective-rank artifact rows | Upper-bound theorem. |
| `effective_task_independence_bounded_by_raw_transmissions` | Forwarding-event and effective-rank artifact rows | Upper-bound theorem. |
| `effective_task_independence_connected_to_audit_fields` | Receiver accepted contribution ids, evidence overlap, duplicate count, lineage/contact-diversity rows | Connects task independence to ledgers, receiver histories, lineage/contact diversity, and duplicate discounting. |
| `direct_statistic_commitment_requires_task_effective_guard` | Guarded commitment and evidence-count rows | Commitment guard theorem. |
| `monoid_homomorphism_preserves_decision_quality_under_partial_accumulation` | Task-family interface rows plus quality summaries | Partial accumulation preserves task quality only for certified mergeable-statistic quality maps. |
| `high_raw_spread_does_not_imply_task_effective_independence` | Raw-spread/effective-rank counterexample rows | Negative witness: raw spread is insufficient. |
| `exactReconstructionEffectiveIndependence` | k-of-n threshold rows | Exact recovery instance. |
| `additiveAnomalyEffectiveIndependence` | anomaly score-vector rows | Additive anomaly instance. |
| `majorityThresholdEffectiveIndependence` | majority/threshold second-task rows | Second compact mergeable-task instance. |
| `demandSummaryFromReceiverState` | `generate_active_demand_summary`, `ActiveDemandSummary`, `ReceiverIndexedBeliefState` | Proof-facing deterministic demand constructor from audited receiver state. |
| `demand_summary_from_receiver_state_valid` | demand caps, encoded bytes, and TTL checks in `ActiveDemandSummary::try_new` | Boundedness theorem for state-derived demand. |
| `demand_summary_from_receiver_state_has_canonical_singleton_order` | canonical demand-entry ordering tests in `crates/field/src/research.rs` | Proof-facing canonical order for the reduced constructor. |
| `demand_guided_reaches_threshold_when_passive_reaches` | active-versus-passive equal-budget rows | Restricted useful-arrival threshold theorem under demand-quality assumption. |
| `DemandThresholdEfficiencyCertificate` | active-versus-passive rows plus theorem-assumption metadata | Clean-model threshold-efficiency certificate. |
| `demand_guided_threshold_with_no_more_useful_transmissions` | active demand ablation rows | Restricted theorem for active threshold success under no-more-useful-transmissions assumption. |
| `ReceiverDisagreementCause` | disagreement, stale demand, bias, duplicate, and evidence-delta fields in active validation rows | Bounded disagreement-cause vocabulary. |
| `ReceiverDisagreementExplanation` | receiver-indexed validation rows and active grid rows | Replay-visible disagreement explanation certificate. |
| `receiver_disagreement_has_replay_visible_cause` | active multi-receiver validation rows | Theorem that disagreement causes are bounded and replay visible. |
| `RawUsefulReproductionPressure` | `measured_r_est_raw_permille`, `measured_r_est_useful_permille`, active forwarding opportunities, innovative/duplicate arrivals | Split achieved raw and useful reproduction pressure. |
| `useful_reproduction_pressure_is_bounded_by_raw_pressure` | raw/useful reproduction fields in active rows | Useful spread is bounded by raw spread. |
| `useful_reproduction_pressure_in_achieved_band` | useful reproduction pressure and target-band rows | Achieved useful-pressure band theorem. |
| `EffectiveIndependenceInferencePotential` | near-critical potential rows plus effective-rank/independence rows | Inference potential with explicit effective-independence deficit. |
| `effective_independence_potential_total_is_accounted_sum` | near-critical potential named-term exports | Potential accounting theorem. |
| `EffectiveIndependencePotentialDrift` | near-critical controller and potential rows | Bounded drift certificate. |
| `effective_independence_potential_drift_bounded` | operating-region diagram and potential rows | Useful-control drift theorem. |
| `ContactEntropySummary` | contact entropy, carrier lineage, bridge-crossing, forwarding-event, and dispersion replay fields | Finite contact-entropy and dispersion certificate. |
| `contact_entropy_and_dispersion_bounded_by_raw_activity` | contact/diversity theorem-profile rows | Entropy and dispersion are bounded by raw activity. |
| `low_contact_entropy_can_coexist_with_high_transmission_count` | high-contact low-diversity witness rows | Negative witness for contact rate versus entropy. |
| `TemporalGeneratorMatrixCertificate` | effective-rank and lineage-diversity replay proxy rows | Finite generator-rank proxy for contact-induced code structure. |
| `effective_rank_bounded_by_temporal_generator_rank` | effective-rank theorem-profile rows | Effective rank is bounded by generator-rank proxy. |
| `duplicate_lineage_rows_do_not_increase_rank_proxy` | duplicate-lineage and duplicate-arrival rows | Duplicate lineage rows do not certify rank growth. |
| `EntropyDispersionReconstructionBound` | contact-entropy, dispersion, byte-budget, time-horizon, and rank rows | Reconstruction bound from entropy/dispersion certificate. |
| `reconstruction_bound_from_entropy_and_dispersion` | theorem-assumption rows for entropy/dispersion bounds | Effective reconstruction bounded by the entropy/dispersion certificate. |
| `TemporalContactCapacityCertificate` | finite trace, byte-budget, storage-cap, contact-diversity, and TTL rows | Narrow finite temporal-contact capacity certificate. |
| `temporal_contact_capacity_bounded_by_independent_arrivals` | temporal capacity theorem-profile rows | Capacity bounded by independent arrivals. |
| `temporal_contact_capacity_monotone_in_budget` | capacity sweep rows | Monotonicity over certified independent-arrival capacity. |
| `raw_contact_rate_increase_does_not_imply_capacity_increase` | raw-contact/effective-capacity witness rows | Negative witness for contact rate versus capacity. |
| `ReliabilityResourceAmbiguityBoundary` | reliability, cost, and observer-ambiguity proxy rows | Limit-triangle certificate. |
| `reliability_resource_ambiguity_triangle_incompatibility` | observer ambiguity and cost frontier rows | Low cost, high reliability, and high ambiguity cannot all be certified below the boundary. |
| `raw_reproduction_above_one_does_not_imply_effective_reproduction_above_one` | raw/useful reproduction rows | Raw supercritical spread does not imply useful supercritical spread. |
| `effective_reproduction_finite_horizon_bound` | raw/useful reproduction pressure fields | Achieved useful reproduction is explicit and bounded by raw reproduction. |
| `MatchedEntropyNetworkPair` | matched contact-diversity scenario rows | Matched-network theorem witness with entropy and dispersion. |
| `matched_networks_separate_by_entropy_and_effective_rank` | matched correlated/diverse replay rows | Equal raw spread can separate by entropy, dispersion, rank, and recovery. |
| `ErrorCorrectionLimitSketchStatus` | paper-boundary status rows and work notes | Tracks proved, replay-validated, deferred, or removed theorem sketches. |
| `stochasticCapacitySketchStatus` | paper-boundary status rows and `work/error_correction_limits.md` | Marks universal stochastic temporal-contact capacity as deferred, not proved by the finite certificate. |

New Rust/replay fields:

- `ActiveBeliefGridRow::measured_r_est_raw_permille`
- `ActiveBeliefGridRow::measured_r_est_useful_permille`
- `ActiveVersusPassiveRow::measured_r_est_raw_permille`
- `ActiveVersusPassiveRow::measured_r_est_useful_permille`

The existing `measured_r_est_permille` remains the useful reproduction value
for backward compatibility with earlier artifacts. New paper text should prefer
the raw/useful split when discussing near-critical control.

### Theorem Dependency Table

| Paper claim | Lean object | Depends on | Rust/replay target |
| --- | --- | --- | --- |
| Demand is bounded first-class communication data | `DemandSummary`, `validDemandSummary`, `ActiveMessage.demand` | entry, byte, and ttl caps | demand emitted/received replay rows with caps |
| Demand carries no contribution identity | `demand_message_carries_no_contribution` | `ActiveMessage.contributionId?` | demand rows with no contribution id field or an explicit empty contribution slot |
| Evidence carries audited contribution identity | `evidence_message_carries_contribution` | `EvidenceProposal`, `ContributionId` | evidence rows with contribution id and validity fields |
| Demand cannot validate invalid evidence | `demand_cannot_validate_invalid_evidence` | `demandAwareAccept`, `EvidenceProposal.validEvidence` | invalid evidence rejection counters under active policy |
| Demand-driven duplicates do not inflate rank | `demand_duplicate_non_inflation` | `acceptContribution`, `ReceiverRank` | duplicate arrival rows and receiver-rank stability checks |
| Stale demand cannot justify invalid evidence | `expired_demand_does_not_accept_invalid_evidence` | `expiredDemandSummary`, `demandAwareAccept` | stale-demand ignored/rejected replay rows |
| Commitment lead time is a replay metric | `commitment_lead_time_soundness` | `CommitmentTimeline` | commitment and full-recovery event rows |
| Positive lead time under explicit useful-inference assumptions | `useful_inference_can_support_positive_commitment_lead_time` | `LeadTimeWitness` + strong assumption rows | theorem-profile rows + commitment lead-time summaries |
| Receiver-arrival replay rows imply arrival assumptions | `replay_certificate_implies_receiver_arrival_bound` | `ReceiverArrivalReplayCertificate` | theorem-assumption rows + path validation rank floors |
| Useful-inference replay rows imply useful-arrival assumptions | `replay_certificate_implies_useful_inference_arrival_bound` | `UsefulInferenceReplayCertificate` | theorem-assumption rows + useful contribution floors |
| Score replay rows imply margin and guard assumptions | `score_trace_certificate_implies_margin_guard` | `ScoreTraceCertificate` | anomaly-localization margin and evidence rows |
| Demand-policy replay rows imply active improvement assumptions | `demand_policy_certificate_implies_useful_arrival_improvement` | `DemandPolicyReplayCertificate` | active-versus-passive matched ablation rows |
| Bounded demand variance deflection is replay-visible | `demand_induced_allocation_variance_deflection_bounded` | `DemandVarianceDeflectionCertificate` | demand-byte budget and variance-deflection rows |
| Finite active-belief trace folds to audited receiver state | `active_belief_trace_soundness` | `ActiveBeliefFiniteTrace` | replay event fold, no-static-path, and commitment-before-recovery rows |
| Active demand improves under value-order model | `active_demand_policy_improves_under_value_model` | `ActiveDemandPolicyValueModel` | active-versus-passive value and equal-budget rows |
| Stable decision can precede reconstruction | `stable_decision_basin_before_reconstruction` | `StableDecisionBasinCertificate` | commitment-before-recovery and rank-below-quorum rows |
| Decision sufficiency is strictly weaker than reconstruction in a finite witness | `decision_sufficiency_strictly_weaker_than_reconstruction_example` | concrete certificate | theorem-assumption table witness row |
| Exact recovery is the threshold special case | `exact_reconstruction_is_decision_sufficiency_special_case` | `RecoveryAsDecisionCertificate` | threshold task rows |
| Decision byte cost can be below reconstruction byte cost | `bytes_to_decision_can_be_less_than_bytes_to_reconstruction` | `StableDecisionBasinCertificate` | bytes-at-commitment versus recovery-cost rows |
| Demand targets decision-basin progress | `demand_value_targets_decision_basin_progress` | `DemandBasinProgressCertificate` | demand-value and uncertainty rows |
| Non-stable partial decisions can disagree with full statistic | `nonstable_partial_decision_counterexample` | concrete statistic pair | non-claim boundary |
| Decision-first error correction limit | `distributed_error_correction_decision_limit` | `DistributedErrorCorrectionDecisionLimit` | theorem-profile rows for decision-before-recovery |
| Effective rank is bounded by raw copies | `effective_rank_bounded_by_raw_copies` | `TemporalContactDiversitySummary` | effective-rank and raw-copy rows |
| Effective rank is bounded by raw transmissions | `effective_rank_bounded_by_raw_transmissions` | `TemporalContactDiversitySummary` | effective-rank and forwarding-event rows |
| Reconstruction requires effective fragment rank | `reconstruction_requires_effective_fragment_rank` | `effectiveReconstructable` | rank-eff threshold rows |
| Effective rank suffices for the reconstruction threshold | `effective_rank_reconstruction_suffices` | `effectiveReconstructable` | rank-eff threshold rows |
| Recovery probability is bounded by effective independence probability | `recovery_probability_bounded_by_effective_independence` | `IndependenceLimitedRecoveryBound` | theorem-assumption permille rows |
| Many copies do not imply many independent fragments | `many_copies_do_not_imply_many_independent_fragments` | concrete low-diversity witness | raw-copy versus rank-eff counterexample |
| Raw reproduction above one is insufficient | `raw_reproduction_above_one_does_not_imply_reconstruction` | concrete low-diversity witness | R-versus-rank-eff counterexample |
| Same budget and raw spread can reconstruct differently | `same_budget_and_raw_spread_can_have_different_reconstruction` | `MatchedTemporalNetworkPair` | matched contact-diversity scenarios |
| Low cost, fast time, and high independence are jointly constrained | `cost_time_independence_triangle_incompatibility` | `CostTimeIndependenceBoundary` | limit-triangle rows |
| Effective reproduction tracks independent useful fragments | `effective_reproduction_tracks_independent_useful_fragments` | `EffectiveReproductionSummary` | effective-R controller rows |
| Temporal independence limits distributed error correction | `distributed_error_correction_independence_limit` | `DistributedErrorCorrectionIndependenceLimit` | temporal independence theorem-profile rows |
| Trace-class temporal contact implies the independence limit | `trace_class_temporal_contact_implies_independence_limit` | `TraceClassIndependenceCertificate` | Path A trace-class theorem-profile rows |
| Guarded commitment decodes the merged statistic | `guarded_commitment_from_mergeable_statistic_correct` | `AdditiveScoreStatistic` | theorem-profile rows + guarded commitment summaries |
| Compatible commitments do not require identical histories | `compatible_partial_histories_yield_compatible_commitments` | `PartialHistoryWitness` | receiver compatibility summaries |
| Demand changes control, not merged-statistic semantics | `demand_guided_statistic_acceptance_matches_plain_acceptance` | `AcceptedStatisticContribution` | host/bridge demand replay rows |
| Innovative evidence sharpens the theorem-facing quality order | `innovative_valid_evidence_quality_monotone` | `ReceiverQualityOrder` | receiver quality summaries |
| Compatible decisions are guarded local decisions | `same_guarded_basin_compatible`, `compatible_commitments_have_same_hypothesis` | `GuardedCommitment` | receiver agreement rows over committed hypotheses |
| Uncertified stochastic receiver-arrival target remains measured | `receiver_arrival_stochastic_bound_is_narrowed` | `TheoremTargetStatus` | multi-seed validation rows outside certificate scope |
| Uncertified margin concentration target remains measured | `anomaly_margin_concentration_is_narrowed` | `TheoremTargetStatus` | anomaly-localization rows outside certificate scope |
| Inference potential uses named deterministic terms | `inference_potential_total_is_accounted_sum`, `inference_progress_uncertainty_nonincreasing` | `InferencePotential` | near-critical potential rows |
| Controller replay rows imply achieved-band assumptions | `near_critical_controller_enters_band_under_opportunity_bounds` | `NearCriticalControllerOpportunityCertificate` | operating-region band and budget rows |
| Rust theorem-profile rows are narrow proof metadata | `rust_replay_rows_sound_for_active_belief_theorem_profiles` | `ActiveBeliefTheoremProfileReplayRow` | theorem-assumption CSV metadata |
| Rust replay validator rows are adequate for trace/profile inputs | `trace_validator_adequacy` | `RustReplayValidatorRow` | theorem-assumption and trace-validation metadata |
| Bounded stress replay rows imply stress budget assumptions | `bounded_stress_certificate_implies_guarded_commitment_bound` | `BoundedStressReplayCertificate` | robustness duplicate, stale-demand, and withholding rows |
| Bounded-Sybil stress degrades gracefully under signed identities | `bounded_sybil_graceful_degradation` | `BoundedSybilReplayCertificate` | byzantine-fragment injection and malicious-identity ceiling rows |
| Second task is mergeable and duplicate-safe | `majority_duplicate_non_inflation`, `majority_positive_innovative_increases_vote_count` | `MajorityThresholdState` | majority-threshold second task rows |

### Active Belief Non-Claim Note

Active belief diffusion exchanges two bounded replay-visible message classes:
coded evidence and demand summaries. They are symmetric as communication
objects, but not semantically symmetric. Evidence can carry audited contribution
identity into the mergeable statistic. Demand can describe uncertainty and shape
priority, custody, recoding, and allocation. Demand cannot validate evidence,
create contribution identity, change merge semantics, or directly change a
belief statistic. The active belief theorem pack also does not claim consensus,
common knowledge, globally identical receiver beliefs, active-policy optimality,
formal privacy, or robustness against arbitrary adversaries.

## Legacy Route/Corridor Baseline Packs

These packs remain in the repository for comparison, regression, and proof-scaffold reuse, but they are no longer the active research theorem path:

- `Field/Router/*`
  - legacy router-owned publication, admission, installation, lifecycle, selector, canonical-route, cost, optimality, probabilistic, and resilience stack
- `Field/Search/*`
  - legacy proof-facing private route-search boundary
- `Field/Quality/*`
  - legacy route-view comparison and support-only refinement stack
- route/canonical portions of `Field/Adequacy/*` and `Field/System/*`
  - reusable only after conversion to reconstruction-facing runtime projection, fragment movement, or observer-projection statements

## Local Model

- `Field/Model/API.lean`
  - retained for coded diffusion as the reduced local controller model; semantic state vocabulary, explicit `ReducedBeliefSummary` reduction boundary, explicit `LocalOrderParameter` vocabulary, abstract round-step operations, and boundedness/harmony laws
- `Field/Model/Instance.lean`
  - first bounded concrete realization, structural theorems, temporal theorems, the Bayesian posterior companion view, a `LocalState` that stores `ReducedBeliefSummary` and `LocalOrderParameter` explicitly, explicit control-fusion from the stored summary into mean-field state, and regime classification over the stored order-parameter surface
- `Field/Model/Refinement.lean`
  - reduction-preservation, order-parameter preservation, stored-summary / stored-order-parameter chain theorems, sufficiency, conservativity, boundedness/monotonicity, and exogenous-control-dependence theorems for the controller-facing summary, plus the composed-round honesty/refinement pack
- `Field/Model/Decision.lean`
  - one-step finite exploration / decision procedure over a small evidence alphabet

## Information Layer

- `Field/Information/API.lean`
  - retained for coded diffusion as the observer ambiguity layer; abstract probability-simplex style normalization and information-theoretic operations over `FiniteBelief`
- `Field/Information/Instance.lean`
  - first concrete probability-simplex belief object, weight-normalized distribution, and entropy/mass theorems
- `Field/Information/Probabilistic.lean`
  - finite probabilistic route-hypothesis space, retained aggregate-mass helpers, and public-macrostate/blindness lemmas showing how the current public projection forgets latent quality/reliability structure; the controller-facing reduced summary still lives separately in `Field/Model/*`
- `Field/Information/Bayesian.lean`
  - Bayesian priors, factorized likelihoods, normalized posterior update, support/fallback theorems, and explicit boundary markers for correlated regimes outside the current factorized model
- `Field/Information/Calibration.lean`
  - confidence-threshold, posterior-probability, expected-utility, and regret-interpretation targets, plus decision-validity theorems, trusted explicit-observation soundness, public-projection distortion bounds, and an explicit correlated-regime calibration non-claim
- `Field/Information/Blindness.lean`
  - field-side information-cost / blindness bridge over the reduction-to-public-observer chain, including reduction-level erasure theorems, public-macrostate erasure, and aggregate-mass macrostate stability facts
- `Field/Information/Quantitative.lean`
  - L1 belief distance, small reduced-summary aggregate-gap objects, and first quantitative lemmas connecting posterior aggregate differences to reduction-level differences

## Private Protocol

- `Field/Protocol/API.lean`
  - retained for coded diffusion as bounded summary exchange and fragment-control coordination; reduced protocol roles, labels, machine state, global choreography, abstract projection/step/export laws
- `Field/Protocol/Boundary.lean`
  - thin boundary-facing import surface exposing the protocol API plus the current reduced instance for higher-layer boundary modules
- `Field/Protocol/Instance.lean`
  - first reduced summary-exchange instance
- `Field/Protocol/Bridge.lean`
  - Telltale-shaped reduced protocol-machine fragment and replay/observer bridge
- `Field/Protocol/Conservation.lean`
  - field-side conservation pack for evidence, authority, and replay-equivalent fragment traces, with direct-family instantiations kept separate from remaining local glue
- `Field/Protocol/Coherence.lean`
  - reduced updated-edge / incident-edge / unrelated-edge coherence lemmas
- `Field/Protocol/ReceiveRefinement.lean`
  - first typed receive-refinement hook aligned to `Consume` / subtype-replacement shape
- `Field/Protocol/Fixtures.lean`
  - proof-facing concrete summary/ack fixtures, fragment-trace observer-projection agreement, and fixed-participant/supported-reconfiguration examples
- `Field/Protocol/Closure.lean`
  - final reduced protocol-boundary theorem pack covering family alignment, receive-refinement witness closure, and fixed-participant/observational-reconfiguration closure
- `Field/Protocol/Reconfiguration.lean`
  - reduced protocol reconfiguration vocabulary covering owner transfer, checkpoint/restore, and continuation shift under a fixed participant set

## Boundary And Adequacy

- `Field/Model/Boundary.lean`
  - protocol/controller boundary from protocol exports and traces, with no runtime-artifact ownership
- `Field/Adequacy/API.lean`
  - abstract Rust-runtime artifact boundary, reduced router-facing runtime projection, runtime/search linkage metadata, reduced probabilistic slice, and reduced runtime-to-trace simulation witness
- `Field/Adequacy/Runtime.lean`
  - reduced runtime state, one-step runtime execution semantics, artifact extraction from runtime states/steps, and state-level adequacy/admission preservation lemmas
- `Field/Adequacy/Canonical.lean`
  - runtime-to-canonical refinement theorems connecting extracted runtime lifecycle routes to the system/router canonical selector under an explicit alignment boundary
- `Field/Adequacy/Cost.lean`
  - runtime/system cost-preservation theorems for projected artifacts, including exact preservation of canonical-search input, input size, search space, and search work units under the reduced runtime projection
- `Field/Adequacy/Optimality.lean`
  - projected-runtime budgeted-optimality theorems showing exact canonical agreement and zero regret once the reduced search budget covers the projected canonical-search surface
- `Field/Adequacy/Projection.lean`
  - reduced runtime artifact projection generated from `systemStep`, admission/honesty lemmas for that projection, and stronger runtime/system canonical refinement theorems with no extra alignment hypothesis
- `Field/Adequacy/Probabilistic.lean`
  - leading-evidence posterior extraction from runtime artifacts, runtime/trace confidence-threshold preservation, min-regret decision preservation, expected-utility order preservation, decision-relevant completeness for the reduced probabilistic projection, and an explicit erased-tail non-claim for the current reduced runtime view
- `Field/Adequacy/Search.lean`
  - reduced search projection, runtime-search adequacy object, optional reduced protocol reconfiguration, search-projection extraction functions, canonical-route refinement over quiescent runtime-search bundles, and negative-boundary theorems keeping router truth runtime-owned
- `Field/Adequacy/Refinement.lean`
  - runtime-state / system-state refinement relation, stuttering preservation of that relation under reduced runtime steps, and quiescent runtime-state consequences for canonical outcomes and first safety-preservation theorems; the semantic runtime-state object stays distinct from theorem-pack packaging and fixtures
- `Field/Adequacy/Safety.lean`
  - runtime/system reduction-soundness results for support conservativity, no false explicit-path promotion, no route creation from silence, admissible lifecycle origin, and quiescent observational equivalence
- `Field/Adequacy/Fixtures.lean`
  - proof-facing reduced runtime fixture cases covering canonical support selection, stronger router-selection tie handling, empty-runtime silence, one explicit non-claim scenario, and a small fixture-generation path from runtime artifacts or projected system states into proof-facing fixture objects
- `Field/Adequacy/ReplayFixtures.lean`
  - reduced replay-derived fixture vocabulary mirroring the maintained Rust replay export surface across search projection, protocol reconfiguration, runtime linkage, and recovery outcome scenarios
- `Field/Adequacy/ProbabilisticFixtures.lean`
  - proof-facing probabilistic fixtures covering explicit-evidence posterior support, correlated-evidence boundary marking, miscalibrated-likelihood divergence, and a sparse-evidence confidence guardrail
- `Field/Adequacy/Instance.lean`
  - first concrete runtime extraction, execution-level observational trace theorem, reduced simulation theorem, router-projection honesty facts, and evidence-agreement theorems
- `Field/AssumptionCore.lean`
  - proof-contract vocabulary, default/strengthened contract builders, and explicit convergence/resilience/search profile-family accessors over semantic, protocol-envelope, runtime-envelope, transport, participation, refinement, budget, and regime-profile assumption families
- `Field/AssumptionTheorems.lean`
  - theorem packaging layer deriving adequacy, quality, canonical-router, runtime-canonical, runtime-state execution refinement, and resilience-boundary consequences from the shared proof-contract vocabulary

## Network And Router

- `Field/Network/API.lean`
  - finite node/destination vocabulary, synchronous round buffer, delivered-message view, and local-harmony lift
- `Field/Network/Safety.lean`
  - first reduced network safety theorems connecting local honesty to publication, admission, and installation
- `Field/Router/Publication.lean`
  - router-facing publication candidates and publication honesty / well-formedness theorems
- `Field/Router/Selector.lean`
  - shared selector-family abstraction for lifecycle-route selection, covering candidate domain, eligibility filtering, fold-based best-route extraction, explicit selector-semantics metadata, explicit search execution-policy vocabulary, and a posture-to-execution-policy mapping that preserves selector semantics
- `Field/Router/Admission.lean`
  - reduced observed/admitted/rejected boundary and first admission conservativity theorems
- `Field/Router/Installation.lean`
  - minimal canonical installed-route object and installation honesty theorems
- `Field/Router/Lifecycle.lean`
  - reduced observed/admitted/installed/withdrawn/expired/refreshed lifecycle object plus maintenance and conservativity theorems
- `Field/Router/Canonical.lean`
  - router-owned destination-local canonical support selector over lifecycle routes, shared selector-family wrappers, support-best, eligibility, destination-scope containment, unique-eligible selection, a destination-local sparse-scaling theorem for off-destination route growth, a threshold discontinuity example, and threshold-emergence/disappearance theorems for canonical route truth
- `Field/Router/CanonicalStrong.lean`
  - stronger router-owned support-then-hop-then-stable selector over eligible lifecycle routes, plus shared selector-family wrappers and membership/eligibility theorems for the stronger canonical surface
- `Field/Router/Cost.lean`
  - proof-facing linear search-cost model for the canonical selector, including worst-case, incremental, stable-input, search-space, and maintenance-invariance bounds
- `Field/Router/Optimality.lean`
  - budgeted support-only canonical search, explicit support-regret vocabulary, anytime monotonicity, deadline-safety, and threshold-region theorems for the current router-owned objective
- `Field/Router/Probabilistic.lean`
  - router-owned confidence-threshold decision semantics over posterior belief, secondary posterior expectation / cost / risk / regret objects, threshold admissibility, dominance-monotonicity theorems, and explicit non-claim theorems separating posterior truth from support ranking and exported route views
- `Field/Router/Resilience.lean`
  - first participation-fault vocabulary, silence-only dropout budget, surviving-route projection, bounded-dropout support-stability theorems, and an explicit dishonest-publication non-claim

## Search Boundary

- `Field/Search/API.lean`
  - proof-facing reduced search boundary covering objective-to-query mapping, snapshot identity, execution-policy vocabulary, selected-result shape, reconfiguration metadata, and first replay-style lemmas

## Retention Boundary

- `Field/Retention/API.lean`
  - retained for coded diffusion as fragment custody and bounded holding policy; reduced payload-token, retention-policy input, retention-state, and abstract retention-step vocabulary plus boundary law bundles
- `Field/Retention/Instance.lean`
  - first bounded concrete retention instance with token aging, retain/carry/forward/drop policy, and executable state transitions
- `Field/Retention/Refinement.lean`
  - separation from local posterior/publication/canonical-route truth plus custody-conservation and forwarding-admissibility theorems
- `Field/Retention/Fixtures.lean`
  - proof-facing reduced retention scenarios covering retain, forward, drop, and checkpoint-restore cases

## Async And System Layers

- `Field/Async/API.lean`
  - retained for coded diffusion as delayed fragment delivery and forwarding; reduced async envelopes, explicit delay/retry/loss assumptions, queue stepping, ready-message view, and observer view
- `Field/Async/Safety.lean`
  - first async publication-safety theorems and queue-drain facts connecting the async layer back to local honesty
- `Field/Async/Transport.lean`
  - transport lifecycle lemmas for retry/delivery/drop behavior, publication injection, and the reliable-immediate refinement to the synchronous publication model
- `Field/Async/Bounded.lean`
  - one broader bounded-delay/bounded-retry regime, queue-growth and drain-after-transport bounds, ready-count bounds, no-strengthening theorems for existing in-flight claims, and one-retry-cycle fairness theorems for retry-eligible envelopes
- `Field/System/Statistics.lean`
  - aggregate local-support summaries and in-flight support-mass bounds over the async layer
- `Field/System/Bounded.lean`
  - system-facing queue and lifecycle-cardinality bounds for the broader async regime, plus source-projection preservation theorems, an explicit congestion/loss backlog budget, a proof-facing per-step work-unit bound, a one-retry-cycle queue-drain theorem under no-fresh-publication retry-only backlog assumptions, one-retry-cycle processing fairness for admissible retry-eligible updates, a first single-loss canonical-support stability theorem, a threshold-1 redundancy theorem for recovered support-dominating updates, a first graceful-degradation envelope, explicit intermittent-loss recovery and no-oscillation theorems after the recovery threshold, invalid-update withdrawal safety after retry recovery, and queue-clear recovery aliases back to the reliable-immediate canonical/convergence theorems
- `Field/System/Cost.lean`
  - proof-facing compute, communication, queue, and storage budget model for one reduced `systemStep`, including next-state preservation under the explicit transport-volume budget, stable communication/transport volume under the reliable-immediate fixed-point regime, amortized maintenance invariance, per-destination storage bounds, local/linear computability, max-bottleneck characterization, and a transport-derived graceful-resource-degradation theorem
- `Field/System/Optimality.lean`
  - system-facing wrappers for the budgeted support-only objective, including exact-within-budget, anytime monotonicity, deadline safety against the full optimum, route-view sufficiency, dominance preservation, no-rank-inversion, and threshold-region theorems
- `Field/System/Boundary.lean`
  - thin system-level assumption-boundary summary above the async/runtime stack, including projected-information order-insensitivity unlocks and explicit reliable-immediate fixed-point boundaries
- `Field/System/EndToEnd.lean`
  - reduced end-to-end state and step relation combining async transport, router lifecycle installation, and lifecycle maintenance, plus first safety/observer lemmas
- `Field/System/Convergence.lean`
  - reduced reliable-immediate fixed-point and no-spontaneous-promotion theorems over iterated end-to-end steps, plus a profile-indexed convergence interface separating local quantitative versus distributed/profile claims
- `Field/System/Canonical.lean`
  - system-facing refinement theorems connecting `supportDominance` winners to the router-owned canonical selector, plus shared selector-family wrappers, underconnected and unique-eligible sparse cases, thresholded canonical-support theorems, an explicit critical-threshold boundary, canonical support/knowledge conservativity for winners, a threshold-1 vanishing-support limit, reliable-immediate stability, global support-optimum packaging, one-step recovery and bounded-convergence theorems, and a no-oscillation theorem for the canonical system route in the current reliable-immediate bounded-delay corner
- `Field/System/Probabilistic.lean`
  - reduced probabilistic evidence-flow semantics over async envelopes and lifecycle routes, delayed/lossy/repeated/correlated observation vocabulary, message-to-observation update lemmas, stable-evidence posterior-choice preservation, bounded dropout-degradation and sparse-evidence guardrail theorems, and a system theorem connecting produced explicit candidates back to positive Bayesian explicit-path mass under the clean async regime
- `Field/System/Calibration.lean`
  - system-facing soundness theorem showing an explicit posterior decision on a produced candidate implies positive latent explicit-path mass in the reduced probabilistic state
- `Field/System/CanonicalStrong.lean`
  - system-facing stronger router selector based on support-then-hop-then-stable lifecycle choice, plus reliable-immediate stability and basic membership/eligibility theorems
- `Field/System/Resilience.lean`
  - system-facing bounded-dropout and bounded-non-participation stabilization/degradation theorems connecting reduced participation loss to canonical support behavior under the clean async regime, including reduced participation-cut and unique-bridge disappearance theorems
- `Field/System/Retention.lean`
  - system-facing retention/custody bridge above the async/runtime layer, including silence, no-delivery-without-custody, bounded retention work, and non-strengthening theorems for retained payloads
- `Field/Quality/API.lean`
  - reduced route-comparison views, admissibility rules, objective vocabulary, pairwise comparison objects, destination-filtered best-view selection, and maintenance-idempotence facts for exported route views
- `Field/Quality/Reference.lean`
  - reference admissibility and support-best semantics over exported route views, plus a destination-filtered support-only reference selector
- `Field/Quality/Refinement.lean`
  - support-only refinement theorems connecting `supportDominance` to the reference-best semantics, plus explicit counterexamples showing why tie-break and hop-band objectives are not promoted to global optimality
- `Field/Quality/System.lean`
  - system-facing quality theorems over `systemStep` lifecycle outputs, including stability, explicit-path non-manufacture, sender-local support/knowledge observer results, lifecycle-maintenance idempotence, and one-step appearance theorems for sparse active ready-installed evidence

## Notes

- layering rule:
  - `Field/Router` owns canonical route truth
  - `Field/Router/Probabilistic` owns posterior-based router decision truth
  - `Field/Quality` compares exported route views
  - `Field/Adequacy` owns reduction and runtime projection
  - `Field/Assumptions` packages contracts and theorem access
  - `Field/Information` and `Field/Model` own probabilistic local state, priors, likelihoods, and Bayesian posterior-update semantics
  - `Field/Retention` owns reduced payload custody, retention policy, and bounded retention execution state below router-owned route truth
  - only explicit support/canonical refinement theorems connect `Field/Quality` objectives back to router-owned truth; all other ranking objectives remain observational unless a theorem says otherwise

- stable architecture notes:
  - `PosteriorState`, `ReducedBeliefSummary`, `LocalOrderParameter`, and the Bayesian belief bridge are all explicit, and the reduced summary/order-parameter boundary is now stored directly in `LocalState`
  - `compressMeanFieldImpl` now owns only control fusion from `ReducedBeliefSummary` plus exogenous `controllerPressure`, instead of hiding posterior reduction internally
  - `Field/Model/Refinement.lean` now makes the intended theorem boundary explicit: the reduced summary is sufficient for the mean-field/controller surfaces only under fixed exogenous control inputs, and the theorem pack also records that the reduction alone does not determine the whole downstream control path
  - `LocalOrderParameter` is the explicit local regime/order-parameter surface between posterior reduction and control fusion
  - the corridor/coarse-graining story is now explicit end-to-end across `Field/Information/*` and `Field/Model/*`: retained aggregates feed the stored reduced summary, then the stored order parameter, then controller-facing fusion and public macrostate reasoning

- state taxonomy:
  - epistemic state: `FiniteBelief`, `PosteriorState`, `ProbabilisticRouteBelief`
  - control state: `ReducedBeliefSummary`, `LocalOrderParameter`, `MeanFieldState`, `ControllerState`, `RegimeState`, `PostureState`, `ScoredContinuationSet`
  - publication/public-observable state: `CorridorEnvelopeProjection`, `PublishedCandidate`, `AdmittedCandidate`
  - lifecycle state: `LifecycleRoute`
  - execution state: `AsyncState`, `RetentionState`, `EndToEndState`, `RuntimeState`

- projection taxonomy:
  - protocol projection: choreography/session structure -> local protocol surface (`Field/Protocol/*`)
  - local public projection: local field semantics -> corridor/public observable surface (`Field/Model/*`, `Field/Information/*`, `Field/Model/Boundary.lean`)
  - retention projection: controller/runtime signals -> payload-custody decisions and retained-token execution state (`Field/Retention/*`)
  - runtime projection / adequacy reduction: runtime artifacts or runtime state -> reduced Lean protocol/router/system surface (`Field/Adequacy/*`)

- truth ladder:
  - posterior confidence is local/private semantics
  - reduced summary and local order parameter are controller-facing reduced semantics, not public truth
  - canonical route is router-owned truth
  - quality is exported-view comparison
  - adequacy is a semantic bridge into reduced system/router layers, not a truth owner
  - negative boundaries kept explicit: quality is not truth, posterior confidence is not truth, projection is not installation, adequacy is not semantic ownership

- classical versus distributed split:
  - local quantitative/classical surfaces live primarily in `Field/Model/*` and `Field/Information/*`
  - distributed/profile-envelope surfaces live primarily in `Field/Async/*`, `Field/System/*`, and packaged assumption families
  - bridge theorems connecting local order-parameter interpretation to system convergence should state that boundary explicitly

- semantic versus proof-artifact split:
  - semantic core objects: runtime artifacts, runtime states, lifecycle routes, canonical selectors, probabilistic beliefs
  - theorem packaging: contract unlock theorems, boundary forwarding theorems, refinement wrappers
  - synthetic fixtures: adequacy fixture files and probabilistic fixture files

- docs:
  - `Field/Docs/Model.md`
    - local-model specification, stored posterior-to-reduction boundary, order-parameter interpretation, corridor coarse-graining story, and the explicit note that deferred payload retention stays runtime-facing rather than entering the local model
  - `Field/Docs/Protocol.md`
    - protocol, Telltale mapping, and replay/authority notes
  - `Field/Docs/Adequacy.md`
    - runtime artifact bridge, reduced runtime state/step layer, refinement ladder, and semantic-versus-fixture split in the adequacy stack
  - `Field/Docs/Guide.md`
    - contributor guidance, maturity summary, ownership rules, convergence assumptions, and stack-wide harmonization notes

- probabilistic scope:
  - modeled: route existence, route quality, transport reliability, and observation noise
  - explicitly separate from that scope: support ranking, exported quality views, and runtime extraction convenience layers
  - the current posterior-based router objectives are confidence-threshold routing plus reduced expectation / cost / risk / regret objects in `Field/Router/Probabilistic.lean`; they coexist with the older support-owned canonical selectors and are not implied by exported route views or support ranking unless a theorem says so
  - current Bayesian theorems are for the factorized likelihood model in `Field/Information/Bayesian.lean`; correlated evidence remains boundary-marked unless a replacement theorem says otherwise
  - current calibration/soundness results are confidence-threshold validity, posterior-probability equalities for the normalized update, expected-utility bounds, regret interpretation, explicit-evidence posterior support, produced-candidate latent-mass soundness, and a bounded public-projection weakening theorem; broad correlated calibration still remains out of scope
  - current GF1-style non-claims remain explicit: stronger divergence/update inequalities over the reduction, sharper mutual-information bounds for public observables, and information-theoretic optimality claims for the controller-facing summary are still open
  - explicit non-goals for the current probabilistic roadmap: arbitrary continuous distributions, unproved calibration claims, and full production-runtime probabilistic fidelity

## Maturity Snapshot

- most mature:
  - local boundedness/harmony/honesty theorems
  - reduced private protocol and observational boundary
- moderate:
  - reduced finite network, publication, admission, installation, and lifecycle semantics
  - first network-level safety theorems
  - reduced async semantics, transport lifecycle lemmas, and first async safety theorems
  - router-owned canonical selection over lifecycle routes
  - system-level aggregate summaries, reduced end-to-end safety/observer theorems, reliable-immediate convergence results, and canonical-router refinement
  - first silence-only bounded-dropout resilience theorems
  - reduced route-comparison / ranking semantics and support-only reference refinement above system-facing lifecycle outputs
  - projected reduced runtime/system refinement to router-owned canonical truth
  - runtime/system safety-preservation theorems and proof-facing fixture cases
  - probability-simplex information layer
  - normalized public-projection blindness bridge
  - one-step decision layer
  - reduced protocol-machine fragment
- earliest:
  - stronger extracted-Rust runtime correctness theorem beyond the reduced simulation bridge and projected runtime/system refinement
  - convergence beyond the reliable-immediate / empty-queue / unchanged-network regime
  - stronger global routing optimality theorem beyond the current router-owned support and support-then-hop selectors and their reduced system refinements
  - deeper Telltale-native reuse of conservation and subtype-replacement families
