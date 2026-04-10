import Field.Async.Safety

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemStatistics

open FieldAsyncAPI
open FieldModelAPI
open FieldNetworkAPI

def aggregateSupport
    (network : NetworkState)
    (destination : DestinationClass) : Nat :=
  allNodes.foldl
    (fun acc node => acc + (network.localStates node destination).posterior.support)
    0

def averageSupport
    (network : NetworkState)
    (destination : DestinationClass) : Nat :=
  aggregateSupport network destination / allNodes.length

def readySupportMass
    (state : AsyncState)
    (receiver : NodeId)
    (destination : DestinationClass) : Nat :=
  (readyMessages state receiver destination).foldl
    (fun acc envelope => acc + envelope.projection.support)
    0

theorem aggregateSupport_bounded
    (network : NetworkState)
    (destination : DestinationClass) :
    aggregateSupport network destination ≤ allNodes.length * 1000 := by
  unfold aggregateSupport
  simp [FieldNetworkAPI.allNodes]
  have hAlpha :
      (network.localStates NodeId.alpha destination).posterior.support ≤ 1000 :=
    Nat.min_le_right _ 1000
  have hBeta :
      (network.localStates NodeId.beta destination).posterior.support ≤ 1000 :=
    Nat.min_le_right _ 1000
  have hGamma :
      (network.localStates NodeId.gamma destination).posterior.support ≤ 1000 :=
    Nat.min_le_right _ 1000
  omega

theorem averageSupport_bounded
    (network : NetworkState)
    (destination : DestinationClass) :
    averageSupport network destination ≤ 1000 := by
  unfold averageSupport
  have hAgg := aggregateSupport_bounded network destination
  simpa [FieldNetworkAPI.allNodes, Nat.mul_comm] using Nat.div_le_of_le_mul hAgg

theorem fold_support_mass_bounded_by_length
    (messages : List AsyncEnvelope)
    (hBound : ∀ envelope ∈ messages, envelope.projection.support ≤ 1000) :
    messages.foldl (fun acc envelope => acc + envelope.projection.support) 0 ≤
      messages.length * 1000 := by
  have hAcc :
      ∀ (messages : List AsyncEnvelope) (start : Nat),
        messages.foldl (fun acc envelope => acc + envelope.projection.support) start =
          start + messages.foldl (fun acc envelope => acc + envelope.projection.support) 0 := by
    intro messages start
    induction messages generalizing start with
    | nil =>
        simp
    | cons head tail ih =>
        have hStart := ih (start + head.projection.support)
        have hHead := ih head.projection.support
        calc
          tail.foldl (fun acc envelope => acc + envelope.projection.support)
              (start + head.projection.support)
              =
                (start + head.projection.support) +
                  tail.foldl (fun acc envelope => acc + envelope.projection.support) 0 := hStart
          _ = start +
                (head.projection.support +
                  tail.foldl (fun acc envelope => acc + envelope.projection.support) 0) := by omega
          _ = start +
                List.foldl (fun acc envelope => acc + envelope.projection.support) 0
                  (head :: tail) := by
                    simp [List.foldl, hHead]
  induction messages with
  | nil =>
      simp
  | cons envelope rest ih =>
      simp [List.foldl]
      have hEnvelope : envelope.projection.support ≤ 1000 := hBound envelope (by simp)
      have hRest :
          ∀ envelope' ∈ rest, envelope'.projection.support ≤ 1000 := by
        intro envelope' hMem
        exact hBound envelope' (by simp [hMem])
      have ih' := ih hRest
      calc
        rest.foldl (fun acc envelope => acc + envelope.projection.support) envelope.projection.support
            = envelope.projection.support +
                rest.foldl (fun acc envelope => acc + envelope.projection.support) 0 := by
                  simpa using hAcc rest envelope.projection.support
        _ ≤ 1000 + rest.length * 1000 := Nat.add_le_add hEnvelope ih'
        _ = (rest.length + 1) * 1000 := by omega
        _ = (envelope :: rest).length * 1000 := by simp

theorem ready_support_mass_bounded_by_inflight_budget
    (state : AsyncState)
    (receiver : NodeId)
    (destination : DestinationClass)
    (hBound :
      ∀ envelope ∈ readyMessages (asyncStep state) receiver destination,
        envelope.projection.support ≤ 1000) :
    readySupportMass (asyncStep state) receiver destination ≤
      (observerView (asyncStep state)).inFlightCount * 1000 := by
  unfold readySupportMass observerView
  let messages := readyMessages (asyncStep state) receiver destination
  have hMass :
      messages.foldl (fun acc envelope => acc + envelope.projection.support) 0 ≤
        messages.length * 1000 := by
    apply fold_support_mass_bounded_by_length
    intro envelope hMem
    exact hBound envelope (by simpa [messages] using hMem)
  have hLen : messages.length ≤ (asyncStep state).inFlight.length := by
    unfold messages readyMessages
    exact List.length_filter_le _ _
  have hBudget : messages.length * 1000 ≤ (asyncStep state).inFlight.length * 1000 := by
    exact Nat.mul_le_mul_right 1000 hLen
  exact Nat.le_trans hMass hBudget

end FieldSystemStatistics
