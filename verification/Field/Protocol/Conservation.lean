import Runtime.Proofs.Conservation.Authority
import Runtime.Proofs.Conservation.Evidence
import Runtime.Proofs.ObserverProjection
import Field.Model.Boundary
import Field.Protocol.Bridge

/-!
Field-side theorem pack aligning the reduced protocol boundary with Telltale's
conservation and observer-projection vocabulary.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldProtocolConservation

open FieldBoundary
open FieldModelAPI
open FieldProtocolAPI
open FieldProtocolBridge

/-- Field-side evidence conservation: controller-visible evidence stays on the
observational side of the boundary for the whole trace. -/
abbrev FieldEvidenceConservation (trace : ProtocolTrace) : Prop :=
  ∀ evidence ∈ controllerEvidenceFromTrace trace,
    evidence.reachability = ReachabilitySignal.unknown ∨
      evidence.reachability = ReachabilitySignal.corridorOnly

/-- Field-side authority conservation over exported machine snapshots:
replay-visible semantic objects preserve observational-only authority. -/
abbrev SnapshotAuthorityConservation (snapshot : MachineSnapshot) : Prop :=
  ∀ object ∈ FieldProtocolAPI.exportSemanticObjects snapshot,
    object.authority = OutputAuthority.observationalOnly

/-- Fragment-trace authority conservation: erased protocol-machine fragments do
not gain stronger authority than observational export. -/
abbrev FragmentAuthorityConservation (trace : FragmentTrace) : Prop :=
  ∀ object ∈ fragmentTraceSemanticObjects trace,
    object.authority = OutputAuthority.observationalOnly

theorem protocol_trace_evidence_conserved
    (trace : ProtocolTrace) :
    FieldEvidenceConservation trace :=
  FieldBoundary.trace_controller_evidence_stays_observational trace

theorem snapshot_authority_conserved
    (snapshot : MachineSnapshot) :
    SnapshotAuthorityConservation snapshot :=
  FieldBoundary.semantic_objects_from_snapshot_stay_observational snapshot

theorem fragment_trace_authority_conserved
    (trace : FragmentTrace) :
    FragmentAuthorityConservation trace :=
  FieldProtocolBridge.fragment_semantic_objects_stay_observational trace

theorem replay_equivalent_fragment_traces_preserve_field_evidence
    {left right : FragmentTrace}
    (hEq : fragmentTraceSemanticObjects left = fragmentTraceSemanticObjects right) :
    semanticObjectsToEvidence (fragmentTraceSemanticObjects left) =
      semanticObjectsToEvidence (fragmentTraceSemanticObjects right) :=
  FieldProtocolBridge.replay_equivalent_fragment_traces_induce_equal_controller_evidence hEq

end FieldProtocolConservation
