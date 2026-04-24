//! Coded-diffusion research-path boundary.
//!
//! This module is the feature-neutral namespace for the experimental coded
//! reconstruction path. It owns only fragment movement, rank, custody,
//! duplicate/innovative arrivals, diffusion pressure, and reconstruction
//! quorum vocabulary. It must remain independent of the legacy planner stack.

use jacquard_core::{DurationMs, NodeId, Tick};
use serde::{Deserialize, Serialize};

/// Stable target identifier for one reconstruction or inference objective.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CodedTargetId(pub u32);

/// Stable message identifier for one coded reconstruction objective.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct DiffusionMessageId(pub [u8; 16]);

/// Stable evidence identifier for one reconstruction or inference record.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CodedEvidenceId(pub u32);

/// Stable fragment identifier within one coded reconstruction objective.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct DiffusionFragmentId(pub [u8; 16]);

/// Stable coding-rank identifier for one independent reconstruction contribution.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CodingRankId(pub u32);

/// Stable local-observation identifier for distributed evidence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct LocalObservationId(pub u32);

/// Stable contribution-ledger identifier used to audit useful rank.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ContributionLedgerId(pub u32);

/// Stable inference-task identifier for one anytime landscape.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct InferenceTaskId(pub u32);

/// Stable candidate id for anomaly-localization cluster hypotheses.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct AnomalyClusterId(pub u16);

/// Maximum candidate clusters represented by one anomaly landscape.
pub const ANOMALY_HYPOTHESIS_COUNT_MAX: u16 = 256;

/// Maximum demand entries carried by one active belief demand summary.
pub const ACTIVE_DEMAND_ENTRY_COUNT_MAX: u16 = 32;

/// How one contribution ledger entry is justified.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ContributionLedgerKind {
    /// Independent rank contribution from a source-coded fragment.
    SourceCodedRank,
    /// Independent contribution from a local observation.
    LocalObservation,
    /// Recoded output that forwards parent contribution ids without adding rank.
    ParentLedgerUnion,
    /// Recoded aggregate that adds one valid local observation contribution.
    AggregateWithLocalObservation,
}

/// Validation failure for contribution-ledger records.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ContributionLedgerRecordError {
    /// Original source/local contribution records must not name parent contributions.
    UnexpectedParentContribution,
    /// Local-observation contributions must name the local observation.
    MissingLocalObservation,
    /// Source-coded rank contributions must not name a local observation.
    UnexpectedLocalObservation,
    /// Recoded contribution records must name at least one parent contribution.
    RecodedWithoutParentContributions,
    /// Parent contribution ids must be unique after deterministic ordering.
    DuplicateParentContribution,
    /// Parent-ledger unions can only forward contribution ids that are already parents.
    ParentLedgerUnionIntroducesContribution,
    /// Aggregate contributions must include a local observation contribution.
    AggregateWithoutLocalObservation,
}

/// Construction input for an auditable contribution-ledger record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContributionLedgerRecordInput {
    /// Evidence record whose contribution is being ledgered.
    pub evidence_id: CodedEvidenceId,
    /// Contribution id counted by receiver rank or aggregate logic.
    pub contribution_id: ContributionLedgerId,
    /// Justification class for the contribution.
    pub contribution_kind: ContributionLedgerKind,
    /// Parent contribution ids used by recoded evidence.
    pub parent_contribution_ids: Vec<ContributionLedgerId>,
    /// Local observation used by local or aggregate evidence.
    pub local_observation_id: Option<LocalObservationId>,
}

/// Auditable record explaining why one contribution id is valid.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContributionLedgerRecord {
    /// Evidence record whose contribution is being ledgered.
    pub evidence_id: CodedEvidenceId,
    /// Contribution id counted by receiver rank or aggregate logic.
    pub contribution_id: ContributionLedgerId,
    /// Justification class for the contribution.
    pub contribution_kind: ContributionLedgerKind,
    /// Canonical parent contribution ids used by recoded evidence.
    pub parent_contribution_ids: Vec<ContributionLedgerId>,
    /// Local observation used by local or aggregate evidence.
    pub local_observation_id: Option<LocalObservationId>,
}

impl ContributionLedgerRecord {
    /// Build a canonical contribution-ledger record.
    pub fn try_new(
        mut input: ContributionLedgerRecordInput,
    ) -> Result<Self, ContributionLedgerRecordError> {
        canonicalize_contribution_ids(&mut input.parent_contribution_ids)?;
        validate_contribution_record_shape(&input)?;

        Ok(Self {
            evidence_id: input.evidence_id,
            contribution_id: input.contribution_id,
            contribution_kind: input.contribution_kind,
            parent_contribution_ids: input.parent_contribution_ids,
            local_observation_id: input.local_observation_id,
        })
    }
}

/// Source of one coded-evidence record.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EvidenceOriginMode {
    /// Fragment came from a source-coded reconstruction payload.
    SourceCoded,
    /// Evidence was generated from a node-local observation.
    LocallyGenerated,
    /// Evidence was recoded or aggregated from parent evidence records.
    RecodedAggregated,
}

/// Validity status assigned after deterministic record validation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CodedEvidenceValidity {
    /// Record passed the local syntactic and lineage validity checks.
    Valid,
}

/// Validation failure for coded evidence records.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CodedEvidenceRecordError {
    /// Source-coded evidence must carry both a fragment id and a rank id.
    MissingSourceFragmentOrRank,
    /// Source-coded evidence must not carry a local observation id.
    UnexpectedLocalObservation,
    /// Locally generated evidence must carry a local observation id.
    MissingLocalObservation,
    /// Original source/local records must not carry parent evidence ids.
    UnexpectedParentEvidence,
    /// Recoded or aggregated evidence must name at least one parent.
    RecodedWithoutParents,
    /// A recoded record cannot name itself as a parent.
    SelfParent,
    /// Parent ids must be unique after deterministic ordering.
    DuplicateParentEvidence,
    /// Contribution ledger ids must be nonempty.
    EmptyContributionLedger,
    /// Contribution ledger ids must be unique after deterministic ordering.
    DuplicateContributionLedger,
    /// Evidence payloads must consume at least one byte.
    ZeroPayloadBytes,
    /// Recoding validation was requested for a non-recoded evidence record.
    NotRecodedEvidence,
    /// Recoded evidence is missing a ledger record for one contribution id.
    MissingContributionLedgerRecord,
    /// A ledger record names an evidence id different from the recoded record.
    UnexpectedContributionLedgerRecord,
    /// A recoded ledger union tried to introduce a non-parent contribution id.
    InvalidParentLedgerUnion,
    /// A recoded aggregate contribution did not carry valid local-observation support.
    InvalidAggregateContribution,
}

/// Construction input for one reconstruction or inference evidence record.
///
/// The resulting record is evidence-facing, not route-facing: parent ids and
/// contribution ledgers explain rank or aggregate contribution validity, not
/// route admission, corridor support, selected private paths, or ranking.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CodedEvidenceRecordInput {
    /// Reconstruction or inference target receiving this evidence.
    pub target_id: CodedTargetId,
    /// Message or task id that owns this evidence.
    pub message_id: DiffusionMessageId,
    /// Stable evidence id.
    pub evidence_id: CodedEvidenceId,
    /// Source mode for this evidence.
    pub origin_mode: EvidenceOriginMode,
    /// Source-coded fragment id, when applicable.
    pub fragment_id: Option<DiffusionFragmentId>,
    /// Source-coded rank id, when applicable.
    pub rank_id: Option<CodingRankId>,
    /// Current holder for custody or forwarding.
    pub holder: NodeId,
    /// Local observation id, when this evidence includes local data.
    pub local_observation_id: Option<LocalObservationId>,
    /// Parent evidence records for recoded or aggregated evidence.
    pub parent_evidence_ids: Vec<CodedEvidenceId>,
    /// Canonical contribution ids counted by receiver rank or aggregate logic.
    pub contribution_ledger_ids: Vec<ContributionLedgerId>,
    /// Deterministic payload size in bytes.
    pub payload_bytes: u32,
}

/// Deterministic reconstruction or inference evidence record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CodedEvidenceRecord {
    /// Reconstruction or inference target receiving this evidence.
    pub target_id: CodedTargetId,
    /// Message or task id that owns this evidence.
    pub message_id: DiffusionMessageId,
    /// Stable evidence id.
    pub evidence_id: CodedEvidenceId,
    /// Source mode for this evidence.
    pub origin_mode: EvidenceOriginMode,
    /// Source-coded fragment id, when applicable.
    pub fragment_id: Option<DiffusionFragmentId>,
    /// Source-coded rank id, when applicable.
    pub rank_id: Option<CodingRankId>,
    /// Current holder for custody or forwarding.
    pub holder: NodeId,
    /// Local observation id, when this evidence includes local data.
    pub local_observation_id: Option<LocalObservationId>,
    /// Parent evidence records for recoded or aggregated evidence.
    pub parent_evidence_ids: Vec<CodedEvidenceId>,
    /// Canonical contribution ids counted by receiver rank or aggregate logic.
    pub contribution_ledger_ids: Vec<ContributionLedgerId>,
    /// Deterministic payload size in bytes.
    pub payload_bytes: u32,
    /// Validity marker assigned by `CodedEvidenceRecord::try_new`.
    pub validity: CodedEvidenceValidity,
}

impl CodedEvidenceRecord {
    /// Build a canonical evidence record or reject malformed lineage.
    pub fn try_new(mut input: CodedEvidenceRecordInput) -> Result<Self, CodedEvidenceRecordError> {
        canonicalize_ids(
            &mut input.parent_evidence_ids,
            CodedEvidenceRecordError::DuplicateParentEvidence,
        )?;
        canonicalize_ids(
            &mut input.contribution_ledger_ids,
            CodedEvidenceRecordError::DuplicateContributionLedger,
        )?;
        if input.contribution_ledger_ids.is_empty() {
            return Err(CodedEvidenceRecordError::EmptyContributionLedger);
        }
        if input.payload_bytes == 0 {
            return Err(CodedEvidenceRecordError::ZeroPayloadBytes);
        }
        validate_origin_shape(&input)?;

        Ok(Self {
            target_id: input.target_id,
            message_id: input.message_id,
            evidence_id: input.evidence_id,
            origin_mode: input.origin_mode,
            fragment_id: input.fragment_id,
            rank_id: input.rank_id,
            holder: input.holder,
            local_observation_id: input.local_observation_id,
            parent_evidence_ids: input.parent_evidence_ids,
            contribution_ledger_ids: input.contribution_ledger_ids,
            payload_bytes: input.payload_bytes,
            validity: CodedEvidenceValidity::Valid,
        })
    }

    /// Validate contribution-ledger records for auditable recoded evidence.
    pub fn validate_recoding_ledger(
        &self,
        ledger_records: &[ContributionLedgerRecord],
    ) -> Result<Vec<ContributionLedgerId>, CodedEvidenceRecordError> {
        if self.origin_mode != EvidenceOriginMode::RecodedAggregated {
            return Err(CodedEvidenceRecordError::NotRecodedEvidence);
        }

        let mut accepted_contribution_ids = Vec::with_capacity(self.contribution_ledger_ids.len());
        for contribution_id in &self.contribution_ledger_ids {
            let ledger_record =
                find_ledger_record(self.evidence_id, *contribution_id, ledger_records)?;
            validate_recoded_ledger_record(ledger_record)?;
            accepted_contribution_ids.push(*contribution_id);
        }
        if accepted_contribution_ids.len() != ledger_records.len() {
            return Err(CodedEvidenceRecordError::UnexpectedContributionLedgerRecord);
        }

        Ok(accepted_contribution_ids)
    }
}

fn validate_contribution_record_shape(
    input: &ContributionLedgerRecordInput,
) -> Result<(), ContributionLedgerRecordError> {
    match input.contribution_kind {
        ContributionLedgerKind::SourceCodedRank => {
            if !input.parent_contribution_ids.is_empty() {
                return Err(ContributionLedgerRecordError::UnexpectedParentContribution);
            }
            if input.local_observation_id.is_some() {
                return Err(ContributionLedgerRecordError::UnexpectedLocalObservation);
            }
        }
        ContributionLedgerKind::LocalObservation => {
            if !input.parent_contribution_ids.is_empty() {
                return Err(ContributionLedgerRecordError::UnexpectedParentContribution);
            }
            if input.local_observation_id.is_none() {
                return Err(ContributionLedgerRecordError::MissingLocalObservation);
            }
        }
        ContributionLedgerKind::ParentLedgerUnion => {
            if input.parent_contribution_ids.is_empty() {
                return Err(ContributionLedgerRecordError::RecodedWithoutParentContributions);
            }
            if !input
                .parent_contribution_ids
                .contains(&input.contribution_id)
            {
                return Err(ContributionLedgerRecordError::ParentLedgerUnionIntroducesContribution);
            }
        }
        ContributionLedgerKind::AggregateWithLocalObservation => {
            if input.parent_contribution_ids.is_empty() {
                return Err(ContributionLedgerRecordError::RecodedWithoutParentContributions);
            }
            if input.local_observation_id.is_none() {
                return Err(ContributionLedgerRecordError::AggregateWithoutLocalObservation);
            }
        }
    }
    Ok(())
}

fn canonicalize_contribution_ids(
    values: &mut [ContributionLedgerId],
) -> Result<(), ContributionLedgerRecordError> {
    values.sort_unstable();
    if values.windows(2).any(|window| window[0] == window[1]) {
        return Err(ContributionLedgerRecordError::DuplicateParentContribution);
    }
    Ok(())
}

fn find_ledger_record(
    evidence_id: CodedEvidenceId,
    contribution_id: ContributionLedgerId,
    ledger_records: &[ContributionLedgerRecord],
) -> Result<&ContributionLedgerRecord, CodedEvidenceRecordError> {
    let mut found = None;
    for ledger_record in ledger_records {
        if ledger_record.evidence_id != evidence_id {
            return Err(CodedEvidenceRecordError::UnexpectedContributionLedgerRecord);
        }
        if ledger_record.contribution_id == contribution_id {
            if found.is_some() {
                return Err(CodedEvidenceRecordError::DuplicateContributionLedger);
            }
            found = Some(ledger_record);
        }
    }
    found.ok_or(CodedEvidenceRecordError::MissingContributionLedgerRecord)
}

fn validate_recoded_ledger_record(
    ledger_record: &ContributionLedgerRecord,
) -> Result<(), CodedEvidenceRecordError> {
    match ledger_record.contribution_kind {
        ContributionLedgerKind::ParentLedgerUnion => {
            if !ledger_record
                .parent_contribution_ids
                .contains(&ledger_record.contribution_id)
            {
                return Err(CodedEvidenceRecordError::InvalidParentLedgerUnion);
            }
        }
        ContributionLedgerKind::AggregateWithLocalObservation => {
            if ledger_record.local_observation_id.is_none() {
                return Err(CodedEvidenceRecordError::InvalidAggregateContribution);
            }
        }
        ContributionLedgerKind::SourceCodedRank | ContributionLedgerKind::LocalObservation => {
            return Err(CodedEvidenceRecordError::UnexpectedContributionLedgerRecord);
        }
    }
    Ok(())
}

fn validate_origin_shape(input: &CodedEvidenceRecordInput) -> Result<(), CodedEvidenceRecordError> {
    match input.origin_mode {
        EvidenceOriginMode::SourceCoded => {
            if input.fragment_id.is_none() || input.rank_id.is_none() {
                return Err(CodedEvidenceRecordError::MissingSourceFragmentOrRank);
            }
            if input.local_observation_id.is_some() {
                return Err(CodedEvidenceRecordError::UnexpectedLocalObservation);
            }
            if !input.parent_evidence_ids.is_empty() {
                return Err(CodedEvidenceRecordError::UnexpectedParentEvidence);
            }
        }
        EvidenceOriginMode::LocallyGenerated => {
            if input.local_observation_id.is_none() {
                return Err(CodedEvidenceRecordError::MissingLocalObservation);
            }
            if !input.parent_evidence_ids.is_empty() {
                return Err(CodedEvidenceRecordError::UnexpectedParentEvidence);
            }
        }
        EvidenceOriginMode::RecodedAggregated => {
            if input.parent_evidence_ids.is_empty() {
                return Err(CodedEvidenceRecordError::RecodedWithoutParents);
            }
            if input.parent_evidence_ids.contains(&input.evidence_id) {
                return Err(CodedEvidenceRecordError::SelfParent);
            }
        }
    }
    Ok(())
}

fn canonicalize_ids<T: Copy + Ord>(
    values: &mut [T],
    duplicate_error: CodedEvidenceRecordError,
) -> Result<(), CodedEvidenceRecordError> {
    values.sort_unstable();
    if values.windows(2).any(|window| window[0] == window[1]) {
        return Err(duplicate_error);
    }
    Ok(())
}

/// Bounded coding-width description for one message.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CodingWindow {
    /// Independent rank required for reconstruction.
    pub required_rank: u16,
    /// Number of encoded fragments available to diffuse.
    pub encoded_fragments: u16,
}

impl CodingWindow {
    /// Construct a valid coding window.
    pub fn try_new(required_rank: u16, encoded_fragments: u16) -> Option<Self> {
        if required_rank == 0 || encoded_fragments < required_rank {
            return None;
        }

        Some(Self {
            required_rank,
            encoded_fragments,
        })
    }
}

/// Fixed-budget comparison mode for coded evidence experiments.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PayloadBudgetKind {
    /// Coded fragments and uncoded replicas are configured to spend the same payload bytes.
    EqualPayloadBytes,
    /// Secondary comparison where forwarding opportunities, not bytes, are fixed.
    FixedForwardingOpportunities,
    /// Secondary comparison where retained storage bytes are fixed.
    FixedStorageBytes,
}

/// Construction failure for deterministic coded-vs-uncoded payload budgets.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PayloadBudgetError {
    /// Coded fragment payload size must be nonzero.
    ZeroFragmentPayloadBytes,
    /// Uncoded message payload size must be nonzero.
    ZeroUncodedMessagePayloadBytes,
    /// Uncoded replica count must be nonzero.
    ZeroUncodedReplicaCount,
    /// Payload multiplication exceeded the representable deterministic budget.
    PayloadBudgetOverflow,
    /// Equal-byte comparisons require coded and uncoded payload budgets to match.
    UnequalPayloadByteBudget,
}

/// Receiver-rank construction or update failure.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReceiverRankError {
    /// Exact reconstruction requires a positive rank threshold.
    ZeroRequiredRank,
    /// Receiver contribution ledgers are bounded by the serialized `u16` rank shape.
    ContributionLedgerFull,
}

/// Integer byte-budget metadata for fair coded-vs-uncoded comparisons.
///
/// This is reconstruction accounting only. It names the fixed comparison
/// budget and byte units; it does not score routes or affect route admission.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PayloadBudgetMetadata {
    /// Comparison rule used for this budget.
    pub budget_kind: PayloadBudgetKind,
    /// Reconstruction window for the coded policy.
    pub coding_window: CodingWindow,
    /// Deterministic payload bytes carried by one coded fragment.
    pub fragment_payload_bytes: u32,
    /// Deterministic payload bytes carried by one uncoded full-message replica.
    pub uncoded_message_payload_bytes: u32,
    /// Number of uncoded full-message replicas under the fixed budget.
    pub uncoded_replica_count: u16,
    /// Fixed payload-byte budget represented by this comparison.
    pub fixed_payload_budget_bytes: u32,
}

impl PayloadBudgetMetadata {
    /// Build equal-byte budget metadata for coded fragments and uncoded replicas.
    pub fn equal_payload_bytes(
        coding_window: CodingWindow,
        fragment_payload_bytes: u32,
        uncoded_message_payload_bytes: u32,
        uncoded_replica_count: u16,
    ) -> Result<Self, PayloadBudgetError> {
        if fragment_payload_bytes == 0 {
            return Err(PayloadBudgetError::ZeroFragmentPayloadBytes);
        }
        if uncoded_message_payload_bytes == 0 {
            return Err(PayloadBudgetError::ZeroUncodedMessagePayloadBytes);
        }
        if uncoded_replica_count == 0 {
            return Err(PayloadBudgetError::ZeroUncodedReplicaCount);
        }

        let coded_payload_bytes = checked_payload_product(
            u32::from(coding_window.encoded_fragments),
            fragment_payload_bytes,
        )?;
        let uncoded_payload_bytes = checked_payload_product(
            u32::from(uncoded_replica_count),
            uncoded_message_payload_bytes,
        )?;
        if coded_payload_bytes != uncoded_payload_bytes {
            return Err(PayloadBudgetError::UnequalPayloadByteBudget);
        }

        Ok(Self {
            budget_kind: PayloadBudgetKind::EqualPayloadBytes,
            coding_window,
            fragment_payload_bytes,
            uncoded_message_payload_bytes,
            uncoded_replica_count,
            fixed_payload_budget_bytes: coded_payload_bytes,
        })
    }

    /// Total coded-fragment payload bytes under this metadata.
    #[must_use]
    pub fn coded_payload_budget_bytes(self) -> u32 {
        self.fixed_payload_budget_bytes
    }

    /// Total uncoded full-message payload bytes under this metadata.
    #[must_use]
    pub fn uncoded_payload_budget_bytes(self) -> u32 {
        self.fixed_payload_budget_bytes
    }

    /// Whether this metadata names an equal payload-byte comparison.
    #[must_use]
    pub fn has_equal_payload_byte_budget(self) -> bool {
        self.budget_kind == PayloadBudgetKind::EqualPayloadBytes
            && self.coded_payload_budget_bytes() == self.uncoded_payload_budget_bytes()
    }
}

fn checked_payload_product(count: u32, bytes: u32) -> Result<u32, PayloadBudgetError> {
    count
        .checked_mul(bytes)
        .ok_or(PayloadBudgetError::PayloadBudgetOverflow)
}

/// Validation failure for anomaly-localization landscape state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AnomalyLandscapeError {
    /// A landscape must contain at least two candidate hypotheses.
    TooFewHypotheses,
    /// Candidate hypotheses are capped for deterministic replay surfaces.
    TooManyHypotheses,
    /// Candidate hypothesis ids must be unique after canonical ordering.
    DuplicateHypothesis,
    /// Fixture ground truth must be one of the candidate hypotheses.
    HiddenHypothesisMissing,
    /// Score vectors must name each candidate exactly once.
    MalformedScoreVector,
    /// Score vector entries must not duplicate a hypothesis id.
    DuplicateScoreHypothesis,
    /// Score vector entries must not name a non-candidate hypothesis id.
    ScoreForUnknownHypothesis,
    /// Decision margin thresholds are nonnegative integer values.
    NegativeDecisionMarginThreshold,
    /// Decision commitment requires at least one independent contribution.
    ZeroMinimumDecisionEvidence,
}

/// Validation failure for anomaly evidence-vector records.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EvidenceVectorRecordError {
    /// Evidence vectors must carry at least one score update.
    EmptyScoreUpdate,
    /// Evidence-vector records must target the same coded target as the landscape.
    TargetMismatch,
    /// The contribution id must be present in the Phase 1 evidence record.
    ContributionNotInEvidence,
    /// Score update vectors must match the landscape hypotheses exactly.
    MalformedScoreUpdate,
    /// Local-observation records must carry the Phase 1 local observation id.
    MalformedLocalObservationReference,
    /// Recoded or aggregate records must carry unambiguous parent evidence lineage.
    AmbiguousRecodedLineage,
    /// A batch must contain at least one evidence-vector record.
    EmptyBatch,
    /// Batch records must all belong to the same inference task.
    MixedInferenceTask,
    /// A batch can update a contribution id at most once.
    DuplicateContributionUpdate,
}

/// Validation failure for pure anomaly landscape update reduction.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LandscapeUpdateError {
    /// Update records must belong to the landscape inference task.
    TaskMismatch,
    /// Update records must belong to the landscape coded target.
    TargetMismatch,
    /// A reducer input can update a contribution id at most once.
    DuplicateContributionUpdate,
    /// Receiver-rank state rejected the contribution update.
    ReceiverRankUpdateFailed,
}

/// Validation failure for anomaly decision commitment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DecisionCommitmentError {
    /// Commitment state must belong to the landscape inference task.
    TaskMismatch,
    /// Commitment state must belong to the landscape coded target.
    TargetMismatch,
}

/// One deterministic integer score for one anomaly hypothesis.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnomalyHypothesisScore {
    /// Candidate cluster being scored.
    pub hypothesis_id: AnomalyClusterId,
    /// Deterministic scaled score or energy for this candidate.
    pub scaled_score: i32,
}

/// Bounded fixture/noise class attached to deterministic anomaly evidence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnomalyEvidenceClass(pub u8);

/// Bounded candidate metadata for one anomaly-localization landscape.
///
/// This is inference state only. It does not describe route admission,
/// corridor support, selected private paths, or route-quality ranking.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnomalyHypothesisSet {
    /// Inference task represented by this landscape.
    pub task_id: InferenceTaskId,
    /// Coded target that owns this landscape.
    pub target_id: CodedTargetId,
    /// Optional ground-truth cluster used by deterministic fixtures.
    pub hidden_anomaly_cluster_id: Option<AnomalyClusterId>,
    /// Canonical candidate cluster ids.
    pub candidate_hypotheses: Vec<AnomalyClusterId>,
}

impl AnomalyHypothesisSet {
    /// Build canonical anomaly-hypothesis metadata.
    pub fn try_new(
        task_id: InferenceTaskId,
        target_id: CodedTargetId,
        mut candidate_hypotheses: Vec<AnomalyClusterId>,
        hidden_anomaly_cluster_id: Option<AnomalyClusterId>,
    ) -> Result<Self, AnomalyLandscapeError> {
        canonicalize_anomaly_hypotheses(&mut candidate_hypotheses)?;
        if let Some(hidden) = hidden_anomaly_cluster_id {
            if !candidate_hypotheses.contains(&hidden) {
                return Err(AnomalyLandscapeError::HiddenHypothesisMissing);
            }
        }

        Ok(Self {
            task_id,
            target_id,
            hidden_anomaly_cluster_id,
            candidate_hypotheses,
        })
    }

    /// Number of candidate hypotheses.
    #[must_use]
    pub fn hypothesis_count(&self) -> u16 {
        u16::try_from(self.candidate_hypotheses.len()).unwrap_or(u16::MAX)
    }
}

/// Decision guard for anomaly-localization commitment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnomalyDecisionGuard {
    /// Required top-vs-runner-up margin before commitment.
    pub margin_threshold: i32,
    /// Required independent evidence count before commitment.
    pub minimum_independent_evidence: u16,
}

impl AnomalyDecisionGuard {
    /// Build decision-guard metadata.
    pub fn try_new(
        margin_threshold: i32,
        minimum_independent_evidence: u16,
    ) -> Result<Self, AnomalyLandscapeError> {
        if margin_threshold < 0 {
            return Err(AnomalyLandscapeError::NegativeDecisionMarginThreshold);
        }
        if minimum_independent_evidence == 0 {
            return Err(AnomalyLandscapeError::ZeroMinimumDecisionEvidence);
        }

        Ok(Self {
            margin_threshold,
            minimum_independent_evidence,
        })
    }
}

/// Deterministic quality summary for one anomaly landscape.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnomalyLandscapeSummary {
    /// Highest-scoring hypothesis after deterministic tie-breaking.
    pub top_hypothesis: AnomalyClusterId,
    /// Next-highest hypothesis after deterministic tie-breaking.
    pub runner_up_hypothesis: AnomalyClusterId,
    /// Top score minus runner-up score.
    pub top_hypothesis_margin: i32,
    /// Fixed-denominator uncertainty proxy in permille.
    pub uncertainty_permille: u16,
    /// Integer energy gap derived from score gaps.
    pub energy_gap: i32,
}

/// Receiver-local anomaly-localization landscape.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnomalyLandscape {
    /// Candidate metadata for this inference task.
    pub hypotheses: AnomalyHypothesisSet,
    /// Canonical score vector ordered by hypothesis id.
    pub scores: Vec<AnomalyHypothesisScore>,
    /// Decision guard used by later commitment checks.
    pub decision_guard: AnomalyDecisionGuard,
    /// Deterministic quality summary for the current score vector.
    pub summary: AnomalyLandscapeSummary,
}

/// Deterministic score update attached to one accepted evidence contribution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceVectorRecord {
    /// Inference task updated by this vector.
    pub task_id: InferenceTaskId,
    /// Coded target updated by this vector.
    pub target_id: CodedTargetId,
    /// Evidence record carrying the contribution.
    pub evidence_id: CodedEvidenceId,
    /// Contribution id that gates one score update.
    pub contribution_id: ContributionLedgerId,
    /// Phase 1 origin mode preserved for logs and summaries.
    pub origin_mode: EvidenceOriginMode,
    /// Optional local observation id for local or aggregate evidence.
    pub local_observation_id: Option<LocalObservationId>,
    /// Parent evidence ids for recoded or aggregate evidence.
    pub parent_evidence_ids: Vec<CodedEvidenceId>,
    /// Canonical integer score update ordered by hypothesis id.
    pub score_update: Vec<AnomalyHypothesisScore>,
    /// Optional bounded fixture/noise class for anomaly tests.
    pub evidence_class: Option<AnomalyEvidenceClass>,
    /// Deterministic payload bytes inherited from the Phase 1 evidence record.
    pub payload_bytes: u32,
}

impl EvidenceVectorRecord {
    /// Build an evidence-vector record from validated Phase 1 evidence.
    pub fn try_from_evidence(
        landscape: &AnomalyLandscape,
        evidence: &CodedEvidenceRecord,
        contribution_id: ContributionLedgerId,
        mut score_update: Vec<AnomalyHypothesisScore>,
        evidence_class: Option<AnomalyEvidenceClass>,
    ) -> Result<Self, EvidenceVectorRecordError> {
        if score_update.is_empty() {
            return Err(EvidenceVectorRecordError::EmptyScoreUpdate);
        }
        if evidence.target_id != landscape.hypotheses.target_id {
            return Err(EvidenceVectorRecordError::TargetMismatch);
        }
        if !evidence.contribution_ledger_ids.contains(&contribution_id) {
            return Err(EvidenceVectorRecordError::ContributionNotInEvidence);
        }
        validate_evidence_vector_origin(evidence)?;
        canonicalize_anomaly_scores(&landscape.hypotheses, &mut score_update)
            .map_err(|_| EvidenceVectorRecordError::MalformedScoreUpdate)?;

        Ok(Self {
            task_id: landscape.hypotheses.task_id,
            target_id: evidence.target_id,
            evidence_id: evidence.evidence_id,
            contribution_id,
            origin_mode: evidence.origin_mode,
            local_observation_id: evidence.local_observation_id,
            parent_evidence_ids: evidence.parent_evidence_ids.clone(),
            score_update,
            evidence_class,
            payload_bytes: evidence.payload_bytes,
        })
    }
}

/// Canonical batch of score updates for one inference landscape.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceVectorBatch {
    /// Inference task updated by this batch.
    pub task_id: InferenceTaskId,
    /// Canonical records ordered by contribution id.
    pub records: Vec<EvidenceVectorRecord>,
}

/// Replay-visible result of applying one evidence-vector record.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LandscapeUpdateEvent {
    /// Inference task updated by this event.
    pub task_id: InferenceTaskId,
    /// Evidence record that carried the score update.
    pub evidence_id: CodedEvidenceId,
    /// Contribution id used for duplicate suppression.
    pub contribution_id: ContributionLedgerId,
    /// Phase 1 origin mode preserved for logs.
    pub origin_mode: EvidenceOriginMode,
    /// Whether this contribution changed rank and landscape score.
    pub arrival_class: FragmentArrivalClass,
    /// Receiver rank before applying the contribution gate.
    pub rank_before: u16,
    /// Receiver rank after applying the contribution gate.
    pub rank_after: u16,
    /// Summary before applying this update.
    pub summary_before: AnomalyLandscapeSummary,
    /// Summary after applying this update.
    pub summary_after: AnomalyLandscapeSummary,
}

/// Pure reducer output for receiver-rank and landscape updates.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LandscapeUpdateOutcome {
    /// Updated receiver rank state.
    pub receiver_rank: ReceiverRankState,
    /// Updated anomaly landscape.
    pub landscape: AnomalyLandscape,
    /// Replay-visible update events in canonical contribution order.
    pub events: Vec<LandscapeUpdateEvent>,
    /// Number of updates that changed independent rank and scores.
    pub innovative_update_count: u16,
    /// Number of duplicate updates that left scores unchanged.
    pub duplicate_update_count: u16,
}

/// Idempotent decision-commitment state for one anomaly landscape.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DecisionCommitmentState {
    /// Inference task governed by this commitment state.
    pub task_id: InferenceTaskId,
    /// Coded target governed by this commitment state.
    pub target_id: CodedTargetId,
    /// Hypothesis committed by the first successful check.
    pub committed_hypothesis: Option<AnomalyClusterId>,
    /// Tick when margin and independent-evidence guard first passed.
    pub committed_at_tick: Option<Tick>,
    /// Quality summary observed at the first successful commitment.
    pub summary_at_commitment: Option<AnomalyLandscapeSummary>,
}

impl DecisionCommitmentState {
    /// Construct empty commitment state for one landscape.
    #[must_use]
    pub fn new(task_id: InferenceTaskId, target_id: CodedTargetId) -> Self {
        Self {
            task_id,
            target_id,
            committed_hypothesis: None,
            committed_at_tick: None,
            summary_at_commitment: None,
        }
    }

    /// Record the first commitment tick when both declared guards pass.
    pub fn check_commitment(
        &mut self,
        landscape: &AnomalyLandscape,
        receiver_rank: &ReceiverRankState,
        observed_at_tick: Tick,
    ) -> Result<Option<Tick>, DecisionCommitmentError> {
        self.validate_landscape(landscape)?;
        if self.committed_at_tick.is_some() {
            return Ok(self.committed_at_tick);
        }
        if receiver_rank.independent_rank < landscape.decision_guard.minimum_independent_evidence {
            return Ok(None);
        }
        if landscape.summary.top_hypothesis_margin < landscape.decision_guard.margin_threshold {
            return Ok(None);
        }

        self.committed_hypothesis = Some(landscape.summary.top_hypothesis);
        self.committed_at_tick = Some(observed_at_tick);
        self.summary_at_commitment = Some(landscape.summary);
        Ok(self.committed_at_tick)
    }

    fn validate_landscape(
        self,
        landscape: &AnomalyLandscape,
    ) -> Result<(), DecisionCommitmentError> {
        if self.task_id != landscape.hypotheses.task_id {
            return Err(DecisionCommitmentError::TaskMismatch);
        }
        if self.target_id != landscape.hypotheses.target_id {
            return Err(DecisionCommitmentError::TargetMismatch);
        }
        Ok(())
    }
}

/// Point-in-time inference progress separate from exact reconstruction.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnomalyDecisionProgressSummary {
    /// Inference task represented by this summary.
    pub task_id: InferenceTaskId,
    /// Coded target represented by this summary.
    pub target_id: CodedTargetId,
    /// Receiver whose rank is summarized.
    pub receiver: NodeId,
    /// Current independent receiver rank.
    pub receiver_rank: u16,
    /// Exact reconstruction tick from Phase 1 receiver state.
    pub reconstructed_at_tick: Option<Tick>,
    /// Decision commitment tick from Phase 2 commitment state.
    pub committed_at_tick: Option<Tick>,
    /// Current landscape quality, available before reconstruction or commitment.
    pub landscape_summary: AnomalyLandscapeSummary,
}

/// Evidence-origin counts for inference quality summaries.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceOriginUpdateCounts {
    /// Source-coded evidence update events.
    pub source_coded: u16,
    /// Locally generated evidence update events.
    pub locally_generated: u16,
    /// Recoded or aggregate evidence update events.
    pub recoded_aggregated: u16,
}

/// Receiver-facing inference quality summary for one anomaly landscape.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReceiverInferenceQualitySummary {
    /// Inference task represented by this summary.
    pub task_id: InferenceTaskId,
    /// Coded target represented by this summary.
    pub target_id: CodedTargetId,
    /// Receiver whose quality is summarized.
    pub receiver: NodeId,
    /// Current independent receiver rank.
    pub receiver_rank: u16,
    /// Exact reconstruction tick from Phase 1 receiver state.
    pub exact_reconstruction_tick: Option<Tick>,
    /// Decision commitment tick from Phase 2 commitment state.
    pub decision_commitment_tick: Option<Tick>,
    /// Current top hypothesis.
    pub top_hypothesis: AnomalyClusterId,
    /// Current runner-up hypothesis.
    pub runner_up_hypothesis: AnomalyClusterId,
    /// Current top-vs-runner-up margin.
    pub top_hypothesis_margin: i32,
    /// Current fixed-denominator uncertainty proxy.
    pub uncertainty_permille: u16,
    /// Current integer energy gap.
    pub energy_gap: i32,
    /// Number of update events that changed rank and quality.
    pub innovative_update_count: u16,
    /// Number of update events that left quality unchanged.
    pub duplicate_update_count: u16,
    /// Evidence-origin counts preserved for logs and summaries.
    pub origin_counts: EvidenceOriginUpdateCounts,
}

/// Stable demand-entry identifier for active belief diffusion.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ActiveDemandEntryId(pub u32);

/// Construction failure for bounded active demand summaries.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActiveDemandSummaryError {
    /// Demand summaries must carry at least one entry.
    EmptyDemand,
    /// Demand entry caps must be nonzero.
    ZeroEntryCap,
    /// Demand byte caps must be nonzero.
    ZeroByteCap,
    /// Demand time-to-live must be nonzero.
    ZeroTimeToLive,
    /// Demand expiration tick must be later than the issue tick.
    ExpirationBeforeIssue,
    /// Entry cap exceeds the deterministic replay maximum.
    EntryCapTooLarge,
    /// Entry count exceeded the declared cap.
    EntryCapExceeded,
    /// Demand entry ids must be unique after canonical ordering.
    DuplicateDemandEntry,
    /// Demand entries must consume at least one replay-visible byte.
    ZeroEntryBytes,
    /// Entry byte accounting overflowed the deterministic summary budget.
    DemandByteCountOverflow,
    /// Entry byte count exceeded the declared byte cap.
    DemandByteCapExceeded,
}

/// One bounded request for evidence that would improve a local belief landscape.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActiveDemandEntry {
    /// Stable entry id for canonical ordering and replay.
    pub entry_id: ActiveDemandEntryId,
    /// Hypothesis whose separation would improve the receiver landscape.
    pub hypothesis_id: AnomalyClusterId,
    /// Optional contribution class or id requested by the receiver.
    pub requested_contribution_id: Option<ContributionLedgerId>,
    /// Deterministic priority used only for allocation and forwarding order.
    pub priority: u16,
    /// Replay-visible encoded size for this demand entry.
    pub encoded_bytes: u32,
}

/// Construction input for one active belief demand summary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActiveDemandSummaryInput {
    /// Target whose belief landscape generated the demand.
    pub target_id: CodedTargetId,
    /// Inference task whose uncertainty generated the demand.
    pub task_id: InferenceTaskId,
    /// Receiver that emitted this demand summary.
    pub receiver: NodeId,
    /// Candidate demand entries.
    pub entries: Vec<ActiveDemandEntry>,
    /// Declared maximum entry count.
    pub entry_cap: u16,
    /// Declared maximum encoded demand bytes.
    pub byte_cap: u32,
    /// Demand lifetime as typed duration, not raw wall-clock time.
    pub ttl: DurationMs,
    /// Tick when the demand summary was issued.
    pub issued_at_tick: Tick,
    /// Last tick where this demand may shape priority or custody.
    pub expires_at_tick: Tick,
}

/// First-class bounded demand summary exchanged alongside coded evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActiveDemandSummary {
    /// Target whose belief landscape generated the demand.
    pub target_id: CodedTargetId,
    /// Inference task whose uncertainty generated the demand.
    pub task_id: InferenceTaskId,
    /// Receiver that emitted this demand summary.
    pub receiver: NodeId,
    /// Canonical demand entries ordered by entry id.
    pub entries: Vec<ActiveDemandEntry>,
    /// Declared maximum entry count.
    pub entry_cap: u16,
    /// Declared maximum encoded demand bytes.
    pub byte_cap: u32,
    /// Demand lifetime as typed duration.
    pub ttl: DurationMs,
    /// Tick when the demand summary was issued.
    pub issued_at_tick: Tick,
    /// Last tick where this demand may shape priority or custody.
    pub expires_at_tick: Tick,
    /// Total encoded bytes across entries.
    pub encoded_bytes: u32,
}

impl ActiveDemandSummary {
    /// Build a bounded, canonical, replay-visible demand summary.
    pub fn try_new(mut input: ActiveDemandSummaryInput) -> Result<Self, ActiveDemandSummaryError> {
        validate_demand_caps(
            input.entry_cap,
            input.byte_cap,
            input.ttl,
            input.issued_at_tick,
            input.expires_at_tick,
        )?;
        canonicalize_demand_entries(&mut input.entries)?;
        if input.entries.is_empty() {
            return Err(ActiveDemandSummaryError::EmptyDemand);
        }
        if input.entries.len() > usize::from(input.entry_cap) {
            return Err(ActiveDemandSummaryError::EntryCapExceeded);
        }
        let encoded_bytes = demand_entries_encoded_bytes(&input.entries)?;
        if encoded_bytes > input.byte_cap {
            return Err(ActiveDemandSummaryError::DemandByteCapExceeded);
        }

        Ok(Self {
            target_id: input.target_id,
            task_id: input.task_id,
            receiver: input.receiver,
            entries: input.entries,
            entry_cap: input.entry_cap,
            byte_cap: input.byte_cap,
            ttl: input.ttl,
            issued_at_tick: input.issued_at_tick,
            expires_at_tick: input.expires_at_tick,
            encoded_bytes,
        })
    }

    /// Whether this demand summary is still live.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.ttl.0 > 0
    }

    /// Whether this demand summary may still shape priority at the observed tick.
    #[must_use]
    pub fn is_live_at(&self, observed_at_tick: Tick) -> bool {
        self.issued_at_tick <= observed_at_tick && observed_at_tick <= self.expires_at_tick
    }
}

/// Active demand propagation mode used by experiments and replay labels.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActiveDemandPropagationMode {
    /// Demand summaries are not emitted.
    None,
    /// Demand summaries are exchanged only with local contacts.
    LocalOnly,
    /// Demand summaries can be carried alongside coded evidence.
    PiggybackedPeerDemand,
}

/// Receiver-indexed belief summaries for one active belief diffusion task.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReceiverIndexedBeliefState {
    /// Inference task represented by this receiver set.
    pub task_id: InferenceTaskId,
    /// Coded target represented by this receiver set.
    pub target_id: CodedTargetId,
    /// Canonical receiver summaries ordered by receiver id.
    pub receivers: Vec<ReceiverInferenceQualitySummary>,
}

/// Construction failure for receiver-indexed belief state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReceiverIndexedBeliefStateError {
    /// A receiver-indexed state must contain at least one receiver.
    EmptyReceiverSet,
    /// Receiver summaries must all name the same inference task.
    MixedInferenceTask,
    /// Receiver summaries must all name the same coded target.
    MixedTarget,
    /// Receiver ids must be unique after canonical ordering.
    DuplicateReceiver,
    /// Receiver count exceeded the serialized `u16` replay summary shape.
    ReceiverCountOverflow,
}

impl ReceiverIndexedBeliefState {
    /// Build canonical receiver-indexed belief state from local summaries.
    pub fn try_new(
        mut receivers: Vec<ReceiverInferenceQualitySummary>,
    ) -> Result<Self, ReceiverIndexedBeliefStateError> {
        let first = receivers
            .first()
            .copied()
            .ok_or(ReceiverIndexedBeliefStateError::EmptyReceiverSet)?;
        if receivers.len() > usize::from(u16::MAX) {
            return Err(ReceiverIndexedBeliefStateError::ReceiverCountOverflow);
        }
        receivers.sort_unstable_by_key(|summary| summary.receiver);
        for (index, summary) in receivers.iter().enumerate() {
            if summary.task_id != first.task_id {
                return Err(ReceiverIndexedBeliefStateError::MixedInferenceTask);
            }
            if summary.target_id != first.target_id {
                return Err(ReceiverIndexedBeliefStateError::MixedTarget);
            }
            if index > 0 && receivers[index - 1].receiver == summary.receiver {
                return Err(ReceiverIndexedBeliefStateError::DuplicateReceiver);
            }
        }

        Ok(Self {
            task_id: first.task_id,
            target_id: first.target_id,
            receivers,
        })
    }
}

/// Inputs used to generate bounded non-evidential demand from local uncertainty.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActiveDemandGenerationInput {
    /// Receiver belief quality that generated the demand.
    pub quality: ReceiverInferenceQualitySummary,
    /// Contribution ids whose arrival would improve local coverage.
    pub missing_contribution_ids: Vec<ContributionLedgerId>,
    /// Fixed-denominator coverage gap used as a priority term.
    pub coverage_gap_permille: u16,
    /// Declared maximum entry count.
    pub entry_cap: u16,
    /// Declared maximum encoded demand bytes.
    pub byte_cap: u32,
    /// Demand lifetime as typed duration.
    pub ttl: DurationMs,
    /// Tick when the demand summary is issued.
    pub issued_at_tick: Tick,
    /// Last tick where this demand may shape priority or custody.
    pub expires_at_tick: Tick,
}

/// Demand generation failure.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActiveDemandGenerationError {
    /// Missing contribution ids must be unique after canonical ordering.
    DuplicateMissingContribution,
    /// Generated summary violated bounded-demand construction rules.
    Summary(ActiveDemandSummaryError),
}

/// Generate first-class bounded demand from uncertainty without creating evidence.
pub fn generate_active_demand_summary(
    mut input: ActiveDemandGenerationInput,
) -> Result<Option<ActiveDemandSummary>, ActiveDemandGenerationError> {
    canonicalize_missing_contributions(&mut input.missing_contribution_ids)?;
    let priority = demand_priority(
        input.quality.top_hypothesis_margin,
        input.quality.uncertainty_permille,
        input.coverage_gap_permille,
    );
    if priority == 0 && input.missing_contribution_ids.is_empty() {
        return Ok(None);
    }
    let entries = demand_entries_from_generation_input(&input, priority);
    ActiveDemandSummary::try_new(ActiveDemandSummaryInput {
        target_id: input.quality.target_id,
        task_id: input.quality.task_id,
        receiver: input.quality.receiver,
        entries,
        entry_cap: input.entry_cap,
        byte_cap: input.byte_cap,
        ttl: input.ttl,
        issued_at_tick: input.issued_at_tick,
        expires_at_tick: input.expires_at_tick,
    })
    .map(Some)
    .map_err(ActiveDemandGenerationError::Summary)
}

/// Active belief messages are symmetric in transport, not in semantics.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActiveBeliefMessage {
    /// Coded evidence can carry audited contribution identity.
    CodedEvidence(CodedEvidenceRecord),
    /// Demand is first-class control communication, but never evidence.
    DemandSummary(ActiveDemandSummary),
}

impl ActiveBeliefMessage {
    /// Return contribution ids carried by evidence messages and none for demand.
    #[must_use]
    pub fn contribution_ledger_ids(&self) -> &[ContributionLedgerId] {
        match self {
            Self::CodedEvidence(evidence) => &evidence.contribution_ledger_ids,
            Self::DemandSummary(_) => &[],
        }
    }

    /// Whether this message is non-evidential demand.
    #[must_use]
    pub fn is_demand_summary(&self) -> bool {
        matches!(self, Self::DemandSummary(_))
    }

    /// Replay-visible bytes carried by this active message.
    #[must_use]
    pub fn encoded_bytes(&self) -> u32 {
        match self {
            Self::CodedEvidence(evidence) => evidence.payload_bytes,
            Self::DemandSummary(summary) => summary.encoded_bytes,
        }
    }
}

/// Replay-visible result of applying active demand around an evidence arrival.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActiveDemandReplayEvent {
    /// Tick when the demand interaction was observed.
    pub observed_at_tick: Tick,
    /// Receiver whose local uncertainty emitted the demand.
    pub receiver: NodeId,
    /// Target governed by the demand summary.
    pub target_id: CodedTargetId,
    /// Inference task governed by the demand summary.
    pub task_id: InferenceTaskId,
    /// Number of bounded entries in the summary.
    pub entry_count: u16,
    /// Encoded bytes carried by the demand summary.
    pub encoded_bytes: u32,
    /// Number of demand entries satisfied by an evidence message.
    pub satisfied_entry_count: u16,
    /// Whether the demand was expired or ignored.
    pub ignored_stale_demand: bool,
}

/// Receiver compatibility summary over guarded local decisions.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReceiverBeliefCompatibilitySummary {
    /// Inference task summarized across receivers.
    pub task_id: InferenceTaskId,
    /// Number of receivers represented.
    pub receiver_count: u16,
    /// Number of receivers with a guarded commitment.
    pub committed_receiver_count: u16,
    /// Number of committed receivers agreeing with the modal hypothesis.
    pub agreeing_receiver_count: u16,
    /// Fixed-denominator agreement score.
    pub agreement_permille: u16,
    /// Fixed-denominator divergence proxy.
    pub belief_divergence_permille: u16,
}

/// Apply an evidence message under demand-aware policy without letting demand
/// alter evidence validity or duplicate accounting.
pub fn record_active_evidence_arrival(
    receiver_rank: &mut ReceiverRankState,
    message: &ActiveBeliefMessage,
    observed_at_tick: Tick,
) -> Result<Option<FragmentArrivalClass>, ReceiverRankError> {
    let ActiveBeliefMessage::CodedEvidence(evidence) = message else {
        return Ok(None);
    };
    let Some(contribution_id) = evidence.contribution_ledger_ids.first().copied() else {
        return Ok(None);
    };
    receiver_rank
        .record_contribution_arrival(contribution_id, observed_at_tick)
        .map(Some)
}

impl AnomalyDecisionProgressSummary {
    /// Build a progress summary without mutating rank, landscape, or commitment state.
    pub fn from_state(
        landscape: &AnomalyLandscape,
        receiver_rank: &ReceiverRankState,
        commitment: &DecisionCommitmentState,
    ) -> Result<Self, DecisionCommitmentError> {
        commitment.validate_landscape(landscape)?;
        Ok(Self {
            task_id: landscape.hypotheses.task_id,
            target_id: landscape.hypotheses.target_id,
            receiver: receiver_rank.receiver,
            receiver_rank: receiver_rank.independent_rank,
            reconstructed_at_tick: receiver_rank.reconstructed_at_tick,
            committed_at_tick: commitment.committed_at_tick,
            landscape_summary: landscape.summary,
        })
    }
}

impl ReceiverInferenceQualitySummary {
    /// Build a quality summary from current state and replay-visible update events.
    pub fn from_events(
        landscape: &AnomalyLandscape,
        receiver_rank: &ReceiverRankState,
        commitment: &DecisionCommitmentState,
        events: &[LandscapeUpdateEvent],
    ) -> Result<Self, DecisionCommitmentError> {
        commitment.validate_landscape(landscape)?;
        let (innovative_update_count, duplicate_update_count, origin_counts) =
            summarize_update_events(events);
        Ok(Self {
            task_id: landscape.hypotheses.task_id,
            target_id: landscape.hypotheses.target_id,
            receiver: receiver_rank.receiver,
            receiver_rank: receiver_rank.independent_rank,
            exact_reconstruction_tick: receiver_rank.reconstructed_at_tick,
            decision_commitment_tick: commitment.committed_at_tick,
            top_hypothesis: landscape.summary.top_hypothesis,
            runner_up_hypothesis: landscape.summary.runner_up_hypothesis,
            top_hypothesis_margin: landscape.summary.top_hypothesis_margin,
            uncertainty_permille: landscape.summary.uncertainty_permille,
            energy_gap: landscape.summary.energy_gap,
            innovative_update_count,
            duplicate_update_count,
            origin_counts,
        })
    }
}

fn validate_demand_caps(
    entry_cap: u16,
    byte_cap: u32,
    ttl: DurationMs,
    issued_at_tick: Tick,
    expires_at_tick: Tick,
) -> Result<(), ActiveDemandSummaryError> {
    if entry_cap == 0 {
        return Err(ActiveDemandSummaryError::ZeroEntryCap);
    }
    if entry_cap > ACTIVE_DEMAND_ENTRY_COUNT_MAX {
        return Err(ActiveDemandSummaryError::EntryCapTooLarge);
    }
    if byte_cap == 0 {
        return Err(ActiveDemandSummaryError::ZeroByteCap);
    }
    if ttl.0 == 0 {
        return Err(ActiveDemandSummaryError::ZeroTimeToLive);
    }
    if expires_at_tick <= issued_at_tick {
        return Err(ActiveDemandSummaryError::ExpirationBeforeIssue);
    }
    Ok(())
}

fn canonicalize_demand_entries(
    entries: &mut [ActiveDemandEntry],
) -> Result<(), ActiveDemandSummaryError> {
    entries.sort_unstable_by_key(|entry| entry.entry_id);
    for (index, entry) in entries.iter().enumerate() {
        if entry.encoded_bytes == 0 {
            return Err(ActiveDemandSummaryError::ZeroEntryBytes);
        }
        if index > 0 && entries[index - 1].entry_id == entry.entry_id {
            return Err(ActiveDemandSummaryError::DuplicateDemandEntry);
        }
    }
    Ok(())
}

fn demand_entries_encoded_bytes(
    entries: &[ActiveDemandEntry],
) -> Result<u32, ActiveDemandSummaryError> {
    let mut encoded_bytes = 0_u32;
    for entry in entries {
        encoded_bytes = encoded_bytes
            .checked_add(entry.encoded_bytes)
            .ok_or(ActiveDemandSummaryError::DemandByteCountOverflow)?;
    }
    Ok(encoded_bytes)
}

fn canonicalize_missing_contributions(
    values: &mut [ContributionLedgerId],
) -> Result<(), ActiveDemandGenerationError> {
    values.sort_unstable();
    if values.windows(2).any(|window| window[0] == window[1]) {
        return Err(ActiveDemandGenerationError::DuplicateMissingContribution);
    }
    Ok(())
}

fn demand_priority(margin: i32, uncertainty_permille: u16, coverage_gap_permille: u16) -> u16 {
    let margin_gap = if margin <= 0 {
        1_000
    } else {
        1_000_u16.saturating_sub(u16::try_from(margin).unwrap_or(u16::MAX).min(1_000))
    };
    uncertainty_permille
        .max(coverage_gap_permille)
        .max(margin_gap)
}

fn demand_entries_from_generation_input(
    input: &ActiveDemandGenerationInput,
    priority: u16,
) -> Vec<ActiveDemandEntry> {
    if input.missing_contribution_ids.is_empty() {
        return vec![ActiveDemandEntry {
            entry_id: ActiveDemandEntryId(0),
            hypothesis_id: input.quality.runner_up_hypothesis,
            requested_contribution_id: None,
            priority,
            encoded_bytes: 12,
        }];
    }

    input
        .missing_contribution_ids
        .iter()
        .enumerate()
        .map(|(index, contribution_id)| ActiveDemandEntry {
            entry_id: ActiveDemandEntryId(u32::try_from(index).unwrap_or(u32::MAX)),
            hypothesis_id: input.quality.runner_up_hypothesis,
            requested_contribution_id: Some(*contribution_id),
            priority,
            encoded_bytes: 12,
        })
        .collect()
}

impl EvidenceVectorBatch {
    /// Build a canonical batch and reject duplicate contribution updates.
    pub fn try_new(
        task_id: InferenceTaskId,
        mut records: Vec<EvidenceVectorRecord>,
    ) -> Result<Self, EvidenceVectorRecordError> {
        if records.is_empty() {
            return Err(EvidenceVectorRecordError::EmptyBatch);
        }
        records.sort_unstable_by_key(|record| record.contribution_id);
        for (index, record) in records.iter().enumerate() {
            if record.task_id != task_id {
                return Err(EvidenceVectorRecordError::MixedInferenceTask);
            }
            if index > 0 && records[index - 1].contribution_id == record.contribution_id {
                return Err(EvidenceVectorRecordError::DuplicateContributionUpdate);
            }
        }

        Ok(Self { task_id, records })
    }
}

/// Apply evidence-vector records to receiver rank and anomaly landscape.
///
/// Score addition uses saturating integer arithmetic so replay cannot diverge
/// through platform-specific overflow behavior.
pub fn reduce_landscape_updates(
    receiver_rank: &ReceiverRankState,
    landscape: &AnomalyLandscape,
    updates: &[EvidenceVectorRecord],
    observed_at_tick: Tick,
) -> Result<LandscapeUpdateOutcome, LandscapeUpdateError> {
    let mut ordered_updates = updates.to_vec();
    canonicalize_landscape_updates(landscape, &mut ordered_updates)?;

    let mut next_rank = receiver_rank.clone();
    let mut next_landscape = landscape.clone();
    let mut events = Vec::with_capacity(ordered_updates.len());
    let mut innovative_update_count = 0_u16;
    let mut duplicate_update_count = 0_u16;

    for update in &ordered_updates {
        let summary_before = next_landscape.summary;
        let rank_before = next_rank.independent_rank;
        let arrival_class = next_rank
            .record_contribution_arrival(update.contribution_id, observed_at_tick)
            .map_err(|_| LandscapeUpdateError::ReceiverRankUpdateFailed)?;
        if arrival_class == FragmentArrivalClass::Innovative {
            apply_score_update(&mut next_landscape, &update.score_update);
            innovative_update_count = innovative_update_count.saturating_add(1);
        } else {
            duplicate_update_count = duplicate_update_count.saturating_add(1);
        }
        let summary_after = next_landscape.summary;
        events.push(LandscapeUpdateEvent {
            task_id: update.task_id,
            evidence_id: update.evidence_id,
            contribution_id: update.contribution_id,
            origin_mode: update.origin_mode,
            arrival_class,
            rank_before,
            rank_after: next_rank.independent_rank,
            summary_before,
            summary_after,
        });
    }

    Ok(LandscapeUpdateOutcome {
        receiver_rank: next_rank,
        landscape: next_landscape,
        events,
        innovative_update_count,
        duplicate_update_count,
    })
}

impl AnomalyLandscape {
    /// Build a canonical anomaly landscape from candidate scores.
    pub fn try_new(
        hypotheses: AnomalyHypothesisSet,
        mut scores: Vec<AnomalyHypothesisScore>,
        decision_guard: AnomalyDecisionGuard,
    ) -> Result<Self, AnomalyLandscapeError> {
        canonicalize_anomaly_scores(&hypotheses, &mut scores)?;
        let summary = summarize_anomaly_scores(&scores);

        Ok(Self {
            hypotheses,
            scores,
            decision_guard,
            summary,
        })
    }
}

fn canonicalize_anomaly_hypotheses(
    hypotheses: &mut [AnomalyClusterId],
) -> Result<(), AnomalyLandscapeError> {
    hypotheses.sort_unstable();
    if hypotheses.len() < 2 {
        return Err(AnomalyLandscapeError::TooFewHypotheses);
    }
    if hypotheses.len() > usize::from(ANOMALY_HYPOTHESIS_COUNT_MAX) {
        return Err(AnomalyLandscapeError::TooManyHypotheses);
    }
    if hypotheses.windows(2).any(|window| window[0] == window[1]) {
        return Err(AnomalyLandscapeError::DuplicateHypothesis);
    }
    Ok(())
}

fn canonicalize_anomaly_scores(
    hypotheses: &AnomalyHypothesisSet,
    scores: &mut [AnomalyHypothesisScore],
) -> Result<(), AnomalyLandscapeError> {
    scores.sort_unstable_by_key(|score| score.hypothesis_id);
    if scores.len() != hypotheses.candidate_hypotheses.len() {
        return Err(AnomalyLandscapeError::MalformedScoreVector);
    }

    for (index, score) in scores.iter().enumerate() {
        if index > 0 && scores[index - 1].hypothesis_id == score.hypothesis_id {
            return Err(AnomalyLandscapeError::DuplicateScoreHypothesis);
        }
        if hypotheses.candidate_hypotheses[index] != score.hypothesis_id {
            return Err(AnomalyLandscapeError::ScoreForUnknownHypothesis);
        }
    }

    Ok(())
}

fn summarize_anomaly_scores(scores: &[AnomalyHypothesisScore]) -> AnomalyLandscapeSummary {
    debug_assert!(scores.len() >= 2);
    let mut ranked = scores.to_vec();
    ranked.sort_unstable_by(|left, right| {
        right
            .scaled_score
            .cmp(&left.scaled_score)
            .then_with(|| left.hypothesis_id.cmp(&right.hypothesis_id))
    });
    let top = ranked[0];
    let runner_up = ranked[1];
    let margin = top.scaled_score.saturating_sub(runner_up.scaled_score);

    AnomalyLandscapeSummary {
        top_hypothesis: top.hypothesis_id,
        runner_up_hypothesis: runner_up.hypothesis_id,
        top_hypothesis_margin: margin,
        uncertainty_permille: anomaly_uncertainty_permille(margin),
        energy_gap: margin,
    }
}

fn anomaly_uncertainty_permille(margin: i32) -> u16 {
    let positive_margin = u32::try_from(margin.max(0)).unwrap_or(u32::MAX);
    let uncertainty = 1000_u32.saturating_sub(positive_margin.saturating_mul(20));
    u16::try_from(uncertainty).unwrap_or(0)
}

fn validate_evidence_vector_origin(
    evidence: &CodedEvidenceRecord,
) -> Result<(), EvidenceVectorRecordError> {
    match evidence.origin_mode {
        EvidenceOriginMode::SourceCoded => {
            if evidence.local_observation_id.is_some() {
                return Err(EvidenceVectorRecordError::MalformedLocalObservationReference);
            }
        }
        EvidenceOriginMode::LocallyGenerated => {
            if evidence.local_observation_id.is_none() {
                return Err(EvidenceVectorRecordError::MalformedLocalObservationReference);
            }
        }
        EvidenceOriginMode::RecodedAggregated => {
            if evidence.parent_evidence_ids.is_empty() {
                return Err(EvidenceVectorRecordError::AmbiguousRecodedLineage);
            }
        }
    }
    Ok(())
}

fn canonicalize_landscape_updates(
    landscape: &AnomalyLandscape,
    updates: &mut [EvidenceVectorRecord],
) -> Result<(), LandscapeUpdateError> {
    updates.sort_unstable_by_key(|update| (update.contribution_id, update.evidence_id));
    for (index, update) in updates.iter().enumerate() {
        if update.task_id != landscape.hypotheses.task_id {
            return Err(LandscapeUpdateError::TaskMismatch);
        }
        if update.target_id != landscape.hypotheses.target_id {
            return Err(LandscapeUpdateError::TargetMismatch);
        }
        if index > 0 && updates[index - 1].contribution_id == update.contribution_id {
            return Err(LandscapeUpdateError::DuplicateContributionUpdate);
        }
    }
    Ok(())
}

fn apply_score_update(landscape: &mut AnomalyLandscape, update: &[AnomalyHypothesisScore]) {
    debug_assert_eq!(landscape.scores.len(), update.len());
    for (score, delta) in landscape.scores.iter_mut().zip(update.iter()) {
        debug_assert_eq!(score.hypothesis_id, delta.hypothesis_id);
        score.scaled_score = score.scaled_score.saturating_add(delta.scaled_score);
    }
    landscape.summary = summarize_anomaly_scores(&landscape.scores);
}

fn summarize_update_events(
    events: &[LandscapeUpdateEvent],
) -> (u16, u16, EvidenceOriginUpdateCounts) {
    let mut innovative_update_count = 0_u16;
    let mut duplicate_update_count = 0_u16;
    let mut origin_counts = EvidenceOriginUpdateCounts::default();
    for event in events {
        match event.arrival_class {
            FragmentArrivalClass::Innovative => {
                innovative_update_count = innovative_update_count.saturating_add(1);
            }
            FragmentArrivalClass::Duplicate => {
                duplicate_update_count = duplicate_update_count.saturating_add(1);
            }
        }
        match event.origin_mode {
            EvidenceOriginMode::SourceCoded => {
                origin_counts.source_coded = origin_counts.source_coded.saturating_add(1);
            }
            EvidenceOriginMode::LocallyGenerated => {
                origin_counts.locally_generated = origin_counts.locally_generated.saturating_add(1);
            }
            EvidenceOriginMode::RecodedAggregated => {
                origin_counts.recoded_aggregated =
                    origin_counts.recoded_aggregated.saturating_add(1);
            }
        }
    }
    (
        innovative_update_count,
        duplicate_update_count,
        origin_counts,
    )
}

/// Classification of one received fragment relative to receiver state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FragmentArrivalClass {
    /// The fragment increased independent receiver rank.
    Innovative,
    /// The fragment repeated information already represented at the receiver.
    Duplicate,
}

/// Observer-visible custody for one fragment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FragmentCustody {
    /// Message that owns the fragment.
    pub message_id: DiffusionMessageId,
    /// Fragment being retained or forwarded.
    pub fragment_id: DiffusionFragmentId,
    /// Node currently observed with custody.
    pub custodian: NodeId,
    /// Whether the current custodian is expected to retain the fragment.
    pub retained: bool,
}

/// Receiver-local reconstruction progress.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReceiverRankState {
    /// Message being reconstructed.
    pub message_id: DiffusionMessageId,
    /// Receiver whose rank is measured.
    pub receiver: NodeId,
    /// Independent rank required for exact reconstruction.
    pub required_rank: u16,
    /// Current independent rank.
    pub independent_rank: u16,
    /// Canonical contribution ids already counted by this receiver.
    pub accepted_contribution_ids: Vec<ContributionLedgerId>,
    /// Count of arrivals that increased rank.
    pub innovative_arrivals: u16,
    /// Count of arrivals that did not increase rank.
    pub duplicate_arrivals: u16,
    /// Tick when reconstruction first became true.
    pub reconstructed_at_tick: Option<Tick>,
}

impl ReceiverRankState {
    /// Construct empty receiver state for one reconstruction target.
    pub fn try_new(
        message_id: DiffusionMessageId,
        receiver: NodeId,
        required_rank: u16,
    ) -> Result<Self, ReceiverRankError> {
        if required_rank == 0 {
            return Err(ReceiverRankError::ZeroRequiredRank);
        }

        Ok(Self {
            message_id,
            receiver,
            required_rank,
            independent_rank: 0,
            accepted_contribution_ids: Vec::new(),
            innovative_arrivals: 0,
            duplicate_arrivals: 0,
            reconstructed_at_tick: None,
        })
    }

    /// Construct empty receiver state from a validated coding window.
    #[must_use]
    pub fn for_window(
        message_id: DiffusionMessageId,
        receiver: NodeId,
        coding_window: CodingWindow,
    ) -> Self {
        Self {
            message_id,
            receiver,
            required_rank: coding_window.required_rank,
            independent_rank: 0,
            accepted_contribution_ids: Vec::new(),
            innovative_arrivals: 0,
            duplicate_arrivals: 0,
            reconstructed_at_tick: None,
        }
    }

    /// Classify and record a contribution arrival by canonical contribution id.
    pub fn record_contribution_arrival(
        &mut self,
        contribution_id: ContributionLedgerId,
        observed_at_tick: Tick,
    ) -> Result<FragmentArrivalClass, ReceiverRankError> {
        if insert_contribution_id(&mut self.accepted_contribution_ids, contribution_id)? {
            self.independent_rank = u16::try_from(self.accepted_contribution_ids.len())
                .map_err(|_| ReceiverRankError::ContributionLedgerFull)?;
            self.innovative_arrivals = self.innovative_arrivals.saturating_add(1);
            self.record_reconstruction_if_complete(observed_at_tick);
            return Ok(FragmentArrivalClass::Innovative);
        }

        self.duplicate_arrivals = self.duplicate_arrivals.saturating_add(1);
        self.record_reconstruction_if_complete(observed_at_tick);
        Ok(FragmentArrivalClass::Duplicate)
    }

    /// Record the first reconstruction tick if rank has reached the threshold.
    pub fn record_reconstruction_if_complete(&mut self, observed_at_tick: Tick) -> Option<Tick> {
        if self.reconstructed_at_tick.is_none() && self.independent_rank >= self.required_rank {
            self.reconstructed_at_tick = Some(observed_at_tick);
        }
        self.reconstructed_at_tick
    }

    /// Whether exact reconstruction has been reached.
    #[must_use]
    pub fn is_reconstructed(&self) -> bool {
        self.reconstructed_at_tick.is_some()
    }
}

fn insert_contribution_id(
    accepted_contribution_ids: &mut Vec<ContributionLedgerId>,
    contribution_id: ContributionLedgerId,
) -> Result<bool, ReceiverRankError> {
    match accepted_contribution_ids.binary_search(&contribution_id) {
        Ok(_) => Ok(false),
        Err(index) => {
            if accepted_contribution_ids.len() >= usize::from(u16::MAX) {
                return Err(ReceiverRankError::ContributionLedgerFull);
            }
            accepted_contribution_ids.insert(index, contribution_id);
            Ok(true)
        }
    }
}

/// Aggregate reconstruction status over the observed receiver population.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReconstructionQuorum {
    /// Message being reconstructed.
    pub message_id: DiffusionMessageId,
    /// Rank required for reconstruction.
    pub required_rank: u16,
    /// Number of receivers represented by this aggregate.
    pub observed_receivers: u16,
    /// Number of represented receivers at or above the required rank.
    pub complete_receivers: u16,
    /// Minimum observed independent rank across represented receivers.
    pub min_independent_rank: u16,
}

impl ReconstructionQuorum {
    /// Whether every represented receiver has reached reconstruction rank.
    #[must_use]
    pub fn is_complete(self) -> bool {
        self.observed_receivers > 0
            && self.complete_receivers == self.observed_receivers
            && self.min_independent_rank >= self.required_rank
    }
}

/// Deterministic pressure signal for local coded diffusion control.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionPressure {
    /// Need to keep fragments in bounded custody, in permille.
    pub custody_pressure_permille: u16,
    /// Need to move innovative fragments, in permille.
    pub innovation_pressure_permille: u16,
    /// Need to suppress duplicate movement, in permille.
    pub duplicate_pressure_permille: u16,
}

impl DiffusionPressure {
    /// Clamp pressure components to the normalized deterministic range.
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            custody_pressure_permille: self.custody_pressure_permille.min(1000),
            innovation_pressure_permille: self.innovation_pressure_permille.min(1000),
            duplicate_pressure_permille: self.duplicate_pressure_permille.min(1000),
        }
    }
}

/// Reduced observer belief about fragment spread and reconstruction progress.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FragmentSpreadBelief {
    /// Message being observed.
    pub message_id: DiffusionMessageId,
    /// Distinct fragments observed in custody or movement.
    pub observed_fragment_count: u16,
    /// Distinct custodians observed for this message.
    pub custody_node_count: u16,
    /// Current reconstruction quorum summary.
    pub reconstruction_quorum: ReconstructionQuorum,
}

/// Local order parameters for near-critical coded diffusion control.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionOrderParameters {
    /// Diffusion/innovation/duplicate pressure vector.
    pub pressure: DiffusionPressure,
    /// Bounded storage pressure, in permille.
    pub storage_pressure_permille: u16,
    /// Rank still needed before the local reconstruction target is complete.
    pub rank_deficit: u16,
    /// Duplicate arrivals as a normalized local pressure, in permille.
    pub duplicate_arrival_permille: u16,
}

impl DiffusionOrderParameters {
    /// Clamp normalized pressure components to the deterministic range.
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            pressure: self.pressure.clamped(),
            storage_pressure_permille: self.storage_pressure_permille.min(1000),
            rank_deficit: self.rank_deficit,
            duplicate_arrival_permille: self.duplicate_arrival_permille.min(1000),
        }
    }
}

/// Near-critical control state for local coded diffusion decisions.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NearCriticalControlState {
    /// Current reduced order parameters.
    pub order_parameters: DiffusionOrderParameters,
    /// Consecutive rounds spent inside the controller's stable band.
    pub stable_band_rounds: u16,
    /// Whether the controller should currently prefer retention over spread.
    pub retention_biased: bool,
}

/// Bounded fragment holding policy.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FragmentRetentionPolicy {
    /// Maximum fragments retained for one message.
    pub fragment_budget: u16,
    /// Pressure threshold at which custody is preferred, in permille.
    pub custody_threshold_permille: u16,
    /// Whether duplicate fragments are evicted before innovative fragments.
    pub evict_duplicates_first: bool,
}

impl FragmentRetentionPolicy {
    /// Construct a normalized bounded retention policy.
    #[must_use]
    pub fn new(
        fragment_budget: u16,
        custody_threshold_permille: u16,
        evict_duplicates_first: bool,
    ) -> Self {
        Self {
            fragment_budget,
            custody_threshold_permille: custody_threshold_permille.min(1000),
            evict_duplicates_first,
        }
    }
}

/// Delayed fragment arrival or forwarding observation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelayedFragmentEvent {
    /// Message that owns the fragment.
    pub message_id: DiffusionMessageId,
    /// Fragment being moved.
    pub fragment_id: DiffusionFragmentId,
    /// Sender observed for the movement.
    pub from_node: NodeId,
    /// Receiver observed for the movement.
    pub to_node: NodeId,
    /// Deterministic observation tick.
    pub observed_at_tick: Tick,
    /// Whether the receiver gained independent rank.
    pub arrival_class: FragmentArrivalClass,
}

/// Replay-facing coded-diffusion event vocabulary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FragmentReplayEvent {
    /// Contact opportunity considered for a fragment.
    Contact {
        /// Sender observed in the contact.
        from_node: NodeId,
        /// Receiver observed in the contact.
        to_node: NodeId,
        /// Deterministic observation tick.
        observed_at_tick: Tick,
    },
    /// Fragment movement was attempted.
    Forwarded(DelayedFragmentEvent),
    /// Fragment movement reached the receiver.
    Arrived(DelayedFragmentEvent),
    /// Reconstruction quorum was updated.
    Reconstruction(ReconstructionQuorum),
}

/// Role assigned to private protocol hooks retained for coded diffusion.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PrivateProtocolRole {
    /// Bounded summary exchange for fragment/rank/custody observations.
    BoundedSummaryExchange,
    /// Local coordination over fragment-control decisions.
    FragmentControlCoordination,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id16(fill: u8) -> [u8; 16] {
        [fill; 16]
    }

    fn node_id(fill: u8) -> NodeId {
        NodeId([fill; 32])
    }

    fn source_input() -> CodedEvidenceRecordInput {
        CodedEvidenceRecordInput {
            target_id: CodedTargetId(10),
            message_id: DiffusionMessageId(id16(1)),
            evidence_id: CodedEvidenceId(1),
            origin_mode: EvidenceOriginMode::SourceCoded,
            fragment_id: Some(DiffusionFragmentId(id16(2))),
            rank_id: Some(CodingRankId(3)),
            holder: node_id(4),
            local_observation_id: None,
            parent_evidence_ids: Vec::new(),
            contribution_ledger_ids: vec![ContributionLedgerId(3)],
            payload_bytes: 32,
        }
    }

    fn source_record(
        evidence_id: u32,
        fragment_fill: u8,
        contribution_id: u32,
    ) -> CodedEvidenceRecord {
        CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(evidence_id),
            fragment_id: Some(DiffusionFragmentId(id16(fragment_fill))),
            rank_id: Some(CodingRankId(contribution_id)),
            contribution_ledger_ids: vec![ContributionLedgerId(contribution_id)],
            ..source_input()
        })
        .expect("source-coded evidence record")
    }

    fn demand_entry(entry_id: u32, priority: u16) -> ActiveDemandEntry {
        ActiveDemandEntry {
            entry_id: ActiveDemandEntryId(entry_id),
            hypothesis_id: AnomalyClusterId(2),
            requested_contribution_id: Some(ContributionLedgerId(entry_id)),
            priority,
            encoded_bytes: 12,
        }
    }

    fn demand_summary() -> ActiveDemandSummary {
        ActiveDemandSummary::try_new(ActiveDemandSummaryInput {
            target_id: CodedTargetId(10),
            task_id: InferenceTaskId(20),
            receiver: node_id(7),
            entries: vec![demand_entry(2, 50), demand_entry(1, 80)],
            entry_cap: 4,
            byte_cap: 64,
            ttl: DurationMs(250),
            issued_at_tick: Tick(10),
            expires_at_tick: Tick(20),
        })
        .expect("active demand summary")
    }

    fn quality_summary(receiver_fill: u8) -> ReceiverInferenceQualitySummary {
        ReceiverInferenceQualitySummary {
            task_id: InferenceTaskId(20),
            target_id: CodedTargetId(10),
            receiver: node_id(receiver_fill),
            receiver_rank: 1,
            exact_reconstruction_tick: None,
            decision_commitment_tick: None,
            top_hypothesis: AnomalyClusterId(1),
            runner_up_hypothesis: AnomalyClusterId(2),
            top_hypothesis_margin: 120,
            uncertainty_permille: 640,
            energy_gap: 120,
            innovative_update_count: 1,
            duplicate_update_count: 0,
            origin_counts: EvidenceOriginUpdateCounts::default(),
        }
    }

    fn record_evidence_contributions(
        state: &mut ReceiverRankState,
        evidence: &CodedEvidenceRecord,
        observed_at_tick: Tick,
    ) {
        for contribution_id in &evidence.contribution_ledger_ids {
            state
                .record_contribution_arrival(*contribution_id, observed_at_tick)
                .expect("record contribution arrival");
        }
    }

    #[test]
    fn active_demand_summaries_are_bounded_and_canonical() {
        let summary = demand_summary();

        assert_eq!(summary.entries[0].entry_id, ActiveDemandEntryId(1));
        assert_eq!(summary.entries[1].entry_id, ActiveDemandEntryId(2));
        assert_eq!(summary.encoded_bytes, 24);
        assert!(summary.is_live());
        assert!(summary.is_live_at(Tick(15)));
        assert!(!summary.is_live_at(Tick(21)));
    }

    #[test]
    fn active_demand_rejects_unbounded_or_ambiguous_inputs() {
        let duplicate = ActiveDemandSummary::try_new(ActiveDemandSummaryInput {
            target_id: CodedTargetId(10),
            task_id: InferenceTaskId(20),
            receiver: node_id(7),
            entries: vec![demand_entry(1, 50), demand_entry(1, 80)],
            entry_cap: 4,
            byte_cap: 64,
            ttl: DurationMs(250),
            issued_at_tick: Tick(1),
            expires_at_tick: Tick(2),
        });
        assert_eq!(
            duplicate,
            Err(ActiveDemandSummaryError::DuplicateDemandEntry)
        );

        let expired = ActiveDemandSummary::try_new(ActiveDemandSummaryInput {
            ttl: DurationMs(0),
            entries: vec![demand_entry(1, 50)],
            target_id: CodedTargetId(10),
            task_id: InferenceTaskId(20),
            receiver: node_id(7),
            entry_cap: 4,
            byte_cap: 64,
            issued_at_tick: Tick(1),
            expires_at_tick: Tick(2),
        });
        assert_eq!(expired, Err(ActiveDemandSummaryError::ZeroTimeToLive));

        let stale_at_issue = ActiveDemandSummary::try_new(ActiveDemandSummaryInput {
            ttl: DurationMs(250),
            entries: vec![demand_entry(1, 50)],
            target_id: CodedTargetId(10),
            task_id: InferenceTaskId(20),
            receiver: node_id(7),
            entry_cap: 4,
            byte_cap: 64,
            issued_at_tick: Tick(2),
            expires_at_tick: Tick(2),
        });
        assert_eq!(
            stale_at_issue,
            Err(ActiveDemandSummaryError::ExpirationBeforeIssue)
        );
    }

    #[test]
    fn demand_messages_are_first_class_but_non_evidential() {
        let demand = ActiveBeliefMessage::DemandSummary(demand_summary());
        let evidence = ActiveBeliefMessage::CodedEvidence(source_record(1, 2, 3));

        assert!(demand.is_demand_summary());
        assert!(demand.contribution_ledger_ids().is_empty());
        assert_eq!(demand.encoded_bytes(), 24);
        assert_eq!(
            evidence.contribution_ledger_ids(),
            &[ContributionLedgerId(3)]
        );
        assert_eq!(evidence.encoded_bytes(), 32);
    }

    #[test]
    fn active_evidence_arrival_preserves_duplicate_non_inflation() {
        let evidence = ActiveBeliefMessage::CodedEvidence(source_record(1, 2, 3));
        let mut state = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 2)
            .expect("rank state");

        let first = record_active_evidence_arrival(&mut state, &evidence, Tick(1))
            .expect("first active evidence");
        let second = record_active_evidence_arrival(&mut state, &evidence, Tick(2))
            .expect("second active evidence");
        let demand = record_active_evidence_arrival(
            &mut state,
            &ActiveBeliefMessage::DemandSummary(demand_summary()),
            Tick(3),
        )
        .expect("demand arrival");

        assert_eq!(first, Some(FragmentArrivalClass::Innovative));
        assert_eq!(second, Some(FragmentArrivalClass::Duplicate));
        assert_eq!(demand, None);
        assert_eq!(state.independent_rank, 1);
        assert_eq!(state.innovative_arrivals, 1);
        assert_eq!(state.duplicate_arrivals, 1);
    }

    #[test]
    fn receiver_indexed_belief_state_is_canonical_and_receiver_unique() {
        let state = ReceiverIndexedBeliefState::try_new(vec![
            quality_summary(9),
            quality_summary(7),
            quality_summary(8),
        ])
        .expect("receiver-indexed belief state");

        assert_eq!(state.receivers[0].receiver, node_id(7));
        assert_eq!(state.receivers[1].receiver, node_id(8));
        assert_eq!(state.receivers[2].receiver, node_id(9));

        let duplicate =
            ReceiverIndexedBeliefState::try_new(vec![quality_summary(7), quality_summary(7)]);
        assert_eq!(
            duplicate,
            Err(ReceiverIndexedBeliefStateError::DuplicateReceiver)
        );
    }

    #[test]
    fn active_demand_generation_uses_uncertainty_margin_and_missing_coverage() {
        let summary = generate_active_demand_summary(ActiveDemandGenerationInput {
            quality: quality_summary(7),
            missing_contribution_ids: vec![ContributionLedgerId(8), ContributionLedgerId(3)],
            coverage_gap_permille: 700,
            entry_cap: 4,
            byte_cap: 64,
            ttl: DurationMs(250),
            issued_at_tick: Tick(5),
            expires_at_tick: Tick(9),
        })
        .expect("generated demand")
        .expect("useful demand");

        assert_eq!(summary.entries.len(), 2);
        assert_eq!(
            summary.entries[0].requested_contribution_id,
            Some(ContributionLedgerId(3))
        );
        assert_eq!(summary.entries[0].priority, 880);
        assert!(ActiveBeliefMessage::DemandSummary(summary)
            .contribution_ledger_ids()
            .is_empty());
    }

    #[test]
    fn active_demand_generation_rejects_ambiguous_missing_contributions() {
        let generated = generate_active_demand_summary(ActiveDemandGenerationInput {
            quality: quality_summary(7),
            missing_contribution_ids: vec![ContributionLedgerId(3), ContributionLedgerId(3)],
            coverage_gap_permille: 0,
            entry_cap: 4,
            byte_cap: 64,
            ttl: DurationMs(250),
            issued_at_tick: Tick(5),
            expires_at_tick: Tick(9),
        });

        assert_eq!(
            generated,
            Err(ActiveDemandGenerationError::DuplicateMissingContribution)
        );
    }

    fn anomaly_hypotheses() -> AnomalyHypothesisSet {
        AnomalyHypothesisSet::try_new(
            InferenceTaskId(70),
            CodedTargetId(10),
            vec![
                AnomalyClusterId(4),
                AnomalyClusterId(1),
                AnomalyClusterId(3),
                AnomalyClusterId(0),
                AnomalyClusterId(2),
            ],
            Some(AnomalyClusterId(3)),
        )
        .expect("anomaly hypotheses")
    }

    fn anomaly_guard() -> AnomalyDecisionGuard {
        AnomalyDecisionGuard::try_new(12, 3).expect("anomaly decision guard")
    }

    fn anomaly_scores(values: &[(u16, i32)]) -> Vec<AnomalyHypothesisScore> {
        values
            .iter()
            .map(|(hypothesis_id, scaled_score)| AnomalyHypothesisScore {
                hypothesis_id: AnomalyClusterId(*hypothesis_id),
                scaled_score: *scaled_score,
            })
            .collect()
    }

    #[test]
    fn anomaly_landscape_canonicalizes_candidates_and_scores() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(4, 2), (1, 8), (3, 8), (0, 1), (2, 3)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");

        assert_eq!(
            landscape.hypotheses.candidate_hypotheses,
            vec![
                AnomalyClusterId(0),
                AnomalyClusterId(1),
                AnomalyClusterId(2),
                AnomalyClusterId(3),
                AnomalyClusterId(4),
            ]
        );
        assert_eq!(
            landscape
                .scores
                .iter()
                .map(|score| score.hypothesis_id)
                .collect::<Vec<_>>(),
            landscape.hypotheses.candidate_hypotheses
        );
        assert_eq!(landscape.summary.top_hypothesis, AnomalyClusterId(1));
        assert_eq!(landscape.summary.runner_up_hypothesis, AnomalyClusterId(3));
        assert_eq!(landscape.summary.top_hypothesis_margin, 0);
        assert_eq!(landscape.summary.uncertainty_permille, 1000);
        assert_eq!(landscape.summary.energy_gap, 0);
        assert_eq!(landscape.hypotheses.hypothesis_count(), 5);
    }

    #[test]
    fn anomaly_landscape_rejects_invalid_or_ambiguous_shapes() {
        assert_eq!(
            AnomalyHypothesisSet::try_new(
                InferenceTaskId(70),
                CodedTargetId(10),
                vec![AnomalyClusterId(1)],
                Some(AnomalyClusterId(1)),
            ),
            Err(AnomalyLandscapeError::TooFewHypotheses)
        );
        assert_eq!(
            AnomalyHypothesisSet::try_new(
                InferenceTaskId(70),
                CodedTargetId(10),
                vec![AnomalyClusterId(1), AnomalyClusterId(1)],
                Some(AnomalyClusterId(1)),
            ),
            Err(AnomalyLandscapeError::DuplicateHypothesis)
        );
        assert_eq!(
            AnomalyHypothesisSet::try_new(
                InferenceTaskId(70),
                CodedTargetId(10),
                vec![AnomalyClusterId(1), AnomalyClusterId(2)],
                Some(AnomalyClusterId(3)),
            ),
            Err(AnomalyLandscapeError::HiddenHypothesisMissing)
        );
        assert_eq!(
            AnomalyDecisionGuard::try_new(-1, 1),
            Err(AnomalyLandscapeError::NegativeDecisionMarginThreshold)
        );
        assert_eq!(
            AnomalyDecisionGuard::try_new(1, 0),
            Err(AnomalyLandscapeError::ZeroMinimumDecisionEvidence)
        );
    }

    #[test]
    fn landscape_score_vectors_must_match_candidates_exactly() {
        assert_eq!(
            AnomalyLandscape::try_new(
                anomaly_hypotheses(),
                anomaly_scores(&[(0, 1), (1, 2), (2, 3), (3, 4)]),
                anomaly_guard(),
            )
            .map(|landscape| landscape.summary),
            Err(AnomalyLandscapeError::MalformedScoreVector)
        );
        assert_eq!(
            AnomalyLandscape::try_new(
                anomaly_hypotheses(),
                anomaly_scores(&[(0, 1), (1, 2), (2, 3), (3, 4), (3, 5)]),
                anomaly_guard(),
            )
            .map(|landscape| landscape.summary),
            Err(AnomalyLandscapeError::DuplicateScoreHypothesis)
        );
        assert_eq!(
            AnomalyLandscape::try_new(
                anomaly_hypotheses(),
                anomaly_scores(&[(0, 1), (1, 2), (2, 3), (3, 4), (9, 5)]),
                anomaly_guard(),
            )
            .map(|landscape| landscape.summary),
            Err(AnomalyLandscapeError::ScoreForUnknownHypothesis)
        );
    }

    #[test]
    fn landscape_summary_uses_integer_margin_uncertainty_and_gap() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, -2), (1, 4), (2, 6), (3, 21), (4, 9)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");

        assert_eq!(landscape.summary.top_hypothesis, AnomalyClusterId(3));
        assert_eq!(landscape.summary.runner_up_hypothesis, AnomalyClusterId(4));
        assert_eq!(landscape.summary.top_hypothesis_margin, 12);
        assert_eq!(landscape.summary.uncertainty_permille, 760);
        assert_eq!(landscape.summary.energy_gap, 12);
    }

    #[test]
    fn evidence_vector_accepts_source_coded_score_update() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");
        let evidence = source_record(1, 2, 3);

        let record = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &evidence,
            ContributionLedgerId(3),
            anomaly_scores(&[(4, 0), (3, 9), (2, 0), (1, 0), (0, 0)]),
            Some(AnomalyEvidenceClass(1)),
        )
        .expect("source-coded evidence vector");

        assert_eq!(record.evidence_id, CodedEvidenceId(1));
        assert_eq!(record.contribution_id, ContributionLedgerId(3));
        assert_eq!(record.origin_mode, EvidenceOriginMode::SourceCoded);
        assert_eq!(record.local_observation_id, None);
        assert_eq!(record.parent_evidence_ids, Vec::new());
        assert_eq!(record.payload_bytes, 32);
        assert_eq!(
            record
                .score_update
                .iter()
                .map(|score| score.hypothesis_id)
                .collect::<Vec<_>>(),
            landscape.hypotheses.candidate_hypotheses
        );
    }

    #[test]
    fn evidence_vector_preserves_local_observation_origin() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");
        let local = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(2),
            origin_mode: EvidenceOriginMode::LocallyGenerated,
            fragment_id: None,
            rank_id: None,
            local_observation_id: Some(LocalObservationId(44)),
            contribution_ledger_ids: vec![ContributionLedgerId(10_044)],
            ..source_input()
        })
        .expect("local evidence");

        let record = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &local,
            ContributionLedgerId(10_044),
            anomaly_scores(&[(0, -1), (1, 0), (2, 1), (3, 5), (4, 0)]),
            Some(AnomalyEvidenceClass(2)),
        )
        .expect("local evidence vector");

        assert_eq!(record.origin_mode, EvidenceOriginMode::LocallyGenerated);
        assert_eq!(record.local_observation_id, Some(LocalObservationId(44)));
        assert_eq!(record.evidence_class, Some(AnomalyEvidenceClass(2)));
    }

    #[test]
    fn evidence_vector_preserves_recoded_parent_lineage() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");
        let recoded = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(7),
            origin_mode: EvidenceOriginMode::RecodedAggregated,
            fragment_id: None,
            rank_id: None,
            local_observation_id: Some(LocalObservationId(45)),
            parent_evidence_ids: vec![CodedEvidenceId(2), CodedEvidenceId(1)],
            contribution_ledger_ids: vec![ContributionLedgerId(100), ContributionLedgerId(3)],
            ..source_input()
        })
        .expect("recoded evidence");

        let record = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &recoded,
            ContributionLedgerId(100),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 6), (4, 1)]),
            Some(AnomalyEvidenceClass(3)),
        )
        .expect("recoded evidence vector");

        assert_eq!(record.origin_mode, EvidenceOriginMode::RecodedAggregated);
        assert_eq!(
            record.parent_evidence_ids,
            vec![CodedEvidenceId(1), CodedEvidenceId(2)]
        );
        assert_eq!(record.local_observation_id, Some(LocalObservationId(45)));
    }

    #[test]
    fn evidence_vector_rejects_incompatible_contribution_or_shape() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");
        let evidence = source_record(1, 2, 3);

        assert_eq!(
            EvidenceVectorRecord::try_from_evidence(
                &landscape,
                &evidence,
                ContributionLedgerId(9),
                anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 9), (4, 0)]),
                None,
            ),
            Err(EvidenceVectorRecordError::ContributionNotInEvidence)
        );
        assert_eq!(
            EvidenceVectorRecord::try_from_evidence(
                &landscape,
                &evidence,
                ContributionLedgerId(3),
                Vec::new(),
                None,
            ),
            Err(EvidenceVectorRecordError::EmptyScoreUpdate)
        );
        assert_eq!(
            EvidenceVectorRecord::try_from_evidence(
                &landscape,
                &evidence,
                ContributionLedgerId(3),
                anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 9)]),
                None,
            ),
            Err(EvidenceVectorRecordError::MalformedScoreUpdate)
        );
    }

    #[test]
    fn evidence_vector_batch_rejects_duplicate_contribution_ids() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");
        let first = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &source_record(1, 2, 3),
            ContributionLedgerId(3),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 9), (4, 0)]),
            None,
        )
        .expect("first evidence vector");
        let second = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &source_record(2, 3, 3),
            ContributionLedgerId(3),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 7), (4, 0)]),
            None,
        )
        .expect("second evidence vector");

        assert_eq!(
            EvidenceVectorBatch::try_new(landscape.hypotheses.task_id, vec![second, first]),
            Err(EvidenceVectorRecordError::DuplicateContributionUpdate)
        );
    }

    #[test]
    fn landscape_update_applies_single_innovative_vector() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");
        let receiver_rank = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 3)
            .expect("receiver rank");
        let update = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &source_record(1, 2, 3),
            ContributionLedgerId(3),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 9), (4, 1)]),
            None,
        )
        .expect("evidence vector");

        let outcome = reduce_landscape_updates(&receiver_rank, &landscape, &[update], Tick(20))
            .expect("landscape update");

        assert_eq!(outcome.receiver_rank.independent_rank, 1);
        assert_eq!(outcome.innovative_update_count, 1);
        assert_eq!(outcome.duplicate_update_count, 0);
        assert_eq!(
            outcome.landscape.summary.top_hypothesis,
            AnomalyClusterId(3)
        );
        assert_eq!(outcome.landscape.summary.top_hypothesis_margin, 8);
        assert_eq!(outcome.landscape.summary.uncertainty_permille, 840);
        assert_eq!(outcome.landscape.summary.energy_gap, 8);
        assert_eq!(
            outcome.events[0].arrival_class,
            FragmentArrivalClass::Innovative
        );
        assert_eq!(outcome.events[0].rank_before, 0);
        assert_eq!(outcome.events[0].rank_after, 1);
    }

    #[test]
    fn landscape_update_duplicate_preserves_quality() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 5), (4, 0)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");
        let mut receiver_rank =
            ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 3)
                .expect("receiver rank");
        receiver_rank
            .record_contribution_arrival(ContributionLedgerId(3), Tick(19))
            .expect("seed duplicate contribution");
        let update = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &source_record(1, 2, 3),
            ContributionLedgerId(3),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 9), (4, 0)]),
            None,
        )
        .expect("duplicate evidence vector");

        let outcome = reduce_landscape_updates(&receiver_rank, &landscape, &[update], Tick(20))
            .expect("landscape update");

        assert_eq!(outcome.receiver_rank.independent_rank, 1);
        assert_eq!(outcome.receiver_rank.duplicate_arrivals, 1);
        assert_eq!(outcome.innovative_update_count, 0);
        assert_eq!(outcome.duplicate_update_count, 1);
        assert_eq!(outcome.landscape.scores, landscape.scores);
        assert_eq!(outcome.landscape.summary, landscape.summary);
        assert_eq!(
            outcome.events[0].arrival_class,
            FragmentArrivalClass::Duplicate
        );
        assert_eq!(
            outcome.events[0].summary_before,
            outcome.events[0].summary_after
        );
    }

    #[test]
    fn landscape_update_is_deterministic_across_input_order() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");
        let receiver_rank = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 3)
            .expect("receiver rank");
        let first = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &source_record(1, 2, 3),
            ContributionLedgerId(3),
            anomaly_scores(&[(0, 0), (1, 2), (2, 0), (3, 5), (4, 0)]),
            None,
        )
        .expect("first update");
        let second = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &source_record(2, 3, 4),
            ContributionLedgerId(4),
            anomaly_scores(&[(0, 0), (1, 7), (2, 0), (3, 1), (4, 0)]),
            None,
        )
        .expect("second update");

        let left = reduce_landscape_updates(
            &receiver_rank,
            &landscape,
            &[second.clone(), first.clone()],
            Tick(20),
        )
        .expect("left update");
        let right =
            reduce_landscape_updates(&receiver_rank, &landscape, &[first, second], Tick(20))
                .expect("right update");

        assert_eq!(left.receiver_rank, right.receiver_rank);
        assert_eq!(left.landscape, right.landscape);
        assert_eq!(left.events, right.events);
        assert_eq!(left.landscape.summary.top_hypothesis, AnomalyClusterId(1));
        assert_eq!(
            left.landscape.summary.runner_up_hypothesis,
            AnomalyClusterId(3)
        );
    }

    #[test]
    fn landscape_update_rejects_duplicate_updates_in_one_input() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");
        let receiver_rank = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 3)
            .expect("receiver rank");
        let first = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &source_record(1, 2, 3),
            ContributionLedgerId(3),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 5), (4, 0)]),
            None,
        )
        .expect("first update");
        let second = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &source_record(2, 3, 3),
            ContributionLedgerId(3),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 1), (4, 0)]),
            None,
        )
        .expect("second update");

        assert_eq!(
            reduce_landscape_updates(&receiver_rank, &landscape, &[first, second], Tick(20))
                .map(|outcome| outcome.innovative_update_count),
            Err(LandscapeUpdateError::DuplicateContributionUpdate)
        );
    }

    #[test]
    fn commitment_can_occur_before_exact_reconstruction() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 24), (4, 4)]),
            AnomalyDecisionGuard::try_new(12, 2).expect("decision guard"),
        )
        .expect("anomaly landscape");
        let mut receiver_rank =
            ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 5)
                .expect("receiver rank");
        receiver_rank
            .record_contribution_arrival(ContributionLedgerId(3), Tick(20))
            .expect("first contribution");
        receiver_rank
            .record_contribution_arrival(ContributionLedgerId(4), Tick(21))
            .expect("second contribution");
        let mut commitment = DecisionCommitmentState::new(
            landscape.hypotheses.task_id,
            landscape.hypotheses.target_id,
        );

        assert_eq!(
            commitment.check_commitment(&landscape, &receiver_rank, Tick(22)),
            Ok(Some(Tick(22)))
        );
        assert_eq!(receiver_rank.reconstructed_at_tick, None);
        assert_eq!(commitment.committed_hypothesis, Some(AnomalyClusterId(3)));
    }

    #[test]
    fn reconstruction_can_occur_without_decision_commitment() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 1), (2, 0), (3, 1), (4, 0)]),
            AnomalyDecisionGuard::try_new(10, 1).expect("decision guard"),
        )
        .expect("anomaly landscape");
        let mut receiver_rank =
            ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 2)
                .expect("receiver rank");
        receiver_rank
            .record_contribution_arrival(ContributionLedgerId(3), Tick(20))
            .expect("first contribution");
        receiver_rank
            .record_contribution_arrival(ContributionLedgerId(4), Tick(21))
            .expect("second contribution");
        let mut commitment = DecisionCommitmentState::new(
            landscape.hypotheses.task_id,
            landscape.hypotheses.target_id,
        );

        assert!(receiver_rank.is_reconstructed());
        assert_eq!(
            commitment.check_commitment(&landscape, &receiver_rank, Tick(22)),
            Ok(None)
        );
        assert_eq!(commitment.committed_at_tick, None);
    }

    #[test]
    fn commitment_requires_minimum_independent_evidence_guard() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 30), (4, 1)]),
            AnomalyDecisionGuard::try_new(12, 2).expect("decision guard"),
        )
        .expect("anomaly landscape");
        let mut receiver_rank =
            ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 5)
                .expect("receiver rank");
        receiver_rank
            .record_contribution_arrival(ContributionLedgerId(3), Tick(20))
            .expect("first contribution");
        let mut commitment = DecisionCommitmentState::new(
            landscape.hypotheses.task_id,
            landscape.hypotheses.target_id,
        );

        assert_eq!(
            commitment.check_commitment(&landscape, &receiver_rank, Tick(21)),
            Ok(None)
        );
        assert_eq!(commitment.committed_at_tick, None);
    }

    #[test]
    fn repeated_commitment_checks_preserve_first_tick() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 24), (4, 4)]),
            AnomalyDecisionGuard::try_new(12, 1).expect("decision guard"),
        )
        .expect("anomaly landscape");
        let mut receiver_rank =
            ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 5)
                .expect("receiver rank");
        receiver_rank
            .record_contribution_arrival(ContributionLedgerId(3), Tick(20))
            .expect("first contribution");
        let mut commitment = DecisionCommitmentState::new(
            landscape.hypotheses.task_id,
            landscape.hypotheses.target_id,
        );

        assert_eq!(
            commitment.check_commitment(&landscape, &receiver_rank, Tick(21)),
            Ok(Some(Tick(21)))
        );
        assert_eq!(
            commitment.check_commitment(&landscape, &receiver_rank, Tick(30)),
            Ok(Some(Tick(21)))
        );
        assert_eq!(commitment.committed_at_tick, Some(Tick(21)));
    }

    #[test]
    fn decision_progress_summary_reports_quality_before_thresholds() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 4), (2, 0), (3, 9), (4, 1)]),
            AnomalyDecisionGuard::try_new(12, 2).expect("decision guard"),
        )
        .expect("anomaly landscape");
        let receiver_rank = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 5)
            .expect("receiver rank");
        let commitment = DecisionCommitmentState::new(
            landscape.hypotheses.task_id,
            landscape.hypotheses.target_id,
        );

        let summary =
            AnomalyDecisionProgressSummary::from_state(&landscape, &receiver_rank, &commitment)
                .expect("progress summary");

        assert_eq!(summary.receiver_rank, 0);
        assert_eq!(summary.reconstructed_at_tick, None);
        assert_eq!(summary.committed_at_tick, None);
        assert_eq!(
            summary.landscape_summary.top_hypothesis,
            AnomalyClusterId(3)
        );
        assert_eq!(summary.landscape_summary.top_hypothesis_margin, 5);
    }

    #[test]
    fn quality_summary_distinguishes_reconstruction_commitment_and_duplicates() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]),
            AnomalyDecisionGuard::try_new(8, 2).expect("decision guard"),
        )
        .expect("anomaly landscape");
        let mut receiver_rank =
            ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 3)
                .expect("receiver rank");
        receiver_rank
            .record_contribution_arrival(ContributionLedgerId(3), Tick(10))
            .expect("seed duplicate contribution");
        let source_update = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &source_record(1, 2, 3),
            ContributionLedgerId(3),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 9), (4, 0)]),
            None,
        )
        .expect("source update");
        let local = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(2),
            origin_mode: EvidenceOriginMode::LocallyGenerated,
            fragment_id: None,
            rank_id: None,
            local_observation_id: Some(LocalObservationId(44)),
            contribution_ledger_ids: vec![ContributionLedgerId(10_044)],
            ..source_input()
        })
        .expect("local evidence");
        let local_update = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &local,
            ContributionLedgerId(10_044),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 5), (4, 0)]),
            None,
        )
        .expect("local update");
        let recoded = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(7),
            origin_mode: EvidenceOriginMode::RecodedAggregated,
            fragment_id: None,
            rank_id: None,
            local_observation_id: Some(LocalObservationId(45)),
            parent_evidence_ids: vec![CodedEvidenceId(1), CodedEvidenceId(2)],
            contribution_ledger_ids: vec![ContributionLedgerId(100)],
            ..source_input()
        })
        .expect("recoded evidence");
        let recoded_update = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &recoded,
            ContributionLedgerId(100),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 6), (4, 0)]),
            None,
        )
        .expect("recoded update");

        let outcome = reduce_landscape_updates(
            &receiver_rank,
            &landscape,
            &[local_update, recoded_update, source_update],
            Tick(20),
        )
        .expect("landscape update");
        let mut commitment = DecisionCommitmentState::new(
            landscape.hypotheses.task_id,
            landscape.hypotheses.target_id,
        );
        commitment
            .check_commitment(&outcome.landscape, &outcome.receiver_rank, Tick(21))
            .expect("commitment check");
        let summary = ReceiverInferenceQualitySummary::from_events(
            &outcome.landscape,
            &outcome.receiver_rank,
            &commitment,
            &outcome.events,
        )
        .expect("quality summary");

        assert_eq!(summary.receiver_rank, 3);
        assert_eq!(summary.exact_reconstruction_tick, Some(Tick(20)));
        assert_eq!(summary.decision_commitment_tick, Some(Tick(21)));
        assert_eq!(summary.innovative_update_count, 2);
        assert_eq!(summary.duplicate_update_count, 1);
        assert_eq!(summary.top_hypothesis, AnomalyClusterId(3));
        assert_eq!(summary.top_hypothesis_margin, 11);
    }

    #[test]
    fn quality_summary_counts_source_local_and_recoded_origins() {
        let before = AnomalyLandscapeSummary {
            top_hypothesis: AnomalyClusterId(3),
            runner_up_hypothesis: AnomalyClusterId(1),
            top_hypothesis_margin: 4,
            uncertainty_permille: 920,
            energy_gap: 4,
        };
        let middle = AnomalyLandscapeSummary {
            top_hypothesis_margin: 8,
            uncertainty_permille: 840,
            energy_gap: 8,
            ..before
        };
        let after = AnomalyLandscapeSummary {
            top_hypothesis_margin: 11,
            uncertainty_permille: 780,
            energy_gap: 11,
            ..before
        };
        let events = [
            LandscapeUpdateEvent {
                task_id: InferenceTaskId(70),
                evidence_id: CodedEvidenceId(1),
                contribution_id: ContributionLedgerId(3),
                origin_mode: EvidenceOriginMode::SourceCoded,
                arrival_class: FragmentArrivalClass::Duplicate,
                rank_before: 1,
                rank_after: 1,
                summary_before: before,
                summary_after: before,
            },
            LandscapeUpdateEvent {
                task_id: InferenceTaskId(70),
                evidence_id: CodedEvidenceId(2),
                contribution_id: ContributionLedgerId(10_044),
                origin_mode: EvidenceOriginMode::LocallyGenerated,
                arrival_class: FragmentArrivalClass::Innovative,
                rank_before: 1,
                rank_after: 2,
                summary_before: before,
                summary_after: middle,
            },
            LandscapeUpdateEvent {
                task_id: InferenceTaskId(70),
                evidence_id: CodedEvidenceId(7),
                contribution_id: ContributionLedgerId(100),
                origin_mode: EvidenceOriginMode::RecodedAggregated,
                arrival_class: FragmentArrivalClass::Innovative,
                rank_before: 2,
                rank_after: 3,
                summary_before: middle,
                summary_after: after,
            },
        ];
        let (innovative, duplicate, origin_counts) = summarize_update_events(&events);

        assert_eq!(innovative, 2);
        assert_eq!(duplicate, 1);
        assert_eq!(origin_counts.source_coded, 1);
        assert_eq!(origin_counts.locally_generated, 1);
        assert_eq!(origin_counts.recoded_aggregated, 1);
    }

    #[test]
    fn monotone_evidence_inclusion_preserves_quality_order() {
        let landscape = AnomalyLandscape::try_new(
            anomaly_hypotheses(),
            anomaly_scores(&[(0, 0), (1, 2), (2, 0), (3, 4), (4, 0)]),
            anomaly_guard(),
        )
        .expect("anomaly landscape");
        let receiver_rank = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 4)
            .expect("receiver rank");
        let first = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &source_record(1, 2, 3),
            ContributionLedgerId(3),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 4), (4, 0)]),
            None,
        )
        .expect("first update");
        let second = EvidenceVectorRecord::try_from_evidence(
            &landscape,
            &source_record(2, 3, 4),
            ContributionLedgerId(4),
            anomaly_scores(&[(0, 0), (1, 0), (2, 0), (3, 5), (4, 0)]),
            None,
        )
        .expect("second update");

        let first_outcome = reduce_landscape_updates(
            &receiver_rank,
            &landscape,
            std::slice::from_ref(&first),
            Tick(20),
        )
        .expect("first outcome");
        let second_outcome = reduce_landscape_updates(
            &first_outcome.receiver_rank,
            &first_outcome.landscape,
            &[second],
            Tick(21),
        )
        .expect("second outcome");

        assert_eq!(
            first_outcome.landscape.summary.top_hypothesis,
            AnomalyClusterId(3)
        );
        assert_eq!(
            second_outcome.landscape.summary.top_hypothesis,
            AnomalyClusterId(3)
        );
        assert!(
            second_outcome.landscape.summary.top_hypothesis_margin
                >= first_outcome.landscape.summary.top_hypothesis_margin
        );
        assert!(
            second_outcome.landscape.summary.uncertainty_permille
                <= first_outcome.landscape.summary.uncertainty_permille
        );
    }

    #[test]
    fn coded_evidence_origin_modes_are_distinct_and_validated() {
        let source = CodedEvidenceRecord::try_new(source_input()).expect("source-coded record");
        assert_eq!(source.origin_mode, EvidenceOriginMode::SourceCoded);
        assert_eq!(source.validity, CodedEvidenceValidity::Valid);

        let local = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(2),
            origin_mode: EvidenceOriginMode::LocallyGenerated,
            fragment_id: None,
            rank_id: None,
            local_observation_id: Some(LocalObservationId(44)),
            contribution_ledger_ids: vec![ContributionLedgerId(10_044)],
            ..source_input()
        })
        .expect("local record");
        assert_eq!(local.origin_mode, EvidenceOriginMode::LocallyGenerated);
        assert_eq!(local.local_observation_id, Some(LocalObservationId(44)));

        let recoded = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(3),
            origin_mode: EvidenceOriginMode::RecodedAggregated,
            fragment_id: None,
            rank_id: None,
            local_observation_id: Some(LocalObservationId(45)),
            parent_evidence_ids: vec![CodedEvidenceId(2), CodedEvidenceId(1)],
            contribution_ledger_ids: vec![ContributionLedgerId(10_045), ContributionLedgerId(3)],
            ..source_input()
        })
        .expect("recoded record");
        assert_eq!(recoded.origin_mode, EvidenceOriginMode::RecodedAggregated);
        assert_eq!(
            recoded.parent_evidence_ids,
            vec![CodedEvidenceId(1), CodedEvidenceId(2)]
        );
    }

    #[test]
    fn coded_evidence_recoded_lineage_is_canonical_and_auditable() {
        let recoded = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(7),
            origin_mode: EvidenceOriginMode::RecodedAggregated,
            fragment_id: None,
            rank_id: None,
            parent_evidence_ids: vec![CodedEvidenceId(5), CodedEvidenceId(1)],
            contribution_ledger_ids: vec![ContributionLedgerId(9), ContributionLedgerId(3)],
            ..source_input()
        })
        .expect("canonical recoded record");

        assert_eq!(
            recoded.parent_evidence_ids,
            vec![CodedEvidenceId(1), CodedEvidenceId(5)]
        );
        assert_eq!(
            recoded.contribution_ledger_ids,
            vec![ContributionLedgerId(3), ContributionLedgerId(9)]
        );
    }

    #[test]
    fn coded_evidence_rejects_malformed_recoding_lineage() {
        assert_eq!(
            CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
                evidence_id: CodedEvidenceId(7),
                origin_mode: EvidenceOriginMode::RecodedAggregated,
                fragment_id: None,
                rank_id: None,
                parent_evidence_ids: Vec::new(),
                contribution_ledger_ids: vec![ContributionLedgerId(9)],
                ..source_input()
            }),
            Err(CodedEvidenceRecordError::RecodedWithoutParents)
        );
        assert_eq!(
            CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
                evidence_id: CodedEvidenceId(7),
                origin_mode: EvidenceOriginMode::RecodedAggregated,
                fragment_id: None,
                rank_id: None,
                parent_evidence_ids: vec![CodedEvidenceId(7)],
                contribution_ledger_ids: vec![ContributionLedgerId(9)],
                ..source_input()
            }),
            Err(CodedEvidenceRecordError::SelfParent)
        );
        assert_eq!(
            CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
                evidence_id: CodedEvidenceId(7),
                origin_mode: EvidenceOriginMode::RecodedAggregated,
                fragment_id: None,
                rank_id: None,
                parent_evidence_ids: vec![CodedEvidenceId(1), CodedEvidenceId(1)],
                contribution_ledger_ids: vec![ContributionLedgerId(9)],
                ..source_input()
            }),
            Err(CodedEvidenceRecordError::DuplicateParentEvidence)
        );
    }

    #[test]
    fn contribution_ledger_records_validate_parent_and_aggregate_rules() {
        let parent_union = ContributionLedgerRecord::try_new(ContributionLedgerRecordInput {
            evidence_id: CodedEvidenceId(7),
            contribution_id: ContributionLedgerId(3),
            contribution_kind: ContributionLedgerKind::ParentLedgerUnion,
            parent_contribution_ids: vec![ContributionLedgerId(9), ContributionLedgerId(3)],
            local_observation_id: None,
        })
        .expect("parent-union ledger record");
        assert_eq!(
            parent_union.parent_contribution_ids,
            vec![ContributionLedgerId(3), ContributionLedgerId(9)]
        );

        let aggregate = ContributionLedgerRecord::try_new(ContributionLedgerRecordInput {
            evidence_id: CodedEvidenceId(7),
            contribution_id: ContributionLedgerId(100),
            contribution_kind: ContributionLedgerKind::AggregateWithLocalObservation,
            parent_contribution_ids: vec![ContributionLedgerId(3), ContributionLedgerId(9)],
            local_observation_id: Some(LocalObservationId(45)),
        })
        .expect("aggregate ledger record");
        assert_eq!(aggregate.local_observation_id, Some(LocalObservationId(45)));

        assert_eq!(
            ContributionLedgerRecord::try_new(ContributionLedgerRecordInput {
                evidence_id: CodedEvidenceId(7),
                contribution_id: ContributionLedgerId(100),
                contribution_kind: ContributionLedgerKind::ParentLedgerUnion,
                parent_contribution_ids: vec![ContributionLedgerId(3), ContributionLedgerId(9)],
                local_observation_id: None,
            }),
            Err(ContributionLedgerRecordError::ParentLedgerUnionIntroducesContribution)
        );
        assert_eq!(
            ContributionLedgerRecord::try_new(ContributionLedgerRecordInput {
                evidence_id: CodedEvidenceId(7),
                contribution_id: ContributionLedgerId(100),
                contribution_kind: ContributionLedgerKind::AggregateWithLocalObservation,
                parent_contribution_ids: vec![ContributionLedgerId(3), ContributionLedgerId(9)],
                local_observation_id: None,
            }),
            Err(ContributionLedgerRecordError::AggregateWithoutLocalObservation)
        );
    }

    #[test]
    fn recoding_validity_accepts_parent_union_and_local_aggregate() {
        let recoded = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(7),
            origin_mode: EvidenceOriginMode::RecodedAggregated,
            fragment_id: None,
            rank_id: None,
            local_observation_id: Some(LocalObservationId(45)),
            parent_evidence_ids: vec![CodedEvidenceId(1), CodedEvidenceId(2)],
            contribution_ledger_ids: vec![ContributionLedgerId(100), ContributionLedgerId(3)],
            ..source_input()
        })
        .expect("recoded record");
        let parent_union = ContributionLedgerRecord::try_new(ContributionLedgerRecordInput {
            evidence_id: CodedEvidenceId(7),
            contribution_id: ContributionLedgerId(3),
            contribution_kind: ContributionLedgerKind::ParentLedgerUnion,
            parent_contribution_ids: vec![ContributionLedgerId(3), ContributionLedgerId(9)],
            local_observation_id: None,
        })
        .expect("parent-union ledger record");
        let aggregate = ContributionLedgerRecord::try_new(ContributionLedgerRecordInput {
            evidence_id: CodedEvidenceId(7),
            contribution_id: ContributionLedgerId(100),
            contribution_kind: ContributionLedgerKind::AggregateWithLocalObservation,
            parent_contribution_ids: vec![ContributionLedgerId(3), ContributionLedgerId(9)],
            local_observation_id: Some(LocalObservationId(45)),
        })
        .expect("aggregate ledger record");

        assert_eq!(
            recoded.validate_recoding_ledger(&[aggregate, parent_union]),
            Ok(vec![ContributionLedgerId(3), ContributionLedgerId(100)])
        );
    }

    #[test]
    fn recoding_validity_rejects_missing_or_ambiguous_lineage() {
        let recoded = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(7),
            origin_mode: EvidenceOriginMode::RecodedAggregated,
            fragment_id: None,
            rank_id: None,
            parent_evidence_ids: vec![CodedEvidenceId(1), CodedEvidenceId(2)],
            contribution_ledger_ids: vec![ContributionLedgerId(3), ContributionLedgerId(9)],
            ..source_input()
        })
        .expect("recoded record");
        let parent_union = ContributionLedgerRecord::try_new(ContributionLedgerRecordInput {
            evidence_id: CodedEvidenceId(7),
            contribution_id: ContributionLedgerId(3),
            contribution_kind: ContributionLedgerKind::ParentLedgerUnion,
            parent_contribution_ids: vec![ContributionLedgerId(3), ContributionLedgerId(9)],
            local_observation_id: None,
        })
        .expect("parent-union ledger record");
        let wrong_evidence = ContributionLedgerRecord::try_new(ContributionLedgerRecordInput {
            evidence_id: CodedEvidenceId(8),
            contribution_id: ContributionLedgerId(9),
            contribution_kind: ContributionLedgerKind::ParentLedgerUnion,
            parent_contribution_ids: vec![ContributionLedgerId(3), ContributionLedgerId(9)],
            local_observation_id: None,
        })
        .expect("wrong-evidence ledger record");

        assert_eq!(
            recoded.validate_recoding_ledger(std::slice::from_ref(&parent_union)),
            Err(CodedEvidenceRecordError::MissingContributionLedgerRecord)
        );
        assert_eq!(
            recoded.validate_recoding_ledger(&[parent_union, wrong_evidence]),
            Err(CodedEvidenceRecordError::UnexpectedContributionLedgerRecord)
        );
    }

    #[test]
    fn recoded_duplicate_contributions_do_not_inflate_receiver_rank() {
        let mut state = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 3)
            .expect("receiver rank state");
        state
            .record_contribution_arrival(ContributionLedgerId(3), Tick(10))
            .expect("initial contribution");

        let recoded = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(7),
            origin_mode: EvidenceOriginMode::RecodedAggregated,
            fragment_id: None,
            rank_id: None,
            parent_evidence_ids: vec![CodedEvidenceId(1)],
            contribution_ledger_ids: vec![ContributionLedgerId(3)],
            ..source_input()
        })
        .expect("recoded record");
        let parent_union = ContributionLedgerRecord::try_new(ContributionLedgerRecordInput {
            evidence_id: CodedEvidenceId(7),
            contribution_id: ContributionLedgerId(3),
            contribution_kind: ContributionLedgerKind::ParentLedgerUnion,
            parent_contribution_ids: vec![ContributionLedgerId(3)],
            local_observation_id: None,
        })
        .expect("parent-union ledger record");
        let accepted = recoded
            .validate_recoding_ledger(&[parent_union])
            .expect("valid recoded ledger");

        for contribution_id in accepted {
            assert_eq!(
                state.record_contribution_arrival(contribution_id, Tick(11)),
                Ok(FragmentArrivalClass::Duplicate)
            );
        }
        assert_eq!(state.independent_rank, 1);
        assert_eq!(state.duplicate_arrivals, 1);
    }

    #[test]
    fn recoded_parent_only_lineage_is_non_innovative_when_already_counted() {
        let mut state = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 4)
            .expect("receiver rank state");
        for contribution_id in [ContributionLedgerId(3), ContributionLedgerId(9)] {
            state
                .record_contribution_arrival(contribution_id, Tick(10))
                .expect("initial contribution");
        }

        let recoded = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(7),
            origin_mode: EvidenceOriginMode::RecodedAggregated,
            fragment_id: None,
            rank_id: None,
            parent_evidence_ids: vec![CodedEvidenceId(1), CodedEvidenceId(2)],
            contribution_ledger_ids: vec![ContributionLedgerId(3), ContributionLedgerId(9)],
            ..source_input()
        })
        .expect("recoded record");
        let ledger_records = [
            ContributionLedgerRecord::try_new(ContributionLedgerRecordInput {
                evidence_id: CodedEvidenceId(7),
                contribution_id: ContributionLedgerId(3),
                contribution_kind: ContributionLedgerKind::ParentLedgerUnion,
                parent_contribution_ids: vec![ContributionLedgerId(3), ContributionLedgerId(9)],
                local_observation_id: None,
            })
            .expect("first parent-union ledger record"),
            ContributionLedgerRecord::try_new(ContributionLedgerRecordInput {
                evidence_id: CodedEvidenceId(7),
                contribution_id: ContributionLedgerId(9),
                contribution_kind: ContributionLedgerKind::ParentLedgerUnion,
                parent_contribution_ids: vec![ContributionLedgerId(3), ContributionLedgerId(9)],
                local_observation_id: None,
            })
            .expect("second parent-union ledger record"),
        ];

        for contribution_id in recoded
            .validate_recoding_ledger(&ledger_records)
            .expect("valid parent-only recoded ledger")
        {
            assert_eq!(
                state.record_contribution_arrival(contribution_id, Tick(11)),
                Ok(FragmentArrivalClass::Duplicate)
            );
        }
        assert_eq!(state.independent_rank, 2);
        assert_eq!(state.duplicate_arrivals, 2);
    }

    #[test]
    fn source_coded_exact_reconstruction_fixture_reaches_k() {
        let mut state = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 2)
            .expect("receiver rank state");
        let first = source_record(1, 2, 3);
        let second = source_record(2, 3, 4);

        record_evidence_contributions(&mut state, &first, Tick(10));
        assert_eq!(state.reconstructed_at_tick, None);
        record_evidence_contributions(&mut state, &second, Tick(11));

        assert_eq!(state.independent_rank, 2);
        assert_eq!(state.reconstructed_at_tick, Some(Tick(11)));
        assert!(state.is_reconstructed());
    }

    #[test]
    fn duplicate_arrival_fixture_repeats_without_rank_growth() {
        let mut state = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 2)
            .expect("receiver rank state");
        let first = source_record(1, 2, 3);

        record_evidence_contributions(&mut state, &first, Tick(10));
        record_evidence_contributions(&mut state, &first, Tick(11));
        record_evidence_contributions(&mut state, &first, Tick(12));

        assert_eq!(state.independent_rank, 1);
        assert_eq!(state.innovative_arrivals, 1);
        assert_eq!(state.duplicate_arrivals, 2);
        assert_eq!(state.reconstructed_at_tick, None);
    }

    #[test]
    fn increasing_k_makes_recovery_harder_at_fixed_evidence_budget() {
        let mut lower_k = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 2)
            .expect("lower-k receiver rank state");
        let mut higher_k = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 3)
            .expect("higher-k receiver rank state");
        let evidence = [source_record(1, 2, 3), source_record(2, 3, 4)];

        for record in &evidence {
            record_evidence_contributions(&mut lower_k, record, Tick(10));
            record_evidence_contributions(&mut higher_k, record, Tick(10));
        }

        assert!(lower_k.is_reconstructed());
        assert!(!higher_k.is_reconstructed());
        assert_eq!(lower_k.independent_rank, higher_k.independent_rank);
    }

    #[test]
    fn useful_fragment_diversity_improves_recovery_at_fixed_k() {
        let mut low_diversity =
            ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 2)
                .expect("low-diversity receiver rank state");
        let mut high_diversity =
            ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 2)
                .expect("high-diversity receiver rank state");
        let first = source_record(1, 2, 3);
        let second = source_record(2, 3, 4);

        record_evidence_contributions(&mut low_diversity, &first, Tick(10));
        record_evidence_contributions(&mut low_diversity, &first, Tick(11));
        record_evidence_contributions(&mut high_diversity, &first, Tick(10));
        record_evidence_contributions(&mut high_diversity, &second, Tick(11));

        assert!(!low_diversity.is_reconstructed());
        assert!(high_diversity.is_reconstructed());
        assert_eq!(low_diversity.independent_rank, 1);
        assert_eq!(high_diversity.independent_rank, 2);
    }

    #[test]
    fn locally_generated_evidence_counts_without_central_encoder() {
        let local = CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
            evidence_id: CodedEvidenceId(2),
            origin_mode: EvidenceOriginMode::LocallyGenerated,
            fragment_id: None,
            rank_id: None,
            local_observation_id: Some(LocalObservationId(44)),
            contribution_ledger_ids: vec![ContributionLedgerId(10_044)],
            ..source_input()
        })
        .expect("local evidence record");
        let mut state = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 1)
            .expect("receiver rank state");

        record_evidence_contributions(&mut state, &local, Tick(10));

        assert_eq!(local.origin_mode, EvidenceOriginMode::LocallyGenerated);
        assert_eq!(state.independent_rank, 1);
        assert_eq!(state.reconstructed_at_tick, Some(Tick(10)));
    }

    #[test]
    fn coded_evidence_rejects_invalid_original_record_shapes() {
        assert_eq!(
            CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
                fragment_id: None,
                ..source_input()
            }),
            Err(CodedEvidenceRecordError::MissingSourceFragmentOrRank)
        );
        assert_eq!(
            CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
                origin_mode: EvidenceOriginMode::LocallyGenerated,
                fragment_id: None,
                rank_id: None,
                local_observation_id: None,
                ..source_input()
            }),
            Err(CodedEvidenceRecordError::MissingLocalObservation)
        );
        assert_eq!(
            CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
                parent_evidence_ids: vec![CodedEvidenceId(2)],
                ..source_input()
            }),
            Err(CodedEvidenceRecordError::UnexpectedParentEvidence)
        );
        assert_eq!(
            CodedEvidenceRecord::try_new(CodedEvidenceRecordInput {
                payload_bytes: 0,
                ..source_input()
            }),
            Err(CodedEvidenceRecordError::ZeroPayloadBytes)
        );
    }

    #[test]
    fn coding_window_requires_reconstructable_width() {
        assert_eq!(CodingWindow::try_new(0, 4), None);
        assert_eq!(CodingWindow::try_new(5, 4), None);
        assert_eq!(
            CodingWindow::try_new(3, 5),
            Some(CodingWindow {
                required_rank: 3,
                encoded_fragments: 5,
            })
        );
    }

    #[test]
    fn byte_budget_represents_equal_payload_bytes_without_floats() {
        let window = CodingWindow::try_new(8, 12).expect("valid coding window");
        let budget = PayloadBudgetMetadata::equal_payload_bytes(window, 32, 384, 1)
            .expect("equal byte budget");

        assert_eq!(budget.budget_kind, PayloadBudgetKind::EqualPayloadBytes);
        assert_eq!(budget.fragment_payload_bytes, 32);
        assert_eq!(budget.uncoded_message_payload_bytes, 384);
        assert_eq!(budget.fixed_payload_budget_bytes, 384);
        assert_eq!(budget.coded_payload_budget_bytes(), 384);
        assert_eq!(budget.uncoded_payload_budget_bytes(), 384);
        assert!(budget.has_equal_payload_byte_budget());
    }

    #[test]
    fn byte_budget_rejects_invalid_or_unequal_payload_metadata() {
        let window = CodingWindow::try_new(8, 12).expect("valid coding window");

        assert_eq!(
            PayloadBudgetMetadata::equal_payload_bytes(window, 0, 384, 1),
            Err(PayloadBudgetError::ZeroFragmentPayloadBytes)
        );
        assert_eq!(
            PayloadBudgetMetadata::equal_payload_bytes(window, 32, 0, 1),
            Err(PayloadBudgetError::ZeroUncodedMessagePayloadBytes)
        );
        assert_eq!(
            PayloadBudgetMetadata::equal_payload_bytes(window, 32, 384, 0),
            Err(PayloadBudgetError::ZeroUncodedReplicaCount)
        );
        assert_eq!(
            PayloadBudgetMetadata::equal_payload_bytes(window, 32, 256, 1),
            Err(PayloadBudgetError::UnequalPayloadByteBudget)
        );

        let overflow_window = CodingWindow::try_new(1, u16::MAX).expect("valid coding window");
        assert_eq!(
            PayloadBudgetMetadata::equal_payload_bytes(overflow_window, u32::MAX, 1, 1),
            Err(PayloadBudgetError::PayloadBudgetOverflow)
        );
    }

    #[test]
    fn receiver_rank_counts_canonical_contributions_not_copies() {
        let mut state = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 3)
            .expect("receiver rank state");

        assert_eq!(
            state.record_contribution_arrival(ContributionLedgerId(20), Tick(10)),
            Ok(FragmentArrivalClass::Innovative)
        );
        assert_eq!(
            state.record_contribution_arrival(ContributionLedgerId(10), Tick(11)),
            Ok(FragmentArrivalClass::Innovative)
        );
        assert_eq!(
            state.record_contribution_arrival(ContributionLedgerId(20), Tick(12)),
            Ok(FragmentArrivalClass::Duplicate)
        );

        assert_eq!(state.independent_rank, 2);
        assert_eq!(state.innovative_arrivals, 2);
        assert_eq!(state.duplicate_arrivals, 1);
        assert_eq!(
            state.accepted_contribution_ids,
            vec![ContributionLedgerId(10), ContributionLedgerId(20)]
        );
        assert_eq!(state.reconstructed_at_tick, None);
    }

    #[test]
    fn reconstruction_records_first_threshold_crossing_once() {
        let mut state = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 2)
            .expect("receiver rank state");

        assert_eq!(
            state.record_contribution_arrival(ContributionLedgerId(10), Tick(10)),
            Ok(FragmentArrivalClass::Innovative)
        );
        assert_eq!(state.reconstructed_at_tick, None);

        assert_eq!(
            state.record_contribution_arrival(ContributionLedgerId(20), Tick(11)),
            Ok(FragmentArrivalClass::Innovative)
        );
        assert_eq!(state.reconstructed_at_tick, Some(Tick(11)));
        assert!(state.is_reconstructed());

        assert_eq!(
            state.record_contribution_arrival(ContributionLedgerId(20), Tick(12)),
            Ok(FragmentArrivalClass::Duplicate)
        );
        assert_eq!(
            state.record_reconstruction_if_complete(Tick(13)),
            Some(Tick(11))
        );
        assert_eq!(state.reconstructed_at_tick, Some(Tick(11)));
        assert_eq!(state.independent_rank, 2);
        assert_eq!(state.duplicate_arrivals, 1);
    }

    #[test]
    fn receiver_rank_is_deterministic_across_insertion_order() {
        let mut left = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 3)
            .expect("left receiver rank state");
        let mut right = ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 3)
            .expect("right receiver rank state");

        for contribution_id in [
            ContributionLedgerId(30),
            ContributionLedgerId(10),
            ContributionLedgerId(20),
        ] {
            left.record_contribution_arrival(contribution_id, Tick(20))
                .expect("left contribution");
        }
        for contribution_id in [
            ContributionLedgerId(20),
            ContributionLedgerId(30),
            ContributionLedgerId(10),
        ] {
            right
                .record_contribution_arrival(contribution_id, Tick(20))
                .expect("right contribution");
        }

        assert_eq!(left.independent_rank, right.independent_rank);
        assert_eq!(
            left.accepted_contribution_ids,
            right.accepted_contribution_ids
        );
        assert_eq!(left.reconstructed_at_tick, right.reconstructed_at_tick);
    }

    #[test]
    fn receiver_rank_rejects_zero_reconstruction_threshold() {
        assert_eq!(
            ReceiverRankState::try_new(DiffusionMessageId(id16(1)), node_id(7), 0),
            Err(ReceiverRankError::ZeroRequiredRank)
        );
    }

    #[test]
    fn quorum_requires_all_observed_receivers_to_complete() {
        let incomplete = ReconstructionQuorum {
            message_id: DiffusionMessageId(id16(1)),
            required_rank: 3,
            observed_receivers: 2,
            complete_receivers: 1,
            min_independent_rank: 2,
        };
        let complete = ReconstructionQuorum {
            complete_receivers: 2,
            min_independent_rank: 3,
            ..incomplete
        };

        assert!(!incomplete.is_complete());
        assert!(complete.is_complete());
    }

    #[test]
    fn diffusion_pressure_clamps_to_permille_range() {
        assert_eq!(
            DiffusionPressure {
                custody_pressure_permille: 1001,
                innovation_pressure_permille: 1200,
                duplicate_pressure_permille: 999,
            }
            .clamped(),
            DiffusionPressure {
                custody_pressure_permille: 1000,
                innovation_pressure_permille: 1000,
                duplicate_pressure_permille: 999,
            }
        );
    }

    #[test]
    fn order_parameters_clamp_normalized_pressures() {
        let parameters = DiffusionOrderParameters {
            pressure: DiffusionPressure {
                custody_pressure_permille: 1001,
                innovation_pressure_permille: 700,
                duplicate_pressure_permille: 1400,
            },
            storage_pressure_permille: 1200,
            rank_deficit: 4,
            duplicate_arrival_permille: 1300,
        }
        .clamped();

        assert_eq!(parameters.pressure.custody_pressure_permille, 1000);
        assert_eq!(parameters.pressure.innovation_pressure_permille, 700);
        assert_eq!(parameters.pressure.duplicate_pressure_permille, 1000);
        assert_eq!(parameters.storage_pressure_permille, 1000);
        assert_eq!(parameters.rank_deficit, 4);
        assert_eq!(parameters.duplicate_arrival_permille, 1000);
    }

    #[test]
    fn retention_policy_clamps_custody_threshold() {
        assert_eq!(
            FragmentRetentionPolicy::new(8, 1200, true),
            FragmentRetentionPolicy {
                fragment_budget: 8,
                custody_threshold_permille: 1000,
                evict_duplicates_first: true,
            }
        );
    }
}
