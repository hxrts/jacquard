//! Route-scoped recovery state for field runtime continuity.
//!
//! Field keeps protocol/session continuity private, but it still needs one
//! reduced route-scoped recovery record so replay and client tooling can see
//! whether checkpoint/restore and continuation-shift reuse happened.

use serde::{Deserialize, Serialize};

use crate::{choreography::FieldProtocolCheckpoint, route::FieldContinuityBand};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldRouteRecoveryTrigger {
    SuspendForRuntimeLoss,
    RestoreRuntime,
    ContinuationShift,
    EnvelopeNarrowing,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldRouteRecoveryOutcome {
    CheckpointStored,
    CheckpointRestored,
    FreshSessionInstalled,
    ContinuationRetained,
    ContinuityNarrowed,
    NoCheckpointAvailable,
    RecoveryFailed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldBootstrapTransition {
    Activated,
    Held,
    Narrowed,
    Upgraded,
    Withdrawn,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldContinuityTransition {
    EnteredDegradedSteady,
    RecoveredSteady,
    DowngradedToBootstrap,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldPromotionDecision {
    Hold,
    Narrow,
    Promote,
    Withdraw,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldPromotionBlocker {
    SupportTrend,
    Uncertainty,
    AntiEntropyConfirmation,
    ContinuationCoherence,
    Freshness,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldRouteRecoveryState {
    pub checkpoint_available: bool,
    pub last_trigger: Option<FieldRouteRecoveryTrigger>,
    pub last_outcome: Option<FieldRouteRecoveryOutcome>,
    pub bootstrap_active: bool,
    pub continuity_band: Option<FieldContinuityBand>,
    pub last_continuity_transition: Option<FieldContinuityTransition>,
    pub last_bootstrap_transition: Option<FieldBootstrapTransition>,
    pub last_promotion_decision: Option<FieldPromotionDecision>,
    pub last_promotion_blocker: Option<FieldPromotionBlocker>,
    pub bootstrap_activation_count: u32,
    pub bootstrap_hold_count: u32,
    pub bootstrap_narrow_count: u32,
    pub bootstrap_upgrade_count: u32,
    pub bootstrap_withdraw_count: u32,
    pub degraded_steady_entry_count: u32,
    pub degraded_steady_recovery_count: u32,
    pub degraded_to_bootstrap_count: u32,
    pub degraded_steady_round_count: u32,
    pub service_retention_carry_forward_count: u32,
    pub asymmetric_shift_success_count: u32,
    pub checkpoint_capture_count: u32,
    pub checkpoint_restore_count: u32,
    pub continuation_shift_count: u32,
    pub continuity_narrow_count: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct StoredFieldRouteRecovery {
    pub(crate) checkpoint: Option<FieldProtocolCheckpoint>,
    pub(crate) state: FieldRouteRecoveryState,
}

impl StoredFieldRouteRecovery {
    pub(crate) fn note_continuity_band(&mut self, band: FieldContinuityBand) {
        self.state.continuity_band = Some(band);
        if band == FieldContinuityBand::DegradedSteady {
            self.state.degraded_steady_round_count =
                self.state.degraded_steady_round_count.saturating_add(1);
        }
    }

    pub(crate) fn note_degraded_steady_entered(&mut self) {
        self.state.continuity_band = Some(FieldContinuityBand::DegradedSteady);
        self.state.last_continuity_transition =
            Some(FieldContinuityTransition::EnteredDegradedSteady);
        self.state.degraded_steady_entry_count =
            self.state.degraded_steady_entry_count.saturating_add(1);
        self.state.degraded_steady_round_count =
            self.state.degraded_steady_round_count.saturating_add(1);
    }

    pub(crate) fn note_degraded_steady_recovered(&mut self) {
        self.state.continuity_band = Some(FieldContinuityBand::Steady);
        self.state.last_continuity_transition = Some(FieldContinuityTransition::RecoveredSteady);
        self.state.degraded_steady_recovery_count =
            self.state.degraded_steady_recovery_count.saturating_add(1);
    }

    pub(crate) fn note_degraded_to_bootstrap(&mut self) {
        self.state.continuity_band = Some(FieldContinuityBand::Bootstrap);
        self.state.last_continuity_transition =
            Some(FieldContinuityTransition::DowngradedToBootstrap);
        self.state.degraded_to_bootstrap_count =
            self.state.degraded_to_bootstrap_count.saturating_add(1);
    }

    pub(crate) fn note_service_retention_carry_forward(&mut self) {
        self.state.service_retention_carry_forward_count = self
            .state
            .service_retention_carry_forward_count
            .saturating_add(1);
    }

    pub(crate) fn note_asymmetric_shift_success(&mut self) {
        self.state.asymmetric_shift_success_count =
            self.state.asymmetric_shift_success_count.saturating_add(1);
    }

    pub(crate) fn note_checkpoint_stored(&mut self, checkpoint: FieldProtocolCheckpoint) {
        self.checkpoint = Some(checkpoint);
        self.state.checkpoint_available = true;
        self.state.last_trigger = Some(FieldRouteRecoveryTrigger::SuspendForRuntimeLoss);
        self.state.last_outcome = Some(FieldRouteRecoveryOutcome::CheckpointStored);
        self.state.checkpoint_capture_count = self.state.checkpoint_capture_count.saturating_add(1);
    }

    pub(crate) fn note_bootstrap_activated(&mut self) {
        self.state.bootstrap_active = true;
        self.state.continuity_band = Some(FieldContinuityBand::Bootstrap);
        self.state.last_bootstrap_transition = Some(FieldBootstrapTransition::Activated);
        self.state.last_promotion_decision = None;
        self.state.last_promotion_blocker = None;
        self.state.bootstrap_activation_count =
            self.state.bootstrap_activation_count.saturating_add(1);
    }

    pub(crate) fn note_bootstrap_held(&mut self, blocker: FieldPromotionBlocker) {
        self.state.bootstrap_active = true;
        self.state.continuity_band = Some(FieldContinuityBand::Bootstrap);
        self.state.last_bootstrap_transition = Some(FieldBootstrapTransition::Held);
        self.state.last_promotion_decision = Some(FieldPromotionDecision::Hold);
        self.state.last_promotion_blocker = Some(blocker);
        self.state.bootstrap_hold_count = self.state.bootstrap_hold_count.saturating_add(1);
    }

    pub(crate) fn note_bootstrap_narrowed(&mut self, blocker: FieldPromotionBlocker) {
        self.state.bootstrap_active = true;
        self.state.continuity_band = Some(FieldContinuityBand::Bootstrap);
        self.state.last_trigger = Some(FieldRouteRecoveryTrigger::EnvelopeNarrowing);
        self.state.last_outcome = Some(FieldRouteRecoveryOutcome::ContinuityNarrowed);
        self.state.last_bootstrap_transition = Some(FieldBootstrapTransition::Narrowed);
        self.state.last_promotion_decision = Some(FieldPromotionDecision::Narrow);
        self.state.last_promotion_blocker = Some(blocker);
        self.state.bootstrap_narrow_count = self.state.bootstrap_narrow_count.saturating_add(1);
        self.state.continuity_narrow_count = self.state.continuity_narrow_count.saturating_add(1);
    }

    pub(crate) fn note_bootstrap_upgraded(&mut self) {
        self.state.bootstrap_active = false;
        self.state.continuity_band = Some(FieldContinuityBand::Steady);
        self.state.last_bootstrap_transition = Some(FieldBootstrapTransition::Upgraded);
        self.state.last_promotion_decision = Some(FieldPromotionDecision::Promote);
        self.state.last_promotion_blocker = None;
        self.state.bootstrap_upgrade_count = self.state.bootstrap_upgrade_count.saturating_add(1);
    }

    pub(crate) fn note_bootstrap_withdrawn(&mut self, blocker: FieldPromotionBlocker) {
        self.state.bootstrap_active = false;
        self.state.continuity_band = Some(FieldContinuityBand::Bootstrap);
        self.state.last_bootstrap_transition = Some(FieldBootstrapTransition::Withdrawn);
        self.state.last_promotion_decision = Some(FieldPromotionDecision::Withdraw);
        self.state.last_promotion_blocker = Some(blocker);
        self.state.bootstrap_withdraw_count = self.state.bootstrap_withdraw_count.saturating_add(1);
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

    pub(crate) fn note_continuity_narrowed(&mut self) {
        self.state.last_trigger = Some(FieldRouteRecoveryTrigger::EnvelopeNarrowing);
        self.state.last_outcome = Some(FieldRouteRecoveryOutcome::ContinuityNarrowed);
        self.state.continuity_narrow_count = self.state.continuity_narrow_count.saturating_add(1);
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
