import Field.Network.API
import Field.Router.Publication

/-! # Router.Admission — candidate admission statuses and decision logic -/

/-
Define admission status kinds and the decision function that maps a route candidate to an
admission outcome, ensuring admitted candidates satisfy honesty and well-formedness.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterAdmission

open FieldModelAPI
open FieldNetworkAPI
open FieldRouterPublication

/-! ## Admission Statuses -/

inductive AdmissionStatus
  | observed
  | admitted
  | rejected
  deriving Inhabited, Repr, DecidableEq, BEq

def observeCandidate
    (_candidate : PublishedCandidate) : AdmissionStatus :=
  .observed

def CandidateAdmissible
    (localState : LocalState)
    (candidate : PublishedCandidate) : Prop :=
  PublicationHonest localState candidate ∧
    PublicationWellFormed candidate ∧
    candidate.support ≠ 0

instance instDecidableCandidateAdmissible
    (localState : LocalState)
    (candidate : PublishedCandidate) :
    Decidable (CandidateAdmissible localState candidate) := by
  unfold CandidateAdmissible PublicationHonest PublicationWellFormed
  infer_instance

/-! ## Decision Logic -/

def decideAdmission
    (localState : LocalState)
    (candidate : PublishedCandidate) : AdmissionStatus :=
  if CandidateAdmissible localState candidate then
    .admitted
  else
    .rejected

structure AdmittedCandidate where
  localState : LocalState
  candidate : PublishedCandidate
  admissible : CandidateAdmissible localState candidate

def admitPublishedCandidate
    (publisher : NodeId)
    (destination : DestinationClass)
    (localState : LocalState) : Option AdmittedCandidate :=
  let candidate := publishCandidate publisher destination localState
  if h : CandidateAdmissible localState candidate then
    some { localState := localState, candidate := candidate, admissible := h }
  else
    none

/-! ## Honesty Constraints -/

theorem admitted_candidates_satisfy_publication_honesty
    (admitted : AdmittedCandidate) :
    PublicationHonest admitted.localState admitted.candidate :=
  admitted.admissible.1

theorem admission_cannot_strengthen_claim
    (admitted : AdmittedCandidate) :
    admitted.candidate.shape = admitted.localState.projection.shape ∧
      admitted.candidate.support = admitted.localState.projection.support := by
  exact ⟨admitted.admissible.1.1, admitted.admissible.1.2.1⟩

theorem admitted_candidate_matches_publication
    (publisher : NodeId)
    (destination : DestinationClass)
    (localState : LocalState)
    (admitted : AdmittedCandidate)
    (h :
      admitPublishedCandidate publisher destination localState = some admitted) :
    PublicationHonest localState admitted.candidate ∧
      admitted.candidate = publishCandidate publisher destination localState := by
  unfold admitPublishedCandidate at h
  by_cases hAdm :
      CandidateAdmissible localState
        (publishCandidate publisher destination localState)
  · simp [hAdm] at h
    rcases h with rfl
    constructor
    · exact publishCandidate_honest publisher destination localState
    · rfl
  · simp [hAdm] at h

end FieldRouterAdmission
