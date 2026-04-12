import Field.Protocol.API
import Field.Protocol.Bridge
import Field.Protocol.Coherence
import Field.Protocol.Conservation
import Field.Protocol.Fixtures
import Field.Protocol.ReceiveRefinement
import Field.Protocol.Reconfiguration
import Field.Model.Boundary
import SessionTypes.Core

/- 
The Problem. The reduced field protocol has several theorem packs, but the
proof stack still needs one explicit closure statement that says what the final
reduced protocol boundary is and which Telltale-family pieces it actually
inherits.

Solution Structure.
1. Bundle the reduced family-alignment facts already proved in the protocol
   API, bridge, and fixtures.
2. Bundle the closed receive-refinement witness surface.
3. Package the fixed-participant and supported-reconfiguration decision as the final
   protocol-boundary statement.
-/

/-!
 # FieldProtocolClosure

Final reduced protocol-boundary closure for Field.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldProtocolClosure

open FieldProtocolAPI
open FieldProtocolBridge
open FieldProtocolFixtures
open FieldProtocolReceiveRefinement
open FieldProtocolReconfiguration
open FieldBoundary
open SessionTypes.Core

/-! ## Final Reduced Boundary -/

def ReducedTelltaleFamilyAlignment : Prop :=
  project controllerRole =
      projectChoreography globalChoreography controllerRole ∧
    project neighborRole = LocalType.dual (project controllerRole) ∧
    ∀ snapshot,
      SnapshotMatchesFragment snapshot (snapshotToFragment snapshot) ∧
        fragmentSemanticObjects (snapshotToFragment snapshot) =
          exportSemanticObjects snapshot

def ReceiveRefinementClosed : Prop :=
  ∀ receive : RefinedReceive,
    ∃ witness : SubtypeReplacementWitness,
      witness.refined = receive

def FinalProtocolBoundary : Prop :=
  FixedParticipantChoreography ∧
    ReconfiguringProtocolBoundary ∧
    ReducedTelltaleFamilyAlignment ∧
    ReceiveRefinementClosed

theorem reduced_telltale_family_alignment_closed :
    ReducedTelltaleFamilyAlignment := by
  constructor
  · exact controller_projection_from_global
  · constructor
    · exact projection_harmony
    · intro snapshot
      exact
        ⟨snapshot_to_fragment_matches snapshot,
          fragment_erasure_preserves_semantic_objects snapshot⟩

theorem receive_refinement_closed :
    ReceiveRefinementClosed := by
  intro receive
  exact refined_receive_has_subtype_replacement_witness receive

theorem final_protocol_boundary_closed :
    FinalProtocolBoundary := by
  refine ⟨reduced_protocol_is_fixed_participant,
    reduced_protocol_boundary_is_reconfiguring_by_design,
    reduced_telltale_family_alignment_closed,
    receive_refinement_closed⟩

theorem summary_exchange_fixture_respects_final_boundary :
    FinalProtocolBoundary := by
  exact final_protocol_boundary_closed

theorem summary_exchange_fixture_has_family_aligned_observer_projection :
    semanticObjectsToEvidence (snapshotTraceSemanticObjects summaryExchangeSnapshots) =
      semanticObjectsToEvidence
        (fragmentTraceSemanticObjects
          (fragmentTraceOfSnapshots summaryExchangeSnapshots)) := by
  exact summary_exchange_observer_projection_matches_fragment_trace

end FieldProtocolClosure
