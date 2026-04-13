//! Route-scoped recovery state for field runtime continuity.
//!
//! Field keeps protocol/session continuity private, but it still needs one
//! reduced route-scoped recovery record so replay and client tooling can see
//! whether checkpoint/restore and continuation-shift reuse happened.

use serde::{Deserialize, Serialize};

use crate::choreography::FieldProtocolCheckpoint;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldRouteRecoveryTrigger {
    SuspendForRuntimeLoss,
    RestoreRuntime,
    ContinuationShift,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldRouteRecoveryOutcome {
    CheckpointStored,
    CheckpointRestored,
    FreshSessionInstalled,
    ContinuationRetained,
    NoCheckpointAvailable,
    RecoveryFailed,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldRouteRecoveryState {
    pub checkpoint_available: bool,
    pub last_trigger: Option<FieldRouteRecoveryTrigger>,
    pub last_outcome: Option<FieldRouteRecoveryOutcome>,
    pub checkpoint_capture_count: u32,
    pub checkpoint_restore_count: u32,
    pub continuation_shift_count: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct StoredFieldRouteRecovery {
    pub(crate) checkpoint: Option<FieldProtocolCheckpoint>,
    pub(crate) state: FieldRouteRecoveryState,
}

impl StoredFieldRouteRecovery {
    pub(crate) fn note_checkpoint_stored(&mut self, checkpoint: FieldProtocolCheckpoint) {
        self.checkpoint = Some(checkpoint);
        self.state.checkpoint_available = true;
        self.state.last_trigger = Some(FieldRouteRecoveryTrigger::SuspendForRuntimeLoss);
        self.state.last_outcome = Some(FieldRouteRecoveryOutcome::CheckpointStored);
        self.state.checkpoint_capture_count = self.state.checkpoint_capture_count.saturating_add(1);
    }

    pub(crate) fn note_checkpoint_restored(&mut self) {
        self.checkpoint = None;
        self.state.checkpoint_available = false;
        self.state.last_trigger = Some(FieldRouteRecoveryTrigger::RestoreRuntime);
        self.state.last_outcome = Some(FieldRouteRecoveryOutcome::CheckpointRestored);
        self.state.checkpoint_restore_count = self.state.checkpoint_restore_count.saturating_add(1);
    }

    pub(crate) fn note_fresh_session_installed(&mut self) {
        self.checkpoint = None;
        self.state.checkpoint_available = false;
        self.state.last_trigger = Some(FieldRouteRecoveryTrigger::RestoreRuntime);
        self.state.last_outcome = Some(FieldRouteRecoveryOutcome::FreshSessionInstalled);
    }

    pub(crate) fn note_continuation_retained(&mut self) {
        self.state.last_trigger = Some(FieldRouteRecoveryTrigger::ContinuationShift);
        self.state.last_outcome = Some(FieldRouteRecoveryOutcome::ContinuationRetained);
        self.state.continuation_shift_count = self.state.continuation_shift_count.saturating_add(1);
    }

    pub(crate) fn note_no_checkpoint_available(&mut self) {
        self.state.checkpoint_available = self.checkpoint.is_some();
        self.state.last_trigger = Some(FieldRouteRecoveryTrigger::RestoreRuntime);
        self.state.last_outcome = Some(FieldRouteRecoveryOutcome::NoCheckpointAvailable);
    }

    pub(crate) fn note_recovery_failed(&mut self, trigger: FieldRouteRecoveryTrigger) {
        self.state.checkpoint_available = self.checkpoint.is_some();
        self.state.last_trigger = Some(trigger);
        self.state.last_outcome = Some(FieldRouteRecoveryOutcome::RecoveryFailed);
    }
}
