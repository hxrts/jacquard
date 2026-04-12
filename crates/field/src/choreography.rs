//! Protocol session lifecycle and guest runtime for cooperative field
//! protocols.
//!
//! Manages four protocol kinds: `SummaryDissemination` (outbound evidence
//! propagation), `AntiEntropy` (convergence over stale destinations),
//! `RetentionReplay` (recovery of held payloads), and `ExplicitCoordination`
//! (direct peer negotiation). Sessions are keyed by destination and protocol
//! kind; each has an execution policy, task binding, and a bounded step budget
//! per tick to enforce deterministic work.
//!
//! `FieldBridgeState` queues outbound summaries, inbound summaries, and branch
//! choices that flow between the protocol layer and the rest of the engine.
//! `FieldGuestRuntime` steps a session's state machine and returns a
//! `FieldChoreographyAdvance` describing work done and next-tick hints.
//! Checkpointing and recovery support session migration across engine restarts.
// long-file-exception: the field choreography runtime keeps protocol state,
// bridge state, artifact retention, and guest-runtime tests together so the
// host-bridged session semantics can be reviewed as one cohesive unit.

#![expect(
    dead_code,
    reason = "phase-4 choreography contracts are consumed by later protocol/runtime phases"
)]

use std::collections::{BTreeMap, VecDeque};

use jacquard_core::{DestinationId, NodeId, RouteEpoch, RouteId, Tick};

use crate::summary::{FieldSummary, SummaryDestinationKey, FIELD_SUMMARY_ENCODING_BYTES};

pub(crate) const FIELD_PROTOCOL_QUEUE_MAX: usize = 8;
pub(crate) const FIELD_PROTOCOL_ARTIFACT_LIMIT: usize = 64;
pub(crate) const FIELD_PROTOCOL_ARTIFACT_RETENTION_MAX: usize = 8;
pub(crate) const FIELD_PROTOCOL_RECONFIGURATION_RETENTION_MAX: usize = 8;
pub(crate) const FIELD_PROTOCOL_SESSION_MAX: usize = 8;
pub(crate) const FIELD_PROTOCOL_STEP_BUDGET: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldProtocolKind {
    SummaryDissemination,
    AntiEntropy,
    RetentionReplay,
    ExplicitCoordination,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldExecutionPolicyClass {
    Cheap,
    Buffered,
    Coordinated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FieldProtocolExecutionPolicy {
    class: FieldExecutionPolicyClass,
    step_budget: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldProtocolSessionKey {
    pub(crate) protocol: FieldProtocolKind,
    pub(crate) route_id: Option<RouteId>,
    pub(crate) topology_epoch: RouteEpoch,
    pub(crate) destination: Option<SummaryDestinationKey>,
}

impl FieldProtocolSessionKey {
    #[must_use]
    pub fn protocol(&self) -> FieldProtocolKind {
        self.protocol
    }

    #[must_use]
    pub fn route_id(&self) -> Option<RouteId> {
        self.route_id
    }

    #[must_use]
    pub fn topology_epoch(&self) -> RouteEpoch {
        self.topology_epoch
    }

    #[must_use]
    pub fn destination(&self) -> Option<DestinationId> {
        self.destination.as_ref().map(|destination| {
            DestinationId::from(&crate::state::DestinationKey::from(destination))
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldProtocolArtifact {
    pub protocol: FieldProtocolKind,
    session: FieldProtocolSessionKey,
    pub detail: FieldProtocolArtifactDetail,
    pub last_updated_at: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldProtocolReconfigurationCause {
    OwnerTransfer,
    CheckpointRestore,
    ContinuationShift,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldProtocolReconfiguration {
    pub prior_session: FieldProtocolSessionKey,
    pub next_session: FieldProtocolSessionKey,
    pub protocol: FieldProtocolKind,
    pub route_id: Option<RouteId>,
    pub destination: Option<DestinationId>,
    pub prior_owner_tag: u64,
    pub next_owner_tag: u64,
    pub prior_generation: u32,
    pub next_generation: u32,
    pub cause: FieldProtocolReconfigurationCause,
    pub recorded_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldProtocolArtifactDetail {
    detail: String,
}

impl FieldProtocolArtifactDetail {
    #[must_use]
    pub(crate) fn new(detail: impl Into<String>) -> Self {
        let mut detail = detail.into();
        detail.truncate(FIELD_PROTOCOL_ARTIFACT_LIMIT);
        Self { detail }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.detail
    }
}

impl FieldProtocolArtifact {
    #[must_use]
    pub fn session(&self) -> &FieldProtocolSessionKey {
        &self.session
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QueuedProtocolSend {
    pub(crate) protocol: FieldProtocolKind,
    pub(crate) to_neighbor: NodeId,
    pub(crate) payload: [u8; FIELD_SUMMARY_ENCODING_BYTES],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StagedProtocolReceive {
    pub(crate) protocol: FieldProtocolKind,
    pub(crate) from_neighbor: NodeId,
    pub(crate) payload: [u8; FIELD_SUMMARY_ENCODING_BYTES],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockedReceiveMarker {
    Neighbor(NodeId),
    AnyPeer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldRoundDisposition {
    Continue,
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldHostWaitStatus {
    Idle,
    Delivered,
    TimedOut,
    Cancelled,
    Deferred,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldChoreographyRoundResult {
    pub disposition: FieldRoundDisposition,
    pub host_wait_status: FieldHostWaitStatus,
    pub blocked_receive: Option<BlockedReceiveMarker>,
    pub emitted_send_count: usize,
    pub execution_policy: FieldExecutionPolicyClass,
    pub step_budget_remaining: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldChoreographyAdvance {
    pub(crate) round: FieldChoreographyRoundResult,
    pub(crate) flushed_sends: Vec<QueuedProtocolSend>,
    pub(crate) recorded_artifacts: Vec<FieldProtocolArtifact>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FieldBridgeState {
    outbound_summaries: VecDeque<QueuedProtocolSend>,
    inbound_summaries: VecDeque<StagedProtocolReceive>,
    branch_choices: VecDeque<u8>,
    blocked_receive: Option<BlockedReceiveMarker>,
}

impl FieldBridgeState {
    #[must_use]
    pub(crate) fn outbound_len(&self) -> usize {
        self.outbound_summaries.len()
    }

    pub(crate) fn queue_outbound_summary(&mut self, send: QueuedProtocolSend) {
        if self.outbound_summaries.len() >= FIELD_PROTOCOL_QUEUE_MAX {
            self.outbound_summaries.pop_front();
        }
        self.outbound_summaries.push_back(send);
    }

    pub(crate) fn stage_inbound_summary(&mut self, receive: StagedProtocolReceive) {
        if self.inbound_summaries.len() >= FIELD_PROTOCOL_QUEUE_MAX {
            self.inbound_summaries.pop_front();
        }
        self.inbound_summaries.push_back(receive);
    }

    pub(crate) fn queue_branch_choice(&mut self, branch: u8) {
        if self.branch_choices.len() >= FIELD_PROTOCOL_QUEUE_MAX {
            self.branch_choices.pop_front();
        }
        self.branch_choices.push_back(branch);
    }

    pub(crate) fn set_blocked_receive(&mut self, blocked_receive: BlockedReceiveMarker) {
        self.blocked_receive = Some(blocked_receive);
    }

    pub(crate) fn clear_blocked_receive(&mut self) {
        self.blocked_receive = None;
    }

    #[must_use]
    pub(crate) fn blocked_receive(&self) -> Option<BlockedReceiveMarker> {
        self.blocked_receive
    }

    #[must_use]
    pub(crate) fn drain_outbound(&mut self) -> Vec<QueuedProtocolSend> {
        self.outbound_summaries.drain(..).collect()
    }

    #[must_use]
    pub(crate) fn pop_inbound(&mut self) -> Option<StagedProtocolReceive> {
        self.inbound_summaries.pop_front()
    }

    #[must_use]
    pub(crate) fn pop_branch_choice(&mut self) -> Option<u8> {
        self.branch_choices.pop_front()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FieldProtocolEffectBridge {
    SendSummary(FieldSummary),
    FlushOutbound,
    WaitForSummary(BlockedReceiveMarker),
    RecordArtifact(FieldProtocolArtifact),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldSessionCapability {
    pub(crate) session: FieldProtocolSessionKey,
    pub(crate) owner_tag: u64,
    pub(crate) generation: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldProtocolCheckpoint {
    pub(crate) session: FieldProtocolSessionKey,
    pub(crate) owner_tag: u64,
    pub(crate) generation: u32,
    pub(crate) bound_task: Option<u64>,
    pub(crate) blocked_receive: Option<BlockedReceiveMarker>,
    pub(crate) runtime_state: FieldGuestRuntimeState,
    pub(crate) pending_outbound: Vec<QueuedProtocolSend>,
    pub(crate) pending_inbound: Vec<StagedProtocolReceive>,
    pub(crate) pending_branch_choices: Vec<u8>,
    pub(crate) artifacts: Vec<FieldProtocolArtifact>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldSessionError {
    TooManySessions,
    NotFound,
    OwnershipMismatch,
    StaleCapabilityGeneration,
    BindingChanged,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldGuestRuntimeState {
    Ready,
    Waiting(BlockedReceiveMarker),
    Complete,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FieldGuestRuntime {
    protocol: FieldProtocolKind,
    state: FieldGuestRuntimeState,
    step_count: u8,
}

impl FieldGuestRuntime {
    fn new(protocol: FieldProtocolKind) -> Self {
        Self {
            protocol,
            state: FieldGuestRuntimeState::Ready,
            step_count: 0,
        }
    }

    fn restore(protocol: FieldProtocolKind, state: FieldGuestRuntimeState) -> Self {
        Self {
            protocol,
            state,
            step_count: 0,
        }
    }

    // long-block-exception: protocol stepping is a single bounded state-machine
    // transition over all guest-runtime branches and reads best as one unit.
    fn step(
        &mut self,
        session: &FieldProtocolSessionKey,
        bridge: &mut FieldBridgeState,
        now_tick: Tick,
    ) -> (FieldRoundDisposition, Vec<FieldProtocolArtifact>) {
        let policy = execution_policy_for(self.protocol);
        if matches!(self.state, FieldGuestRuntimeState::Cancelled) {
            return (FieldRoundDisposition::Complete, Vec::new());
        }
        if matches!(self.state, FieldGuestRuntimeState::Complete) {
            return (FieldRoundDisposition::Complete, Vec::new());
        }

        self.step_count = self.step_count.saturating_add(1);
        if self.step_count > policy.step_budget.min(FIELD_PROTOCOL_STEP_BUDGET) {
            self.state = FieldGuestRuntimeState::Cancelled;
            bridge.clear_blocked_receive();
            return (
                FieldRoundDisposition::Complete,
                vec![FieldProtocolArtifact {
                    protocol: self.protocol,
                    session: session.clone(),
                    detail: FieldProtocolArtifactDetail::new("step-budget-exceeded"),
                    last_updated_at: now_tick,
                }],
            );
        }

        let mut artifacts = Vec::new();
        let disposition = match self.protocol {
            FieldProtocolKind::SummaryDissemination => {
                if bridge.outbound_len() > 0 {
                    self.state = FieldGuestRuntimeState::Complete;
                    artifacts.push(FieldProtocolArtifact {
                        protocol: self.protocol,
                        session: session.clone(),
                        detail: FieldProtocolArtifactDetail::new("summary-dissemination"),
                        last_updated_at: now_tick,
                    });
                    FieldRoundDisposition::Complete
                } else {
                    bridge.set_blocked_receive(BlockedReceiveMarker::AnyPeer);
                    self.state = FieldGuestRuntimeState::Waiting(BlockedReceiveMarker::AnyPeer);
                    FieldRoundDisposition::Continue
                }
            }
            FieldProtocolKind::AntiEntropy => {
                if bridge.pop_inbound().is_some() {
                    bridge.clear_blocked_receive();
                    self.state = FieldGuestRuntimeState::Complete;
                    artifacts.push(FieldProtocolArtifact {
                        protocol: self.protocol,
                        session: session.clone(),
                        detail: FieldProtocolArtifactDetail::new("anti-entropy-received"),
                        last_updated_at: now_tick,
                    });
                    FieldRoundDisposition::Complete
                } else {
                    bridge.set_blocked_receive(BlockedReceiveMarker::AnyPeer);
                    self.state = FieldGuestRuntimeState::Waiting(BlockedReceiveMarker::AnyPeer);
                    FieldRoundDisposition::Continue
                }
            }
            FieldProtocolKind::RetentionReplay => {
                let branch = bridge.pop_branch_choice();
                if matches!(branch, Some(0)) {
                    self.state = FieldGuestRuntimeState::Complete;
                    artifacts.push(FieldProtocolArtifact {
                        protocol: self.protocol,
                        session: session.clone(),
                        detail: FieldProtocolArtifactDetail::new("retention-replay-deferred"),
                        last_updated_at: now_tick,
                    });
                    FieldRoundDisposition::Complete
                } else if bridge.outbound_len() > 0 {
                    self.state = FieldGuestRuntimeState::Complete;
                    artifacts.push(FieldProtocolArtifact {
                        protocol: self.protocol,
                        session: session.clone(),
                        detail: FieldProtocolArtifactDetail::new("retention-replay-flushed"),
                        last_updated_at: now_tick,
                    });
                    FieldRoundDisposition::Complete
                } else {
                    bridge.set_blocked_receive(BlockedReceiveMarker::AnyPeer);
                    self.state = FieldGuestRuntimeState::Waiting(BlockedReceiveMarker::AnyPeer);
                    FieldRoundDisposition::Continue
                }
            }
            FieldProtocolKind::ExplicitCoordination => {
                if bridge.pop_branch_choice().is_some() || bridge.pop_inbound().is_some() {
                    bridge.clear_blocked_receive();
                    self.state = FieldGuestRuntimeState::Complete;
                    artifacts.push(FieldProtocolArtifact {
                        protocol: self.protocol,
                        session: session.clone(),
                        detail: FieldProtocolArtifactDetail::new("explicit-coordination"),
                        last_updated_at: now_tick,
                    });
                    FieldRoundDisposition::Complete
                } else {
                    bridge.set_blocked_receive(BlockedReceiveMarker::AnyPeer);
                    self.state = FieldGuestRuntimeState::Waiting(BlockedReceiveMarker::AnyPeer);
                    FieldRoundDisposition::Continue
                }
            }
        };

        (disposition, artifacts)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OwnedFieldProtocolSession {
    owner_tag: u64,
    generation: u32,
    bound_task: Option<u64>,
    bridge: FieldBridgeState,
    runtime: FieldGuestRuntime,
    artifacts: VecDeque<FieldProtocolArtifact>,
    cancelled: bool,
}

impl OwnedFieldProtocolSession {
    fn new(protocol: FieldProtocolKind, owner_tag: u64, bound_task: Option<u64>) -> Self {
        Self {
            owner_tag,
            generation: 0,
            bound_task,
            bridge: FieldBridgeState::default(),
            runtime: FieldGuestRuntime::new(protocol),
            artifacts: VecDeque::new(),
            cancelled: false,
        }
    }

    fn capability(&self, session: &FieldProtocolSessionKey) -> FieldSessionCapability {
        FieldSessionCapability {
            session: session.clone(),
            owner_tag: self.owner_tag,
            generation: self.generation,
        }
    }

    fn note_new_work(&mut self) {
        if !self.cancelled {
            self.runtime.state = FieldGuestRuntimeState::Ready;
            self.runtime.step_count = 0;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FieldProtocolRuntime {
    sessions: BTreeMap<FieldProtocolSessionKey, OwnedFieldProtocolSession>,
    reconfigurations: VecDeque<FieldProtocolReconfiguration>,
}

impl FieldProtocolRuntime {
    #[must_use]
    pub(crate) fn artifacts(&self) -> Vec<FieldProtocolArtifact> {
        let mut artifacts = self
            .sessions
            .values()
            .flat_map(|session| session.artifacts.iter().cloned())
            .collect::<Vec<_>>();
        artifacts.sort_by(|left, right| {
            left.last_updated_at
                .cmp(&right.last_updated_at)
                .then_with(|| left.session.cmp(&right.session))
                .then_with(|| left.protocol.cmp(&right.protocol))
        });
        artifacts
    }

    #[must_use]
    pub(crate) fn reconfigurations(&self) -> Vec<FieldProtocolReconfiguration> {
        self.reconfigurations.iter().cloned().collect()
    }

    pub(crate) fn open_session(
        &mut self,
        session: &FieldProtocolSessionKey,
        owner_tag: u64,
        bound_task: Option<u64>,
    ) -> Result<FieldSessionCapability, FieldSessionError> {
        if !self.sessions.contains_key(session) && self.sessions.len() >= FIELD_PROTOCOL_SESSION_MAX
        {
            return Err(FieldSessionError::TooManySessions);
        }
        let protocol = session.protocol;
        let owned = self
            .sessions
            .entry(session.clone())
            .or_insert_with(|| OwnedFieldProtocolSession::new(protocol, owner_tag, bound_task));
        owned.owner_tag = owner_tag;
        owned.bound_task = bound_task;
        Ok(owned.capability(session))
    }

    pub(crate) fn close_session(
        &mut self,
        capability: &FieldSessionCapability,
    ) -> Result<Vec<FieldProtocolArtifact>, FieldSessionError> {
        self.assert_owned(capability, None)?;
        let Some(session) = self.sessions.remove(&capability.session) else {
            return Err(FieldSessionError::NotFound);
        };
        Ok(session.artifacts.into_iter().collect())
    }

    pub(crate) fn transfer_owner(
        &mut self,
        capability: &FieldSessionCapability,
        owner_tag: u64,
        bound_task: Option<u64>,
    ) -> Result<FieldSessionCapability, FieldSessionError> {
        self.transfer_owner_with_cause(
            capability,
            owner_tag,
            bound_task,
            FieldProtocolReconfigurationCause::OwnerTransfer,
            Tick(0),
        )
    }

    pub(crate) fn transfer_owner_with_cause(
        &mut self,
        capability: &FieldSessionCapability,
        owner_tag: u64,
        bound_task: Option<u64>,
        cause: FieldProtocolReconfigurationCause,
        recorded_at: Tick,
    ) -> Result<FieldSessionCapability, FieldSessionError> {
        self.assert_owned(capability, None)?;
        let session = self
            .sessions
            .get_mut(&capability.session)
            .ok_or(FieldSessionError::NotFound)?;
        let prior_owner_tag = session.owner_tag;
        let prior_generation = session.generation;
        session.owner_tag = owner_tag;
        session.generation = session.generation.saturating_add(1);
        session.bound_task = bound_task;
        let updated = session.capability(&capability.session);
        push_bounded_reconfiguration(
            &mut self.reconfigurations,
            FieldProtocolReconfiguration {
                prior_session: capability.session.clone(),
                next_session: updated.session.clone(),
                protocol: capability.session.protocol,
                route_id: capability.session.route_id(),
                destination: capability.session.destination(),
                prior_owner_tag,
                next_owner_tag: updated.owner_tag,
                prior_generation,
                next_generation: updated.generation,
                cause,
                recorded_at,
            },
        );
        Ok(updated)
    }

    pub(crate) fn checkpoint_session(
        &mut self,
        capability: &FieldSessionCapability,
    ) -> Result<FieldProtocolCheckpoint, FieldSessionError> {
        self.assert_owned(capability, None)?;
        let session = self
            .sessions
            .get(&capability.session)
            .ok_or(FieldSessionError::NotFound)?;
        Ok(FieldProtocolCheckpoint {
            session: capability.session.clone(),
            owner_tag: session.owner_tag,
            generation: session.generation,
            bound_task: session.bound_task,
            blocked_receive: session.bridge.blocked_receive(),
            runtime_state: session.runtime.state,
            pending_outbound: session.bridge.outbound_summaries.iter().cloned().collect(),
            pending_inbound: session.bridge.inbound_summaries.iter().cloned().collect(),
            pending_branch_choices: session.bridge.branch_choices.iter().copied().collect(),
            artifacts: session.artifacts.iter().cloned().collect(),
        })
    }

    // long-block-exception: checkpoint restore intentionally rebuilds bridge
    // state, retained artifacts, and the replay-visible reconfiguration marker
    // in one place so the fail-closed restore boundary can be audited end to end.
    pub(crate) fn restore_session(
        &mut self,
        checkpoint: FieldProtocolCheckpoint,
    ) -> Result<FieldSessionCapability, FieldSessionError> {
        if self.sessions.len() >= FIELD_PROTOCOL_SESSION_MAX
            && !self.sessions.contains_key(&checkpoint.session)
        {
            return Err(FieldSessionError::TooManySessions);
        }
        let mut bridge = FieldBridgeState::default();
        for send in checkpoint.pending_outbound {
            bridge.queue_outbound_summary(send);
        }
        for receive in checkpoint.pending_inbound {
            bridge.stage_inbound_summary(receive);
        }
        for branch in checkpoint.pending_branch_choices {
            bridge.queue_branch_choice(branch);
        }
        if let Some(blocked) = checkpoint.blocked_receive {
            bridge.set_blocked_receive(blocked);
        }
        let mut artifacts = VecDeque::new();
        for artifact in checkpoint
            .artifacts
            .into_iter()
            .rev()
            .take(FIELD_PROTOCOL_ARTIFACT_RETENTION_MAX)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            artifacts.push_back(artifact);
        }
        let protocol = checkpoint.session.protocol;
        self.sessions.insert(
            checkpoint.session.clone(),
            OwnedFieldProtocolSession {
                owner_tag: checkpoint.owner_tag,
                generation: checkpoint.generation,
                bound_task: checkpoint.bound_task,
                bridge,
                runtime: FieldGuestRuntime::restore(protocol, checkpoint.runtime_state),
                artifacts,
                cancelled: false,
            },
        );
        let session = self
            .sessions
            .get(&checkpoint.session)
            .expect("restored session");
        let restored = session.capability(&checkpoint.session);
        push_bounded_reconfiguration(
            &mut self.reconfigurations,
            FieldProtocolReconfiguration {
                prior_session: checkpoint.session.clone(),
                next_session: restored.session.clone(),
                protocol: checkpoint.session.protocol,
                route_id: checkpoint.session.route_id(),
                destination: checkpoint.session.destination(),
                prior_owner_tag: checkpoint.owner_tag,
                next_owner_tag: restored.owner_tag,
                prior_generation: checkpoint.generation,
                next_generation: restored.generation,
                cause: FieldProtocolReconfigurationCause::CheckpointRestore,
                recorded_at: Tick(0),
            },
        );
        Ok(restored)
    }

    pub(crate) fn queue_summary_flow(
        &mut self,
        capability: &FieldSessionCapability,
        sends: impl IntoIterator<Item = QueuedProtocolSend>,
    ) -> Result<(), FieldSessionError> {
        self.assert_owned(capability, None)?;
        let session = self
            .sessions
            .get_mut(&capability.session)
            .ok_or(FieldSessionError::NotFound)?;
        session.note_new_work();
        for send in sends {
            session.bridge.queue_outbound_summary(send);
        }
        Ok(())
    }

    pub(crate) fn queue_branch_choice(
        &mut self,
        capability: &FieldSessionCapability,
        branch: u8,
    ) -> Result<(), FieldSessionError> {
        self.assert_owned(capability, None)?;
        let session = self
            .sessions
            .get_mut(&capability.session)
            .ok_or(FieldSessionError::NotFound)?;
        session.note_new_work();
        session.bridge.queue_branch_choice(branch);
        Ok(())
    }

    pub(crate) fn stage_receive(
        &mut self,
        capability: &FieldSessionCapability,
        receive: StagedProtocolReceive,
    ) -> Result<(), FieldSessionError> {
        self.assert_owned(capability, None)?;
        let session = self
            .sessions
            .get_mut(&capability.session)
            .ok_or(FieldSessionError::NotFound)?;
        session.note_new_work();
        session.bridge.stage_inbound_summary(receive);
        Ok(())
    }

    // long-block-exception: one bounded round intentionally keeps
    // cancellation, stepping, artifact capture, and outbound flush together.
    pub(crate) fn advance_host_bridged_round(
        &mut self,
        capability: &FieldSessionCapability,
        bound_task: Option<u64>,
        host_wait_status: FieldHostWaitStatus,
        now_tick: Tick,
    ) -> Result<FieldChoreographyAdvance, FieldSessionError> {
        self.assert_owned(capability, bound_task)?;
        let session = self
            .sessions
            .get_mut(&capability.session)
            .ok_or(FieldSessionError::NotFound)?;

        if matches!(host_wait_status, FieldHostWaitStatus::Cancelled) {
            session.cancelled = true;
            session.runtime.state = FieldGuestRuntimeState::Cancelled;
            session.bridge.clear_blocked_receive();
            return Ok(FieldChoreographyAdvance {
                round: FieldChoreographyRoundResult {
                    disposition: FieldRoundDisposition::Complete,
                    host_wait_status,
                    blocked_receive: None,
                    emitted_send_count: 0,
                    execution_policy: execution_policy_for(capability.session.protocol).class,
                    step_budget_remaining: execution_policy_for(capability.session.protocol)
                        .step_budget
                        .min(FIELD_PROTOCOL_STEP_BUDGET),
                },
                flushed_sends: Vec::new(),
                recorded_artifacts: Vec::new(),
            });
        }

        let (disposition, new_artifacts) =
            session
                .runtime
                .step(&capability.session, &mut session.bridge, now_tick);
        for artifact in &new_artifacts {
            push_bounded_artifact(&mut session.artifacts, artifact.clone());
        }
        let blocked_receive = session.bridge.blocked_receive();
        let flushed_sends = session.bridge.drain_outbound();
        if matches!(host_wait_status, FieldHostWaitStatus::Delivered) && blocked_receive.is_some() {
            session.bridge.clear_blocked_receive();
        }
        let execution_policy = execution_policy_for(capability.session.protocol);
        Ok(FieldChoreographyAdvance {
            round: FieldChoreographyRoundResult {
                disposition,
                host_wait_status,
                blocked_receive,
                emitted_send_count: flushed_sends.len(),
                execution_policy: execution_policy.class,
                step_budget_remaining: execution_policy
                    .step_budget
                    .min(FIELD_PROTOCOL_STEP_BUDGET)
                    .saturating_sub(session.runtime.step_count),
            },
            flushed_sends,
            recorded_artifacts: new_artifacts,
        })
    }

    fn assert_owned(
        &mut self,
        capability: &FieldSessionCapability,
        bound_task: Option<u64>,
    ) -> Result<(), FieldSessionError> {
        let session = self
            .sessions
            .get_mut(&capability.session)
            .ok_or(FieldSessionError::NotFound)?;
        if session.cancelled {
            return Err(FieldSessionError::Cancelled);
        }
        if session.generation != capability.generation {
            return Err(FieldSessionError::StaleCapabilityGeneration);
        }
        if session.owner_tag != capability.owner_tag {
            return Err(FieldSessionError::OwnershipMismatch);
        }
        if let (Some(expected), Some(actual)) = (session.bound_task, bound_task) {
            if expected != actual {
                session.cancelled = true;
                session.runtime.state = FieldGuestRuntimeState::Cancelled;
                return Err(FieldSessionError::BindingChanged);
            }
        }
        Ok(())
    }
}

fn execution_policy_for(protocol: FieldProtocolKind) -> FieldProtocolExecutionPolicy {
    match protocol {
        FieldProtocolKind::SummaryDissemination => FieldProtocolExecutionPolicy {
            class: FieldExecutionPolicyClass::Cheap,
            step_budget: 1,
        },
        FieldProtocolKind::AntiEntropy => FieldProtocolExecutionPolicy {
            class: FieldExecutionPolicyClass::Buffered,
            step_budget: 2,
        },
        FieldProtocolKind::RetentionReplay => FieldProtocolExecutionPolicy {
            class: FieldExecutionPolicyClass::Buffered,
            step_budget: 2,
        },
        FieldProtocolKind::ExplicitCoordination => FieldProtocolExecutionPolicy {
            class: FieldExecutionPolicyClass::Coordinated,
            step_budget: 4,
        },
    }
}

fn push_bounded_artifact(
    artifacts: &mut VecDeque<FieldProtocolArtifact>,
    artifact: FieldProtocolArtifact,
) {
    if artifacts.len() >= FIELD_PROTOCOL_ARTIFACT_RETENTION_MAX {
        artifacts.pop_front();
    }
    artifacts.push_back(artifact);
}

fn push_bounded_reconfiguration(
    reconfigurations: &mut VecDeque<FieldProtocolReconfiguration>,
    reconfiguration: FieldProtocolReconfiguration,
) {
    if reconfigurations.len() >= FIELD_PROTOCOL_RECONFIGURATION_RETENTION_MAX {
        reconfigurations.pop_front();
    }
    reconfigurations.push_back(reconfiguration);
}

#[cfg(test)]
mod tests {
    use jacquard_core::{DestinationId, RouteEpoch};

    use super::*;
    use crate::{
        state::{EntropyBucket, HopBand, SupportBucket},
        summary::{EvidenceContributionClass, SummaryUncertaintyClass},
    };

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn summary(byte: u8) -> FieldSummary {
        FieldSummary {
            destination: SummaryDestinationKey::from(&DestinationId::Node(node(byte))),
            topology_epoch: RouteEpoch(1),
            freshness_tick: Tick(1),
            hop_band: HopBand::new(1, 2),
            delivery_support: SupportBucket::new(800),
            congestion_penalty: EntropyBucket::new(100),
            retention_support: SupportBucket::new(200),
            uncertainty_penalty: EntropyBucket::new(50),
            evidence_class: EvidenceContributionClass::ForwardPropagated,
            uncertainty_class: SummaryUncertaintyClass::Low,
        }
    }

    fn session_key(protocol: FieldProtocolKind) -> FieldProtocolSessionKey {
        FieldProtocolSessionKey {
            protocol,
            route_id: None,
            topology_epoch: RouteEpoch(1),
            destination: Some(SummaryDestinationKey::from(&DestinationId::Node(node(7)))),
        }
    }

    #[test]
    fn artifact_detail_is_bounded() {
        let detail = FieldProtocolArtifactDetail::new("x".repeat(200));
        assert_eq!(detail.as_str().len(), FIELD_PROTOCOL_ARTIFACT_LIMIT);
    }

    #[test]
    fn bridge_state_queues_are_bounded() {
        let mut bridge = FieldBridgeState::default();
        for byte in 0..=u8::try_from(FIELD_PROTOCOL_QUEUE_MAX).unwrap() {
            bridge.queue_outbound_summary(QueuedProtocolSend {
                protocol: FieldProtocolKind::SummaryDissemination,
                to_neighbor: node(byte),
                payload: summary(byte).encode(),
            });
        }
        assert_eq!(bridge.drain_outbound().len(), FIELD_PROTOCOL_QUEUE_MAX);
    }

    #[test]
    fn blocked_receive_marker_round_trips() {
        let mut bridge = FieldBridgeState::default();
        bridge.set_blocked_receive(BlockedReceiveMarker::Neighbor(node(9)));
        assert_eq!(
            bridge.blocked_receive(),
            Some(BlockedReceiveMarker::Neighbor(node(9)))
        );
        bridge.clear_blocked_receive();
        assert_eq!(bridge.blocked_receive(), None);
    }

    #[test]
    fn summary_round_flushes_send_and_completes() {
        let mut runtime = FieldProtocolRuntime::default();
        let capability = runtime
            .open_session(
                &session_key(FieldProtocolKind::SummaryDissemination),
                11,
                Some(21),
            )
            .expect("session");
        runtime
            .queue_summary_flow(
                &capability,
                [QueuedProtocolSend {
                    protocol: FieldProtocolKind::SummaryDissemination,
                    to_neighbor: node(9),
                    payload: summary(9).encode(),
                }],
            )
            .expect("queue");
        let advance = runtime
            .advance_host_bridged_round(&capability, Some(21), FieldHostWaitStatus::Idle, Tick(2))
            .expect("advance");
        assert_eq!(advance.round.disposition, FieldRoundDisposition::Complete);
        assert_eq!(advance.round.emitted_send_count, 1);
        assert_eq!(
            advance.round.execution_policy,
            FieldExecutionPolicyClass::Cheap
        );
        assert_eq!(advance.round.step_budget_remaining, 0);
        assert_eq!(advance.flushed_sends.len(), 1);
        assert_eq!(advance.recorded_artifacts.len(), 1);
    }

    #[test]
    fn anti_entropy_checkpoint_recovery_completes_after_receive() {
        let mut runtime = FieldProtocolRuntime::default();
        let capability = runtime
            .open_session(&session_key(FieldProtocolKind::AntiEntropy), 1, Some(2))
            .expect("session");
        let first = runtime
            .advance_host_bridged_round(&capability, Some(2), FieldHostWaitStatus::Idle, Tick(3))
            .expect("first round");
        assert_eq!(first.round.disposition, FieldRoundDisposition::Continue);
        assert_eq!(
            first.round.blocked_receive,
            Some(BlockedReceiveMarker::AnyPeer)
        );

        let checkpoint = runtime.checkpoint_session(&capability).expect("checkpoint");
        let mut recovered_runtime = FieldProtocolRuntime::default();
        let recovered = recovered_runtime
            .restore_session(checkpoint)
            .expect("restore");
        recovered_runtime
            .stage_receive(
                &recovered,
                StagedProtocolReceive {
                    protocol: FieldProtocolKind::AntiEntropy,
                    from_neighbor: node(5),
                    payload: summary(5).encode(),
                },
            )
            .expect("receive");
        let second = recovered_runtime
            .advance_host_bridged_round(
                &recovered,
                Some(2),
                FieldHostWaitStatus::Delivered,
                Tick(4),
            )
            .expect("second round");
        assert_eq!(second.round.disposition, FieldRoundDisposition::Complete);
        assert_eq!(
            second.round.host_wait_status,
            FieldHostWaitStatus::Delivered
        );
        assert!(second
            .recorded_artifacts
            .iter()
            .any(|artifact| { artifact.detail.as_str() == "anti-entropy-received" }));
        assert!(recovered_runtime
            .reconfigurations()
            .iter()
            .any(|reconfiguration| {
                reconfiguration.cause == FieldProtocolReconfigurationCause::CheckpointRestore
                    && reconfiguration.prior_session == reconfiguration.next_session
            }));
    }

    #[test]
    fn owner_transfer_invalidates_old_generation() {
        let mut runtime = FieldProtocolRuntime::default();
        let capability = runtime
            .open_session(
                &session_key(FieldProtocolKind::ExplicitCoordination),
                100,
                Some(200),
            )
            .expect("session");
        let transferred = runtime
            .transfer_owner(&capability, 101, Some(201))
            .expect("transfer");
        assert_eq!(
            runtime
                .advance_host_bridged_round(
                    &capability,
                    Some(200),
                    FieldHostWaitStatus::Idle,
                    Tick(5),
                )
                .expect_err("stale capability must fail"),
            FieldSessionError::StaleCapabilityGeneration
        );
        runtime
            .queue_branch_choice(&transferred, 1)
            .expect("branch");
        let round = runtime
            .advance_host_bridged_round(&transferred, Some(201), FieldHostWaitStatus::Idle, Tick(5))
            .expect("new owner round");
        assert_eq!(round.round.disposition, FieldRoundDisposition::Complete);
        assert!(runtime.reconfigurations().iter().any(|reconfiguration| {
            reconfiguration.cause == FieldProtocolReconfigurationCause::OwnerTransfer
                && reconfiguration.prior_owner_tag == 100
                && reconfiguration.next_owner_tag == 101
                && reconfiguration.prior_generation == 0
                && reconfiguration.next_generation == 1
        }));
    }

    #[test]
    fn binding_change_cancels_session_fail_closed() {
        let mut runtime = FieldProtocolRuntime::default();
        let capability = runtime
            .open_session(&session_key(FieldProtocolKind::AntiEntropy), 5, Some(10))
            .expect("session");
        assert_eq!(
            runtime
                .advance_host_bridged_round(
                    &capability,
                    Some(11),
                    FieldHostWaitStatus::Idle,
                    Tick(6),
                )
                .expect_err("binding drift must cancel"),
            FieldSessionError::BindingChanged
        );
        assert_eq!(
            runtime
                .advance_host_bridged_round(
                    &capability,
                    Some(10),
                    FieldHostWaitStatus::Idle,
                    Tick(7),
                )
                .expect_err("cancelled session stays invalid"),
            FieldSessionError::Cancelled
        );
    }

    #[test]
    fn retention_replay_and_explicit_coordination_use_guest_runtime_paths() {
        let mut runtime = FieldProtocolRuntime::default();
        let replay = runtime
            .open_session(&session_key(FieldProtocolKind::RetentionReplay), 8, Some(9))
            .expect("replay");
        runtime.queue_branch_choice(&replay, 1).expect("branch");
        runtime
            .queue_summary_flow(
                &replay,
                [QueuedProtocolSend {
                    protocol: FieldProtocolKind::RetentionReplay,
                    to_neighbor: node(4),
                    payload: summary(4).encode(),
                }],
            )
            .expect("queue");
        let replay_round = runtime
            .advance_host_bridged_round(&replay, Some(9), FieldHostWaitStatus::Idle, Tick(8))
            .expect("replay round");
        assert_eq!(
            replay_round.round.disposition,
            FieldRoundDisposition::Complete
        );
        assert_eq!(replay_round.round.emitted_send_count, 1);

        let coordination = runtime
            .open_session(
                &session_key(FieldProtocolKind::ExplicitCoordination),
                8,
                Some(9),
            )
            .expect("coordination");
        runtime
            .queue_branch_choice(&coordination, 1)
            .expect("branch");
        let coordination_round = runtime
            .advance_host_bridged_round(&coordination, Some(9), FieldHostWaitStatus::Idle, Tick(9))
            .expect("coordination round");
        assert_eq!(
            coordination_round.round.disposition,
            FieldRoundDisposition::Complete
        );
        assert!(coordination_round
            .recorded_artifacts
            .iter()
            .any(|artifact| { artifact.detail.as_str() == "explicit-coordination" }));
    }

    #[test]
    fn choreography_artifact_retention_is_bounded() {
        let mut runtime = FieldProtocolRuntime::default();
        let capability = runtime
            .open_session(
                &session_key(FieldProtocolKind::SummaryDissemination),
                55,
                Some(66),
            )
            .expect("session");

        for index in 0..(FIELD_PROTOCOL_ARTIFACT_RETENTION_MAX + 3) {
            runtime
                .queue_summary_flow(
                    &capability,
                    [QueuedProtocolSend {
                        protocol: FieldProtocolKind::SummaryDissemination,
                        to_neighbor: node(u8::try_from(index + 1).unwrap()),
                        payload: summary(u8::try_from(index + 1).unwrap()).encode(),
                    }],
                )
                .expect("queue");
            std::mem::drop(
                runtime
                    .advance_host_bridged_round(
                        &capability,
                        Some(66),
                        FieldHostWaitStatus::Idle,
                        Tick(u64::try_from(index + 10).unwrap()),
                    )
                    .expect("advance"),
            );
        }

        let checkpoint = runtime.checkpoint_session(&capability).expect("checkpoint");
        assert!(checkpoint.artifacts.len() <= FIELD_PROTOCOL_ARTIFACT_RETENTION_MAX);
    }

    #[test]
    fn waiting_rounds_enforce_policy_step_budget() {
        let mut runtime = FieldProtocolRuntime::default();
        let capability = runtime
            .open_session(
                &session_key(FieldProtocolKind::ExplicitCoordination),
                11,
                Some(21),
            )
            .expect("session");

        let mut last = None;
        for tick in 0..=4 {
            last = Some(
                runtime
                    .advance_host_bridged_round(
                        &capability,
                        Some(21),
                        FieldHostWaitStatus::Idle,
                        Tick(u64::try_from(tick + 20).unwrap()),
                    )
                    .expect("advance"),
            );
        }

        let last = last.expect("final advance");
        assert_eq!(
            last.round.execution_policy,
            FieldExecutionPolicyClass::Coordinated
        );
        assert_eq!(last.round.step_budget_remaining, 0);
        assert!(last
            .recorded_artifacts
            .iter()
            .any(|artifact| { artifact.detail.as_str() == "step-budget-exceeded" }));
    }
}
