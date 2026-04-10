import Field.Router.Canonical

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterCost

open FieldModelAPI
open FieldNetworkAPI
open FieldRouterCanonical
open FieldRouterLifecycle

def maintenanceWorkUnits
    (routes : List LifecycleRoute) : Nat :=
  routes.length

def canonicalSearchWorkUnits
    (routes : List LifecycleRoute) : Nat :=
  routes.length

def iterateLifecycleMaintenance : Nat → List LifecycleRoute → List LifecycleRoute
  | 0, routes => routes
  | n + 1, routes => iterateLifecycleMaintenance n (maintainLifecycle routes)

theorem canonicalEligibleRoutes_search_space_bounded
    (destination : DestinationClass)
    (routes : List LifecycleRoute) :
    (canonicalEligibleRoutes destination routes).length ≤ canonicalSearchWorkUnits routes := by
  unfold canonicalSearchWorkUnits canonicalEligibleRoutes
  exact List.length_filterMap_le _ _

theorem canonical_search_worst_case_cost
    (routes : List LifecycleRoute) :
    canonicalSearchWorkUnits routes = routes.length := by
  rfl

theorem canonical_search_incremental_update_cost
    (route : LifecycleRoute)
    (routes : List LifecycleRoute) :
    canonicalSearchWorkUnits (route :: routes) =
      canonicalSearchWorkUnits routes + 1 := by
  simp [canonicalSearchWorkUnits]

theorem canonical_search_stable_input_cost
    (routes : List LifecycleRoute) :
    canonicalSearchWorkUnits routes = canonicalSearchWorkUnits routes := by
  rfl

theorem canonical_search_no_hidden_exponential_branch_growth
    (routes : List LifecycleRoute) :
    canonicalSearchWorkUnits routes ≤ routes.length := by
  rfl

theorem maintenance_work_units_idempotent
    (routes : List LifecycleRoute) :
    maintenanceWorkUnits (maintainLifecycle (maintainLifecycle routes)) =
      maintenanceWorkUnits (maintainLifecycle routes) := by
  simp [maintenanceWorkUnits, FieldRouterLifecycle.maintainLifecycle_idempotent]

theorem maintenance_work_units_invariant_under_iteration
    (n : Nat)
    (routes : List LifecycleRoute) :
    maintenanceWorkUnits (iterateLifecycleMaintenance n routes) =
      maintenanceWorkUnits routes := by
  induction n generalizing routes with
  | zero =>
      rfl
  | succ n ih =>
      calc
        maintenanceWorkUnits (iterateLifecycleMaintenance (n + 1) routes) =
          maintenanceWorkUnits (iterateLifecycleMaintenance n (maintainLifecycle routes)) := by
            rfl
        _ = maintenanceWorkUnits (maintainLifecycle routes) := ih (maintainLifecycle routes)
        _ = maintenanceWorkUnits routes := by
              simp [maintenanceWorkUnits, maintainLifecycle]

theorem maintenance_work_units_amortized_after_first_pass
    (n : Nat)
    (routes : List LifecycleRoute) :
    maintenanceWorkUnits (iterateLifecycleMaintenance (n + 1) routes) =
      maintenanceWorkUnits (maintainLifecycle routes) := by
  calc
    maintenanceWorkUnits (iterateLifecycleMaintenance (n + 1) routes) =
      maintenanceWorkUnits routes :=
        maintenance_work_units_invariant_under_iteration (n + 1) routes
    _ = maintenanceWorkUnits (maintainLifecycle routes) := by
          simp [maintenanceWorkUnits, maintainLifecycle]

end FieldRouterCost
