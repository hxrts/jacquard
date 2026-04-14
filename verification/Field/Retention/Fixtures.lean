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
    custodyClass := .localOnly }

def staleToken : PayloadToken :=
  { baseToken with messageId := 12, ageClass := .stale }

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
    routeInstalled := false }

def steadyForwardInput : RetentionPolicyInput :=
  { regime := .sparse
    posture := .structured
    supportBand := .medium
    uncertaintyBand := .stable
    continuity := .steady
    continuationAvailable := true
    routeInstalled := true }

def riskyBootstrapRetainInput : RetentionPolicyInput :=
  { regime := .retentionFavorable
    posture := .riskSuppressed
    supportBand := .medium
    uncertaintyBand := .risky
    continuity := .bootstrap
    continuationAvailable := false
    routeInstalled := true }

def staleDropInput : RetentionPolicyInput :=
  { regime := .unstable
    posture := .structured
    supportBand := .low
    uncertaintyBand := .risky
    continuity := .degradedSteady
    continuationAvailable := false
    routeInstalled := true }

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
  exact selectRetentionDecision_no_route_retains noRouteRetainInput baseToken rfl

theorem no_route_retain_fixture_step_marks_retain :
    (retentionStepImpl noRouteRetainInput baseRetentionState).lastDecision = some .retain := by
  native_decide

theorem steady_forward_fixture_selects_forward :
    selectRetentionDecisionImpl steadyForwardInput baseToken = .forward := by
  exact
    selectRetentionDecision_steady_with_continuation_forwards
      steadyForwardInput
      baseToken
      rfl
      (Or.inl (by decide))
      (Or.inl (by decide))
      (Or.inl (by decide))
      rfl
      rfl

theorem steady_forward_fixture_releases_one_token :
    (retentionStepImpl steadyForwardInput baseRetentionState).buffer = [] ∧
      (retentionStepImpl steadyForwardInput baseRetentionState).deliveredCount = 1 := by
  native_decide

theorem risky_bootstrap_fixture_selects_retain :
    selectRetentionDecisionImpl riskyBootstrapRetainInput baseToken = .retain := by
  simp [selectRetentionDecisionImpl, riskyBootstrapRetainInput, baseToken]

theorem risky_bootstrap_fixture_step_marks_retain :
    (retentionStepImpl riskyBootstrapRetainInput baseRetentionState).lastDecision = some .retain := by
  native_decide

theorem stale_drop_fixture_selects_drop :
    selectRetentionDecisionImpl staleDropInput staleToken = .drop := by
  simp [selectRetentionDecisionImpl, staleDropInput, staleToken, baseToken]

theorem stale_drop_fixture_drops_and_counts_explicitly :
    (retentionStepImpl staleDropInput staleRetentionState).buffer = [] ∧
      (retentionStepImpl staleDropInput staleRetentionState).droppedCount = 1 ∧
      (retentionStepImpl staleDropInput staleRetentionState).lastDecision = some .drop := by
  native_decide

theorem checkpoint_restore_fixture_preserves_buffer :
    (restoreRetentionStateImpl checkpointRestoreRetentionState).buffer =
      checkpointRestoreRetentionState.buffer := by
  exact checkpoint_restore_preserves_retained_multiset checkpointRestoreRetentionState (by decide)

theorem checkpoint_restore_fixture_clears_last_decision :
    (restoreRetentionStateImpl checkpointRestoreRetentionState).lastDecision = none := by
  simp [restoreRetentionStateImpl, checkpointRestoreRetentionState, mkRetentionState, normalizeBuffer]

end FieldRetentionFixtures
