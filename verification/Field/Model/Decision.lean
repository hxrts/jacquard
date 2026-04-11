import Field.Model.Instance

/-!
One small decision layer over the bounded local field model.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldModelDecision

open FieldModelAPI
open FieldModelInstance

/-- Small evidence alphabet used by the first finite exploration pass. -/
def representativeEvidenceAlphabet : List EvidenceInput :=
  [unknownEvidence, explicitPathEvidence, corridorRiskEvidence]

/-- One-step explored graph for a bounded local state under a finite evidence
alphabet. -/
structure OneStepDecisionGraph where
  root : LocalState
  alphabet : List EvidenceInput
  successors : List LocalState
  deriving Repr

/-- Explore one local round for every evidence object in the finite alphabet. -/
def exploreOneStepGraph
    (root : LocalState)
    (alphabet : List EvidenceInput) : OneStepDecisionGraph :=
  { root := root
    alphabet := alphabet
    successors := alphabet.map (roundStepImpl · root) }

/-- Semantic one-round reachability question decided by the bounded explorer. -/
def ExplicitPathReachableInOneRound
    (root : LocalState)
    (alphabet : List EvidenceInput) : Prop :=
  ∃ evidence ∈ alphabet,
    (FieldModelAPI.roundStep evidence root).projection.shape =
      CorridorShape.explicitPath

/-- Decision question: can explicit-path publication occur in one round under
the chosen finite evidence alphabet? -/
noncomputable def admitsExplicitPathInOneRound
    (root : LocalState)
    (alphabet : List EvidenceInput) : Bool :=
  by
    classical
    exact decide (ExplicitPathReachableInOneRound root alphabet)

theorem explore_one_step_graph_exact
    (root : LocalState)
    (alphabet : List EvidenceInput) :
    (exploreOneStepGraph root alphabet).successors =
      alphabet.map (roundStepImpl · root) := by
  rfl

theorem explicit_path_decider_sound
    (root : LocalState)
    (alphabet : List EvidenceInput)
    (hDecision : admitsExplicitPathInOneRound root alphabet = true) :
    ExplicitPathReachableInOneRound root alphabet := by
  simpa [admitsExplicitPathInOneRound] using hDecision

theorem explicit_path_decider_complete
    (root : LocalState)
    (alphabet : List EvidenceInput)
    (hReachable : ExplicitPathReachableInOneRound root alphabet) :
    admitsExplicitPathInOneRound root alphabet = true := by
  simpa [admitsExplicitPathInOneRound] using hReachable

/-- Concrete one-step decidability witness for the current representative
alphabet. -/
theorem representative_alphabet_decides_explicit_path_from_initial :
    admitsExplicitPathInOneRound initialState representativeEvidenceAlphabet = true := by
  apply explicit_path_decider_complete
  exact ⟨explicitPathEvidence, by simp [representativeEvidenceAlphabet],
    explicit_path_signal_yields_explicit_projection⟩

end FieldModelDecision
