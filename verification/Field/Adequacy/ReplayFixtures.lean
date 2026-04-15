import Field.Adequacy.Search

/- 
The Problem. The field proof stack already had reduced runtime fixtures and
reduced protocol fixtures, but it did not have one maintained vocabulary tied
directly to the exported Rust replay bundle used by the implementation.

Solution Structure.
1. Define one reduced replay-derived fixture vocabulary.
2. Mirror the maintained exported Rust replay families with concrete fixtures.
3. Package small theorem surfaces that pin the expected query, protocol, and
   recovery facts for those fixtures.
-/

/-! # Adequacy.ReplayFixtures — reduced replay-derived fixture vocabulary -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyReplayFixtures

/-! ## Fixture Vocabulary -/

structure RustReplaySearchFixture where
  objectiveClass : String
  queryKind : String
  selectedNeighborPresent : Bool
  snapshotEpochPresent : Bool
  planningFailure : Option String
  deriving Inhabited, Repr, DecidableEq, BEq

structure RustReplayProtocolFixture where
  reconfigurationCauses : List String
  routeBoundReconfigurationCount : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

structure RustReplayRuntimeLinkageFixture where
  artifactCount : Nat
  searchLinkedArtifactCount : Nat
  routeArtifactCount : Nat
  bootstrapRouteArtifactCount : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

structure RustReplayRecoveryFixture where
  lastTrigger : Option String
  lastOutcome : Option String
  bootstrapActive : Bool
  continuityBand : Option String
  lastContinuityTransition : Option String
  lastBootstrapTransition : Option String
  lastPromotionDecision : Option String
  lastPromotionBlocker : Option String
  bootstrapActivationCount : Nat
  bootstrapHoldCount : Nat
  bootstrapNarrowCount : Nat
  bootstrapUpgradeCount : Nat
  bootstrapWithdrawCount : Nat
  degradedSteadyEntryCount : Nat
  degradedSteadyRecoveryCount : Nat
  degradedToBootstrapCount : Nat
  degradedSteadyRoundCount : Nat
  serviceRetentionCarryForwardCount : Nat
  asymmetricShiftSuccessCount : Nat
  checkpointCaptureCount : Nat
  checkpointRestoreCount : Nat
  continuationShiftCount : Nat
  corridorNarrowCount : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

structure RustReplayFixture where
  scenario : String
  search : Option RustReplaySearchFixture
  protocol : RustReplayProtocolFixture
  runtime : RustReplayRuntimeLinkageFixture
  recovery : Option RustReplayRecoveryFixture
  deriving Inhabited, Repr, DecidableEq, BEq

/-! ## Concrete Replay Families -/

def exactNodeActivationFixture : RustReplayFixture :=
  { scenario := "exact-node-activation"
    search := some
      { objectiveClass := "Node"
        queryKind := "SingleGoal"
        selectedNeighborPresent := true
        snapshotEpochPresent := true
        planningFailure := none }
    protocol :=
      { reconfigurationCauses := []
        routeBoundReconfigurationCount := 0 }
    runtime :=
      { artifactCount := 3
        searchLinkedArtifactCount := 0
        routeArtifactCount := 3
        bootstrapRouteArtifactCount := 0 }
    recovery := some
      { lastTrigger := none
        lastOutcome := none
        bootstrapActive := false
        continuityBand := none
        lastContinuityTransition := none
        lastBootstrapTransition := none
        lastPromotionDecision := none
        lastPromotionBlocker := none
        bootstrapActivationCount := 0
        bootstrapHoldCount := 0
        bootstrapNarrowCount := 0
        bootstrapUpgradeCount := 0
        bootstrapWithdrawCount := 0
        degradedSteadyEntryCount := 0
        degradedSteadyRecoveryCount := 0
        degradedToBootstrapCount := 0
        degradedSteadyRoundCount := 0
        serviceRetentionCarryForwardCount := 0
        asymmetricShiftSuccessCount := 0
        checkpointCaptureCount := 0
        checkpointRestoreCount := 0
        continuationShiftCount := 0
        corridorNarrowCount := 0 } }

def candidateSetActivationFixture : RustReplayFixture :=
  { scenario := "candidate-set-activation"
    search := some
      { objectiveClass := "Service"
        queryKind := "CandidateSet"
        selectedNeighborPresent := true
        snapshotEpochPresent := true
        planningFailure := none }
    protocol :=
      { reconfigurationCauses := []
        routeBoundReconfigurationCount := 0 }
    runtime :=
      { artifactCount := 3
        searchLinkedArtifactCount := 0
        routeArtifactCount := 3
        bootstrapRouteArtifactCount := 0 }
    recovery := some
      { lastTrigger := none
        lastOutcome := none
        bootstrapActive := false
        continuityBand := none
        lastContinuityTransition := none
        lastBootstrapTransition := none
        lastPromotionDecision := none
        lastPromotionBlocker := none
        bootstrapActivationCount := 0
        bootstrapHoldCount := 0
        bootstrapNarrowCount := 0
        bootstrapUpgradeCount := 0
        bootstrapWithdrawCount := 0
        degradedSteadyEntryCount := 0
        degradedSteadyRecoveryCount := 0
        degradedToBootstrapCount := 0
        degradedSteadyRoundCount := 0
        serviceRetentionCarryForwardCount := 0
        asymmetricShiftSuccessCount := 0
        checkpointCaptureCount := 0
        checkpointRestoreCount := 0
        continuationShiftCount := 0
        corridorNarrowCount := 0 } }

def continuationShiftFixture : RustReplayFixture :=
  { scenario := "continuation-shift"
    search := some
      { objectiveClass := "Node"
        queryKind := "SingleGoal"
        selectedNeighborPresent := true
        snapshotEpochPresent := true
        planningFailure := none }
    protocol :=
      { reconfigurationCauses := ["ContinuationShift"]
        routeBoundReconfigurationCount := 1 }
    runtime :=
      { artifactCount := 0
        searchLinkedArtifactCount := 0
        routeArtifactCount := 0
        bootstrapRouteArtifactCount := 0 }
    recovery := some
      { lastTrigger := some "ContinuationShift"
        lastOutcome := some "ContinuationRetained"
        bootstrapActive := false
        continuityBand := none
        lastContinuityTransition := none
        lastBootstrapTransition := none
        lastPromotionDecision := none
        lastPromotionBlocker := none
        bootstrapActivationCount := 0
        bootstrapHoldCount := 0
        bootstrapNarrowCount := 0
        bootstrapUpgradeCount := 0
        bootstrapWithdrawCount := 0
        degradedSteadyEntryCount := 0
        degradedSteadyRecoveryCount := 0
        degradedToBootstrapCount := 0
        degradedSteadyRoundCount := 0
        serviceRetentionCarryForwardCount := 0
        asymmetricShiftSuccessCount := 0
        checkpointCaptureCount := 0
        checkpointRestoreCount := 0
        continuationShiftCount := 1
        corridorNarrowCount := 0 } }

def checkpointRestoreFixture : RustReplayFixture :=
  { scenario := "checkpoint-restore"
    search := some
      { objectiveClass := "Node"
        queryKind := "SingleGoal"
        selectedNeighborPresent := true
        snapshotEpochPresent := true
        planningFailure := none }
    protocol :=
      { reconfigurationCauses := ["CheckpointRestore"]
        routeBoundReconfigurationCount := 1 }
    runtime :=
      { artifactCount := 3
        searchLinkedArtifactCount := 0
        routeArtifactCount := 3
        bootstrapRouteArtifactCount := 0 }
    recovery := some
      { lastTrigger := some "RestoreRuntime"
        lastOutcome := some "CheckpointRestored"
        bootstrapActive := false
        continuityBand := none
        lastContinuityTransition := none
        lastBootstrapTransition := none
        lastPromotionDecision := none
        lastPromotionBlocker := none
        bootstrapActivationCount := 0
        bootstrapHoldCount := 0
        bootstrapNarrowCount := 0
        bootstrapUpgradeCount := 0
        bootstrapWithdrawCount := 0
        degradedSteadyEntryCount := 0
        degradedSteadyRecoveryCount := 0
        degradedToBootstrapCount := 0
        degradedSteadyRoundCount := 0
        serviceRetentionCarryForwardCount := 0
        asymmetricShiftSuccessCount := 0
        checkpointCaptureCount := 1
        checkpointRestoreCount := 1
        continuationShiftCount := 0
        corridorNarrowCount := 0 } }

/-! ## Fixture Facts -/

theorem exact_node_activation_fixture_uses_single_goal :
    exactNodeActivationFixture.search.map (·.queryKind) = some "SingleGoal" := by
  rfl

theorem candidate_set_activation_fixture_uses_candidate_set :
    candidateSetActivationFixture.search.map (·.queryKind) = some "CandidateSet" := by
  rfl

theorem continuation_shift_fixture_records_route_bound_reconfiguration :
    continuationShiftFixture.protocol.routeBoundReconfigurationCount = 1 := by
  rfl

theorem checkpoint_restore_fixture_records_checkpoint_restore_outcome :
    checkpointRestoreFixture.recovery.bind (fun recovery => recovery.lastOutcome) =
      some "CheckpointRestored" := by
  rfl

theorem exact_node_activation_fixture_has_no_bootstrap_route_artifacts :
    exactNodeActivationFixture.runtime.bootstrapRouteArtifactCount = 0 := by
  rfl

end FieldAdequacyReplayFixtures
