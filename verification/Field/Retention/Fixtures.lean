import Field.Retention.Refinement

/-
The Problem. The retention layer needs concrete reduced scenarios so the new
theorem surface is anchored to executable examples rather than only abstract
statements.

Solution Structure.
1. Define a small base retention state and representative policy inputs.
2. Add one fixture each for retain, forward, drop, and checkpoint-restore.
3. Prove the intended outcome for each fixture directly.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRetentionFixtures

open FieldModelAPI
open FieldRetentionAPI
open FieldRetentionInstance
open FieldRetentionRefinement

/-! ## Fixture Inputs And States -/

def baseToken : PayloadToken :=
  { messageId := 11
    destination := .corridorA
    sizeClass := .small
    ageClass := .fresh
    custodyClass := .localOnly
    retainedAtTick := ⟨2⟩
    lastRetryTick := ⟨2⟩
    admittedRouteEpoch := ⟨7⟩ }

def staleToken : PayloadToken :=
  { baseToken with messageId := 12, ageClass := .stale }

def staleEpochToken : PayloadToken :=
  { baseToken with messageId := 14, admittedRouteEpoch := ⟨6⟩ }

def expiredToken : PayloadToken :=
  { baseToken with messageId := 15, retainedAtTick := ⟨0⟩, lastRetryTick := ⟨0⟩ }

def baseRetentionState : RetentionState :=
  mkRetentionState [baseToken] 0 0 none

def staleRetentionState : RetentionState :=
  mkRetentionState [staleToken] 0 0 none

def noRouteRetainInput : RetentionPolicyInput :=
  { regime := .sparse
    posture := .structured
    supportBand := .medium
    uncertaintyBand := .stable
    continuity := .bootstrap
    continuationAvailable := false
    routeInstalled := false
    nowTick := ⟨4⟩
    activeRouteEpoch := ⟨7⟩
    custodyTimeout := ⟨5⟩ }

def steadyForwardInput : RetentionPolicyInput :=
  { regime := .sparse
    posture := .structured
    supportBand := .medium
    uncertaintyBand := .stable
    continuity := .steady
    continuationAvailable := true
    routeInstalled := true
    nowTick := ⟨4⟩
    activeRouteEpoch := ⟨7⟩
    custodyTimeout := ⟨5⟩ }

def riskyBootstrapRetainInput : RetentionPolicyInput :=
  { regime := .retentionFavorable
    posture := .riskSuppressed
    supportBand := .medium
    uncertaintyBand := .risky
    continuity := .bootstrap
    continuationAvailable := false
    routeInstalled := true
    nowTick := ⟨4⟩
    activeRouteEpoch := ⟨7⟩
    custodyTimeout := ⟨5⟩ }

def staleDropInput : RetentionPolicyInput :=
  { regime := .unstable
    posture := .structured
    supportBand := .low
    uncertaintyBand := .risky
    continuity := .degradedSteady
    continuationAvailable := false
    routeInstalled := true
    nowTick := ⟨4⟩
    activeRouteEpoch := ⟨7⟩
    custodyTimeout := ⟨5⟩ }

def staleEpochRetainInput : RetentionPolicyInput :=
  { steadyForwardInput with activeRouteEpoch := ⟨8⟩ }

def expiredDropInput : RetentionPolicyInput :=
  { steadyForwardInput with nowTick := ⟨10⟩, custodyTimeout := ⟨5⟩ }

def checkpointRestoreRetentionState : RetentionState :=
  mkRetentionState
    [ baseToken
    , { baseToken with messageId := 13, destination := .corridorB, ageClass := .warm } ]
    1
    0
    (some .carry)

/-! ## Fixture Theorems -/

theorem no_route_retain_fixture_selects_retain :
    selectRetentionDecisionImpl noRouteRetainInput baseToken = .retain := by
  exact
    selectRetentionDecision_no_route_retains
      noRouteRetainInput
      baseToken
      (by decide)
      rfl

theorem no_route_retain_fixture_step_marks_retain :
    (retentionStepImpl noRouteRetainInput baseRetentionState).lastDecision = some .retain := by
  decide

theorem steady_forward_fixture_selects_forward :
    selectRetentionDecisionImpl steadyForwardInput baseToken = .forward := by
  exact
    selectRetentionDecision_steady_with_continuation_forwards
      steadyForwardInput
      baseToken
      (by decide)
      rfl
      rfl
      (Or.inl (by decide))
      (Or.inl (by decide))
      (Or.inl (by decide))
      rfl
      rfl

theorem steady_forward_fixture_releases_one_token :
    (retentionStepImpl steadyForwardInput baseRetentionState).buffer = [] ∧
      (retentionStepImpl steadyForwardInput baseRetentionState).deliveredCount = 1 := by
  decide

theorem risky_bootstrap_fixture_selects_retain :
    selectRetentionDecisionImpl riskyBootstrapRetainInput baseToken = .retain := by
  decide

theorem risky_bootstrap_fixture_step_marks_retain :
    (retentionStepImpl riskyBootstrapRetainInput baseRetentionState).lastDecision = some .retain := by
  decide

theorem stale_drop_fixture_selects_drop :
    selectRetentionDecisionImpl staleDropInput staleToken = .drop := by
  decide

theorem stale_epoch_fixture_selects_retain :
    selectRetentionDecisionImpl staleEpochRetainInput staleEpochToken = .retain := by
  exact
    selectRetentionDecision_stale_epoch_retains
      staleEpochRetainInput
      staleEpochToken
      (by decide)
      rfl
      (by decide)

theorem expired_custody_fixture_selects_drop :
    selectRetentionDecisionImpl expiredDropInput expiredToken = .drop := by
  exact
    selectRetentionDecision_expired_drops
      expiredDropInput
      expiredToken
      (by decide)

theorem stale_drop_fixture_drops_and_counts_explicitly :
    (retentionStepImpl staleDropInput staleRetentionState).buffer = [] ∧
      (retentionStepImpl staleDropInput staleRetentionState).droppedCount = 1 ∧
      (retentionStepImpl staleDropInput staleRetentionState).lastDecision = some .drop := by
  decide

theorem checkpoint_restore_fixture_preserves_buffer :
    (restoreRetentionStateImpl checkpointRestoreRetentionState).buffer =
      checkpointRestoreRetentionState.buffer := by
  exact checkpoint_restore_preserves_retained_multiset checkpointRestoreRetentionState (by decide)

theorem checkpoint_restore_fixture_clears_last_decision :
    (restoreRetentionStateImpl checkpointRestoreRetentionState).lastDecision = none := by
  simp [restoreRetentionStateImpl, checkpointRestoreRetentionState, mkRetentionState, normalizeBuffer]

theorem checkpoint_restore_fixture_preserves_retention_ticks :
    ((restoreRetentionStateImpl checkpointRestoreRetentionState).buffer.map PayloadToken.retainedAtTick) =
      (checkpointRestoreRetentionState.buffer.map PayloadToken.retainedAtTick) := by
  exact
    checkpoint_restore_preserves_retention_timestamps
      checkpointRestoreRetentionState
      (by decide)

theorem checkpoint_restore_fixture_preserves_route_epochs :
    ((restoreRetentionStateImpl checkpointRestoreRetentionState).buffer.map PayloadToken.admittedRouteEpoch) =
      (checkpointRestoreRetentionState.buffer.map PayloadToken.admittedRouteEpoch) := by
  exact
    checkpoint_restore_preserves_retention_epochs
      checkpointRestoreRetentionState
      (by decide)

end FieldRetentionFixtures
