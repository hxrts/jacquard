import Field.Model.Instance
import Field.Network.API
import Field.Router.Admission
import Field.Router.Installation
import Field.Router.Publication

/-! # Network.Safety — network round buffer matches local projections and explicit-path honesty -/

/-
Prove that the synchronous network round buffer is consistent with each node's local projection,
that support mass is conserved across a network step, and that explicit-path route claims require
explicit local knowledge.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldNetworkSafety

open FieldModelAPI
open FieldNetworkAPI
open FieldRouterAdmission
open FieldRouterInstallation
open FieldRouterPublication

/-! ## Projection Consistency -/

theorem network_round_buffer_matches_local_projection
    (state : NetworkState)
    (node : NodeId)
    (destination : DestinationClass) :
    (networkRound state).roundBuffer node destination =
      publishMessage node destination (state.localStates node destination) := by
  rfl

/-! ## Support Conservation -/

theorem network_round_buffer_support_conservative
    (state : NetworkState)
    (hHarmony : NetworkLocallyHarmonious state)
    (node : NodeId)
    (destination : DestinationClass) :
    ((networkRound state).roundBuffer node destination).projection.support ≤
      (state.localStates node destination).posterior.support := by
  have hLocal := hHarmony node destination
  rcases hLocal with ⟨_, _, _, hSupport, _⟩
  simpa [networkRound, initializeRoundBuffer, publishMessage] using hSupport

/-! ## Explicit-Path Honesty -/

theorem explicit_path_installation_requires_explicit_local_knowledge
    (state : NetworkState)
    (hHarmony : NetworkLocallyHarmonious state)
    (node : NodeId)
    (destination : DestinationClass)
    (admitted : AdmittedCandidate)
    (hAdmitted :
      admitPublishedCandidate node destination (state.localStates node destination) =
        some admitted)
    (hShape :
      (installCandidate admitted).shape = CorridorShape.explicitPath) :
    (state.localStates node destination).posterior.knowledge =
      ReachabilityKnowledge.explicitPath := by
  have hMatch :=
    admitted_candidate_matches_publication node destination
      (state.localStates node destination) admitted hAdmitted
  have hPubShape :
      admitted.candidate.shape = CorridorShape.explicitPath := by
    simpa [installCandidate] using hShape
  have hLocalShape :
      (state.localStates node destination).projection.shape = CorridorShape.explicitPath := by
    exact hMatch.1.1.symm.trans hPubShape
  exact
    explicit_path_publication_requires_explicit_knowledge
      (state.localStates node destination)
      (hHarmony node destination)
      (by simpa [publishCandidate] using hLocalShape)

theorem network_installation_never_exceeds_local_support
    (state : NetworkState)
    (hHarmony : NetworkLocallyHarmonious state)
    (node : NodeId)
    (destination : DestinationClass)
    (admitted : AdmittedCandidate)
    (hAdmitted :
      admitPublishedCandidate node destination (state.localStates node destination) =
        some admitted) :
    (installCandidate admitted).support ≤
      (state.localStates node destination).posterior.support := by
  have hEq :=
    admitted_candidate_matches_publication node destination
      (state.localStates node destination) admitted hAdmitted
  have hSupport :=
    publication_support_le_local_support
      (state.localStates node destination)
      (hHarmony node destination)
  have hInstalled :
      (installCandidate admitted).support =
        (publishCandidate node destination (state.localStates node destination)).support := by
    simpa [installCandidate] using congrArg PublishedCandidate.support hEq.2
  exact hInstalled.trans_le hSupport

end FieldNetworkSafety
