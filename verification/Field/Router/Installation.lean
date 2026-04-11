import Field.Router.Admission

/-! # Router.Installation — installed route structure and honesty on candidate conversion -/

/-
Define the installed route record shape and prove that converting an admitted candidate to a
control-plane installed object preserves the honesty invariant.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterInstallation

open FieldNetworkAPI
open FieldRouterAdmission

/-! ## Installed Route -/

/-- Minimal canonical installed-route object for the reduced router-facing
control-plane semantics. -/
structure InstalledRoute where
  destination : DestinationClass
  supportingNode : NodeId
  shape : FieldModelAPI.CorridorShape
  support : Nat
  hopLower : Nat
  hopUpper : Nat
  deriving Repr, DecidableEq, BEq

def installCandidate
    (admitted : AdmittedCandidate) : InstalledRoute :=
  { destination := admitted.candidate.destination
    supportingNode := admitted.candidate.publisher
    shape := admitted.candidate.shape
    support := admitted.candidate.support
    hopLower := admitted.candidate.hopLower
    hopUpper := admitted.candidate.hopUpper }

def CanonicalInstallation
    (installed : InstalledRoute) : Prop :=
  ∃ admitted, installCandidate admitted = installed

/-! ## Honesty Preservation -/

theorem installation_preserves_admitted_honesty
    (admitted : AdmittedCandidate) :
    let installed := installCandidate admitted
    installed.destination = admitted.candidate.destination ∧
      installed.supportingNode = admitted.candidate.publisher ∧
      installed.shape = admitted.candidate.shape ∧
      installed.support = admitted.candidate.support := by
  simp [installCandidate]

theorem installation_cannot_occur_without_admitted_candidate
    (installed : InstalledRoute)
    (hInstalled : CanonicalInstallation installed) :
    ∃ admitted, installCandidate admitted = installed :=
  hInstalled

end FieldRouterInstallation
