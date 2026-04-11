import Field.Network.API

/-
The Problem. The field model needs one reduced asynchronous transport layer
above synchronous publication so later proofs can talk about delay, retry, and
loss without jumping directly to the full production transport stack.

Solution Structure.
1. Define bounded async transport assumptions and envelope/state vocabulary.
2. Define one reduced step relation over in-flight envelopes.
3. Expose a small observer view that later system proofs can consume.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAsyncAPI

open FieldModelAPI
open FieldNetworkAPI

/-! ## Async Envelope Surface -/

structure AsyncAssumptions where
  maxDelay : Nat
  retryBound : Nat
  lossPossible : Prop
  batchBound : Nat

def reliableImmediateAssumptions : AsyncAssumptions :=
  { maxDelay := 0
    retryBound := 0
    lossPossible := False
    batchBound := allNodes.length * allDestinations.length }

structure AsyncEnvelope where
  sender : NodeId
  receiver : NodeId
  destination : DestinationClass
  projection : CorridorEnvelopeProjection
  delay : Nat
  retryCount : Nat
  dropped : Bool
  deriving Repr, DecidableEq, BEq

/-- Execution-state family note: `AsyncState` is an execution object, not a
publication or lifecycle object. It owns in-flight transport state above the
synchronous network but below end-to-end lifecycle composition. -/
structure AsyncState where
  network : NetworkState
  assumptions : AsyncAssumptions
  inFlight : List AsyncEnvelope
  tick : Nat

def publicationEnvelope
    (network : NetworkState)
    (assumptions : AsyncAssumptions)
    (sender receiver : NodeId)
    (destination : DestinationClass) : AsyncEnvelope :=
  { sender := sender
    receiver := receiver
    destination := destination
    projection := (publishMessage sender destination (network.localStates sender destination)).projection
    delay := assumptions.maxDelay
    retryCount := 0
    dropped := False }

def readyForDelivery (envelope : AsyncEnvelope) : Bool :=
  envelope.delay = 0 && !envelope.dropped

def stepEnvelope (envelope : AsyncEnvelope) : AsyncEnvelope :=
  if envelope.delay = 0 then
    envelope
  else
    { envelope with delay := envelope.delay - 1 }

def eligibleForRetry
    (assumptions : AsyncAssumptions)
    (envelope : AsyncEnvelope) : Bool :=
  envelope.dropped && envelope.retryCount < assumptions.retryBound

def retryEnvelope
    (assumptions : AsyncAssumptions)
    (envelope : AsyncEnvelope) : AsyncEnvelope :=
  if eligibleForRetry assumptions envelope then
    { envelope with
        dropped := False
        delay := assumptions.maxDelay
        retryCount := envelope.retryCount + 1 }
  else
    envelope

def lifecycleEnvelope
    (assumptions : AsyncAssumptions)
    (envelope : AsyncEnvelope) : AsyncEnvelope :=
  retryEnvelope assumptions (stepEnvelope envelope)

def enqueuePublications
    (network : NetworkState)
    (assumptions : AsyncAssumptions) : List AsyncEnvelope :=
  (allNodes.foldr
      (fun sender senderAcc =>
        (allNodes.foldr
            (fun receiver receiverAcc =>
              (allDestinations.filterMap fun destination =>
                  if network.neighbors sender receiver then
                    some (publicationEnvelope network assumptions sender receiver destination)
                  else
                    none) ++ receiverAcc)
            []) ++ senderAcc)
      [])

def injectPublications (state : AsyncState) : List AsyncEnvelope :=
  state.inFlight ++ enqueuePublications state.network state.assumptions

def asyncStep (state : AsyncState) : AsyncState :=
  { network := state.network
    assumptions := state.assumptions
    inFlight := (state.inFlight.map stepEnvelope) ++ enqueuePublications state.network state.assumptions
    tick := state.tick + 1 }

def transportStep (state : AsyncState) : AsyncState :=
  { network := state.network
    assumptions := state.assumptions
    inFlight := (state.inFlight.map (lifecycleEnvelope state.assumptions)) ++
      enqueuePublications state.network state.assumptions
    tick := state.tick + 1 }

def readyMessages
    (state : AsyncState)
    (receiver : NodeId)
    (destination : DestinationClass) : List AsyncEnvelope :=
  state.inFlight.filter fun envelope =>
    envelope.receiver = receiver &&
      envelope.destination = destination &&
      readyForDelivery envelope

def drainReadyMessages (state : AsyncState) : AsyncState :=
  { state with
      inFlight := state.inFlight.filter fun envelope => !(readyForDelivery envelope) }

def droppedMessages (state : AsyncState) : List AsyncEnvelope :=
  state.inFlight.filter fun envelope => envelope.dropped

/-- Small observer view for the reduced async layer. -/
structure AsyncObserverView where
  readyCount : Nat
  inFlightCount : Nat
  deriving Repr, DecidableEq, BEq

def observerView (state : AsyncState) : AsyncObserverView :=
  { readyCount := (state.inFlight.filter readyForDelivery).length
    inFlightCount := state.inFlight.length }

end FieldAsyncAPI
