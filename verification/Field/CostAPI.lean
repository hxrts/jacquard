/-! # Field.CostAPI — shared budget and work-unit vocabulary -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldCostAPI

/-- Small shared work-unit wrapper used across router, system, and adequacy
cost surfaces. -/
structure WorkUnits where
  amount : Nat
  deriving Repr, DecidableEq, BEq

/-- Shared budget wrapper. This keeps cost-preservation theorems talking about
one vocabulary even when the underlying metric is still a simple natural. -/
structure WorkBudget where
  units : WorkUnits
  deriving Repr, DecidableEq, BEq

def WorkUnits.le (left right : WorkUnits) : Prop :=
  left.amount ≤ right.amount

def WorkBudget.allows
    (budget : WorkBudget)
    (work : WorkUnits) : Prop :=
  WorkUnits.le work budget.units

def WorkUnits.ofNat (amount : Nat) : WorkUnits :=
  { amount := amount }

def WorkBudget.ofNat (amount : Nat) : WorkBudget :=
  { units := WorkUnits.ofNat amount }

theorem workBudget_allows_of_le
    (used available : Nat)
    (hLe : used ≤ available) :
    (WorkBudget.ofNat available).allows (WorkUnits.ofNat used) := by
  exact hLe

end FieldCostAPI
