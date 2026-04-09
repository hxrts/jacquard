import Mathlib.Tactic

/-! # Hello Jacquard

Smoke-test that Mathlib is correctly wired. If this file elaborates without
errors, the prebuilt olean cache is hydrated and the verification scaffold is
ready for real proofs.
-/

-- A trivial arithmetic identity — requires nothing beyond Mathlib.Tactic.
example : 1 + 1 = 2 := by norm_num

-- Natural number ordering is decidable.
example : 3 ≤ 5 := by norm_num

-- Jacquard uses permille fractions (0–1000). Verify the basic bound.
-- All RatioPermille values satisfy v ≤ 1000.
example (v : ℕ) (h : v ≤ 1000) : v * v ≤ 1000 * 1000 := by
  exact Nat.mul_le_mul h h

#check Nat.le_refl
