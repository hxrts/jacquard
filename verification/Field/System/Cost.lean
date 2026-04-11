import Field.Router.Cost
import Field.System.Bounded
import Field.System.Canonical

/-! # System.Cost — communication, queue, and storage work accounting with bottleneck analysis -/

/-
Define system-level work unit accounting across communication, queue, and storage dimensions
and prove bounded growth and bottleneck localisation theorems.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemCost

open FieldAsyncAPI
open FieldAsyncBounded
open FieldRouterCanonical
open FieldRouterCost
open FieldRouterLifecycle
open FieldQualitySystem
open FieldSystemBounded
open FieldSystemCanonical
open FieldSystemConvergence
open FieldSystemEndToEnd

/-! ## Work Unit Accounting -/

def communicationWorkUnits
    (state : EndToEndState) : Nat :=
  (enqueuePublications state.async.network state.async.assumptions).length

def queueWorkUnits
    (state : EndToEndState) : Nat :=
  state.async.inFlight.length

def storageWorkUnits
    (state : EndToEndState) : Nat :=
  state.async.inFlight.length + state.lifecycle.length

def nextStorageWorkUnits
    (state : EndToEndState) : Nat :=
  (systemStep state).async.inFlight.length + (systemStep state).lifecycle.length

def transportVolumeBudget
    (state : EndToEndState) : Nat :=
  queueWorkUnits state + communicationWorkUnits state

def transportWorkBottleneck
    (state : EndToEndState) : Nat :=
  max (queueWorkUnits state) (communicationWorkUnits state)

theorem communication_work_units_explicit
    (state : EndToEndState) :
    communicationWorkUnits state =
      (enqueuePublications state.async.network state.async.assumptions).length := by
  rfl

theorem queue_work_units_bounded_by_transport_volume
    (state : EndToEndState) :
    (systemStep state).async.inFlight.length ≤
      transportVolumeBudget state := by
  simpa [queueWorkUnits, communicationWorkUnits, transportVolumeBudget] using
    systemStep_inflight_length_bounded_by_current_plus_publications state

theorem storage_work_units_bounded_by_transport_volume
    (state : EndToEndState) :
    nextStorageWorkUnits state ≤
      2 * transportVolumeBudget state := by
  have hQueue :
      (systemStep state).async.inFlight.length ≤
        transportVolumeBudget state := by
    exact queue_work_units_bounded_by_transport_volume state
  have hLifecycle :
      (systemStep state).lifecycle.length ≤
        transportVolumeBudget state := by
    calc
      (systemStep state).lifecycle.length ≤
          ((transportStep state.async).inFlight.filter readyForDelivery).length :=
        systemStep_lifecycle_length_bounded_by_transport_ready_queue state
      _ = (observerView (transportStep state.async)).readyCount := by
            rfl
      _ ≤ state.async.inFlight.length +
            (enqueuePublications state.async.network state.async.assumptions).length :=
        transportStep_ready_count_bounded_by_current_plus_publications state.async
  have hSum :
      (systemStep state).async.inFlight.length + (systemStep state).lifecycle.length ≤
        transportVolumeBudget state + transportVolumeBudget state :=
    Nat.add_le_add hQueue hLifecycle
  simpa [nextStorageWorkUnits, transportVolumeBudget, two_mul, Nat.mul_add,
    Nat.add_assoc, Nat.add_left_comm, Nat.add_comm]
    using hSum

theorem compute_work_units_bounded_by_transport_volume
    (state : EndToEndState) :
    systemStepWorkUnits state ≤
      4 * transportVolumeBudget state := by
  simpa [queueWorkUnits, communicationWorkUnits, transportVolumeBudget] using
    system_step_work_units_bounded_by_transport_volume state

theorem explicit_transport_volume_budget_preserves_next_state
    (state : EndToEndState) :
    (systemStep state).async.inFlight.length ≤
        transportVolumeBudget state
      ∧ nextStorageWorkUnits state ≤
          2 * transportVolumeBudget state
      ∧ systemStepWorkUnits state ≤
          4 * transportVolumeBudget state := by
  constructor
  · exact queue_work_units_bounded_by_transport_volume state
  constructor
  · exact storage_work_units_bounded_by_transport_volume state
  · exact compute_work_units_bounded_by_transport_volume state

theorem maintenance_work_units_amortized_under_reliable_immediate_empty
    (n : Nat)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    maintenanceWorkUnits (iterateLifecycleMaintenance (n + 1) (systemStep state).lifecycle) =
      maintenanceWorkUnits (systemStep state).lifecycle := by
  simpa using
    maintenance_work_units_invariant_under_iteration
      (n + 1) ((systemStep state).lifecycle)

theorem per_destination_storage_bounded_by_system_lifecycle
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState) :
    (canonicalEligibleRoutes destination (systemStep state).lifecycle).length ≤
      (systemStep state).lifecycle.length := by
  exact
    canonicalEligibleRoutes_search_space_bounded
      destination (systemStep state).lifecycle

theorem communication_work_units_stable_under_reliable_immediate_empty
    (n : Nat)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    communicationWorkUnits (iterateSystemStep (n + 1) state) =
      communicationWorkUnits state := by
  induction n generalizing state with
  | zero =>
      have hPres :=
        system_step_preserves_reliable_immediate_empty_queue state hAssumptions hEmpty
      simp [iterateSystemStep, communicationWorkUnits, system_step_preserves_network,
        hAssumptions, hPres.1]
  | succ n ih =>
      have hPres :=
        system_step_preserves_reliable_immediate_empty_queue state hAssumptions hEmpty
      calc
        communicationWorkUnits (iterateSystemStep (Nat.succ n + 1) state) =
          communicationWorkUnits (systemStep state) := by
            simpa [iterateSystemStep, Nat.add_assoc, Nat.add_comm, Nat.add_left_comm] using
              ih (systemStep state) hPres.1 hPres.2
        _ = communicationWorkUnits state := by
              simp [communicationWorkUnits, system_step_preserves_network,
                hAssumptions, hPres.1]

theorem transport_volume_budget_stable_under_reliable_immediate_empty
    (n : Nat)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    transportVolumeBudget (iterateSystemStep (n + 1) state) =
      communicationWorkUnits state := by
  have hIter :=
    iterateSystemStep_preserves_reliable_immediate_empty_queue (n + 1) state hAssumptions hEmpty
  calc
    transportVolumeBudget (iterateSystemStep (n + 1) state) =
      queueWorkUnits (iterateSystemStep (n + 1) state) +
        communicationWorkUnits (iterateSystemStep (n + 1) state) := by
          rfl
    _ = 0 + communicationWorkUnits (iterateSystemStep (n + 1) state) := by
          simp [queueWorkUnits, hIter.2]
    _ = communicationWorkUnits state := by
          rw [communication_work_units_stable_under_reliable_immediate_empty
            n state hAssumptions hEmpty]
          simp

theorem transport_volume_budget_dominates_system_step_work
    (state : EndToEndState) :
    systemStepWorkUnits state ≤ 4 * transportVolumeBudget state := by
  exact compute_work_units_bounded_by_transport_volume state

theorem system_step_work_is_local_to_transport_volume
    (state : EndToEndState) :
    systemStepWorkUnits state ≤ 4 * transportVolumeBudget state := by
  exact transport_volume_budget_dominates_system_step_work state

theorem system_step_work_scales_linearly_with_transport_volume
    (state : EndToEndState) :
    systemStepWorkUnits state ≤ 4 * transportVolumeBudget state := by
  exact system_step_work_is_local_to_transport_volume state

/-! ## Bottleneck Analysis -/

theorem transport_volume_budget_bounded_by_bottleneck
    (state : EndToEndState) :
    transportVolumeBudget state ≤ 2 * transportWorkBottleneck state := by
  unfold transportVolumeBudget transportWorkBottleneck queueWorkUnits communicationWorkUnits
  omega

theorem system_step_work_bottlenecked_by_max_queue_or_communication
    (state : EndToEndState) :
    systemStepWorkUnits state ≤ 8 * transportWorkBottleneck state := by
  have hWork :
      systemStepWorkUnits state ≤ 4 * transportVolumeBudget state :=
    system_step_work_is_local_to_transport_volume state
  have hBudget :
      transportVolumeBudget state ≤ 2 * transportWorkBottleneck state :=
    transport_volume_budget_bounded_by_bottleneck state
  calc
    systemStepWorkUnits state ≤ 4 * transportVolumeBudget state := hWork
    _ ≤ 4 * (2 * transportWorkBottleneck state) := by
          exact Nat.mul_le_mul_left 4 hBudget
    _ = 8 * transportWorkBottleneck state := by omega

theorem resource_pressure_does_not_strengthen_claims
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ (systemStep state).lifecycle) :
    ∃ envelope,
      envelope ∈ (transportStep state.async).inFlight.filter readyForDelivery ∧
        route.candidate.shape = envelope.projection.shape ∧
        route.candidate.support = envelope.projection.support := by
  exact system_step_overload_monotone_degradation state route hMem

theorem resource_pressure_gracefully_degrades_to_transport_derived_claims
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ (systemStep state).lifecycle) :
    ∃ envelope,
      envelope ∈ (transportStep state.async).inFlight.filter readyForDelivery ∧
        route.candidate.shape = envelope.projection.shape ∧
        route.candidate.support = envelope.projection.support := by
  exact resource_pressure_does_not_strengthen_claims state route hMem

end FieldSystemCost
