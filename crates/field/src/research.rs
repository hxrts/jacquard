//! Coded-diffusion research-path boundary.
//!
//! This module is the feature-neutral namespace for the experimental coded
//! reconstruction path. It owns only fragment movement, rank, custody,
//! duplicate/innovative arrivals, diffusion pressure, and reconstruction
//! quorum vocabulary. It must remain independent of the legacy planner stack.

use jacquard_core::{NodeId, Tick};
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
    values: &mut Vec<ContributionLedgerId>,
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
    values: &mut Vec<T>,
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

/// One deterministic integer score for one anomaly hypothesis.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnomalyHypothesisScore {
    /// Candidate cluster being scored.
    pub hypothesis_id: AnomalyClusterId,
    /// Deterministic scaled score or energy for this candidate.
    pub scaled_score: i32,
}

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
    hypotheses: &mut Vec<AnomalyClusterId>,
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
    scores: &mut Vec<AnomalyHypothesisScore>,
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
            self.independent_rank = self.accepted_contribution_ids.len() as u16;
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
            recoded.validate_recoding_ledger(&[parent_union.clone()]),
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
