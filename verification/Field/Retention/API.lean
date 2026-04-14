import Mathlib.Data.List.Basic
import Mathlib.Tactic.DeriveFintype
import Field.Router.Canonical

/-
The Problem. Field needs a reduced payload-retention layer that can express
deferred custody without collapsing the destination-local observer/controller
model into a payload-buffer model.

Solution Structure.
1. Define a reduced payload-token and retention-policy vocabulary.
2. Define a small abstract retention interface and a controlled view that keeps
   local state, routes, and retention state separate.
3. Package the boundary laws needed by later refinement/system proofs.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRetentionAPI

open FieldModelAPI
open FieldNetworkAPI
open FieldRouterLifecycle
open FieldRouterCanonical

/-! ## Reduced Time Vocabulary -/

structure Tick where
  value : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

structure DurationMs where
  value : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

structure RouteEpoch where
  value : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def Tick.elapsedSince (nowTick earlierTick : Tick) : DurationMs :=
  ⟨nowTick.value - earlierTick.value⟩

def DurationMs.within (elapsed limit : DurationMs) : Prop :=
  elapsed.value ≤ limit.value

/-! ## Reduced Retention Vocabulary -/

inductive SizeClass
  | small
  | medium
  | large
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

inductive AgeClass
  | fresh
  | warm
  | stale
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

inductive CustodyClass
  | localOnly
  | forwardedCopy
  | finalDeliveryPending
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

inductive RetentionDecision
  | retain
  | carry
  | forward
  | drop
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

inductive SupportBand
  | low
  | medium
  | high
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

inductive UncertaintyBand
  | stable
  | risky
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

inductive ContinuityBand
  | steady
  | degradedSteady
  | bootstrap
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

structure PayloadToken where
  messageId : Nat
  destination : DestinationClass
  sizeClass : SizeClass
  ageClass : AgeClass
  custodyClass : CustodyClass
  retainedAtTick : Tick
  lastRetryTick : Tick
  admittedRouteEpoch : RouteEpoch
  deriving Inhabited, Repr, DecidableEq, BEq

structure RetentionPolicyInput where
  regime : OperatingRegime
  posture : RoutingPosture
  supportBand : SupportBand
  uncertaintyBand : UncertaintyBand
  continuity : ContinuityBand
  continuationAvailable : Bool
  routeInstalled : Bool
  nowTick : Tick
  activeRouteEpoch : RouteEpoch
  custodyTimeout : DurationMs
  deriving Inhabited, Repr, DecidableEq, BEq

structure RetentionState where
  buffer : List PayloadToken
  retainedCount : Nat
  deliveredCount : Nat
  droppedCount : Nat
  lastDecision : Option RetentionDecision
  deriving Inhabited, Repr, DecidableEq, BEq

structure RetentionStep where
  before : RetentionState
  after : RetentionState
  token : Option PayloadToken
  decision : RetentionDecision
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Execution-facing view that keeps local field state, router lifecycle input,
and retention state separate so later theorems can state that retention
transitions do not change route truth by themselves. -/
structure RetentionControlledView where
  localState : LocalState
  routes : List LifecycleRoute
  retention : RetentionState

def RetentionState.coherent (state : RetentionState) : Prop :=
  state.retainedCount = state.buffer.length

def RetentionState.accountedCount (state : RetentionState) : Nat :=
  state.buffer.length + state.deliveredCount + state.droppedCount

def retentionBufferIds (state : RetentionState) : List Nat :=
  state.buffer.map PayloadToken.messageId

def PayloadToken.custodyAge
    (token : PayloadToken)
    (nowTick : Tick) : DurationMs :=
  Tick.elapsedSince nowTick token.retainedAtTick

def PayloadToken.retryAge
    (token : PayloadToken)
    (nowTick : Tick) : DurationMs :=
  Tick.elapsedSince nowTick token.lastRetryTick

def RetentionPolicyInput.routeEpochFreshFor
    (input : RetentionPolicyInput)
    (token : PayloadToken) : Prop :=
  token.admittedRouteEpoch = input.activeRouteEpoch

def RetentionPolicyInput.custodyFreshFor
    (input : RetentionPolicyInput)
    (token : PayloadToken) : Prop :=
  DurationMs.within (token.custodyAge input.nowTick) input.custodyTimeout

def RetentionPolicyInput.currentAuthorityFor
    (input : RetentionPolicyInput)
    (token : PayloadToken) : Prop :=
  input.routeInstalled = true ∧
    input.continuationAvailable = true ∧
    input.continuity = .steady ∧
    input.routeEpochFreshFor token ∧
    input.custodyFreshFor token

/-- Small abstract interface for reduced retention behavior. -/
structure RetentionInterface where
  selectRetentionDecision : RetentionPolicyInput → PayloadToken → RetentionDecision
  retentionStep : RetentionPolicyInput → RetentionState → RetentionState
  injectPayload : PayloadToken → RetentionState → RetentionState
  restoreRetentionState : RetentionState → RetentionState

def stepControlledView
    (interface : RetentionInterface)
    (input : RetentionPolicyInput)
    (view : RetentionControlledView) : RetentionControlledView :=
  { view with retention := interface.retentionStep input view.retention }

def injectControlledView
    (interface : RetentionInterface)
    (token : PayloadToken)
    (view : RetentionControlledView) : RetentionControlledView :=
  { view with retention := interface.injectPayload token view.retention }

def restoreControlledView
    (interface : RetentionInterface)
    (view : RetentionControlledView) : RetentionControlledView :=
  { view with retention := interface.restoreRetentionState view.retention }

/-! ## Boundary Law Bundles -/

structure RetentionBoundednessLaws
    (interface : RetentionInterface) where
  capacity : Nat
  step_coherent :
    ∀ input state,
      state.coherent →
        (interface.retentionStep input state).coherent
  inject_coherent :
    ∀ token state,
      state.coherent →
        (interface.injectPayload token state).coherent
  restore_coherent :
    ∀ state,
      state.coherent →
        (interface.restoreRetentionState state).coherent
  step_bounded :
    ∀ input state,
      state.coherent →
        (interface.retentionStep input state).buffer.length ≤ capacity
  inject_bounded :
    ∀ token state,
      state.coherent →
        (interface.injectPayload token state).buffer.length ≤ capacity
  restore_bounded :
    ∀ state,
      state.coherent →
        (interface.restoreRetentionState state).buffer.length ≤ capacity

structure ExplicitDropDeliveryLaws
    (interface : RetentionInterface) where
  dropped_count_changes_are_explicit :
    ∀ input state,
      (interface.retentionStep input state).droppedCount ≠ state.droppedCount →
        (interface.retentionStep input state).lastDecision = some .drop
  delivered_count_changes_are_explicit :
    ∀ input state,
      (interface.retentionStep input state).deliveredCount ≠ state.deliveredCount →
        (interface.retentionStep input state).lastDecision = some .forward

structure NoCreationFromSilenceLaws
    (interface : RetentionInterface) where
  step_preserves_message_origin :
    ∀ input state token,
      token ∈ (interface.retentionStep input state).buffer →
        ∃ prior ∈ state.buffer, prior.messageId = token.messageId
  restore_preserves_message_origin :
    ∀ state token,
      token ∈ (interface.restoreRetentionState state).buffer →
        ∃ prior ∈ state.buffer, prior.messageId = token.messageId

structure RouteTruthSeparationLaws
    (interface : RetentionInterface) where
  local_state_unchanged :
    ∀ input view,
      (stepControlledView interface input view).localState = view.localState
  routes_unchanged :
    ∀ input view,
      (stepControlledView interface input view).routes = view.routes
  canonical_truth_unchanged :
    ∀ input view destination,
      canonicalBestRoute destination (stepControlledView interface input view).routes =
        canonicalBestRoute destination view.routes

theorem step_controlled_view_preserves_local_state
    (interface : RetentionInterface)
    (input : RetentionPolicyInput)
    (view : RetentionControlledView) :
    (stepControlledView interface input view).localState = view.localState := by
  rfl

theorem step_controlled_view_preserves_routes
    (interface : RetentionInterface)
    (input : RetentionPolicyInput)
    (view : RetentionControlledView) :
    (stepControlledView interface input view).routes = view.routes := by
  rfl

theorem inject_controlled_view_preserves_local_state
    (interface : RetentionInterface)
    (token : PayloadToken)
    (view : RetentionControlledView) :
    (injectControlledView interface token view).localState = view.localState := by
  rfl

theorem restore_controlled_view_preserves_routes
    (interface : RetentionInterface)
    (view : RetentionControlledView) :
    (restoreControlledView interface view).routes = view.routes := by
  rfl

end FieldRetentionAPI
