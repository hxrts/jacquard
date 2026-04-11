import Field.Router.Installation

/-! # Router.Lifecycle — route lifecycle statuses and maintenance transition rules -/

/-
Define the route lifecycle state machine (observed → admitted → installed → withdrawn /
expired / refreshed) and the maintenance rules that govern transitions based on support
level and route shape.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterLifecycle

open FieldModelAPI
open FieldNetworkAPI
open FieldRouterAdmission
open FieldRouterInstallation
open FieldRouterPublication

/-! ## Lifecycle States -/

inductive LifecycleStatus
  | observed
  | admitted
  | installed
  | withdrawn
  | expired
  | refreshed
  deriving Inhabited, Repr, DecidableEq, BEq

structure LifecycleRoute where
  candidate : PublishedCandidate
  status : LifecycleStatus
  deriving Repr, DecidableEq, BEq

def observeCandidateLifecycle
    (candidate : PublishedCandidate) : LifecycleRoute :=
  { candidate := candidate, status := .observed }

def admitCandidateLifecycle
    (admitted : AdmittedCandidate) : LifecycleRoute :=
  { candidate := admitted.candidate, status := .admitted }

def installCandidateLifecycle
    (admitted : AdmittedCandidate) : LifecycleRoute :=
  { candidate := admitted.candidate, status := .installed }

def withdrawLifecycleRoute
    (route : LifecycleRoute) : LifecycleRoute :=
  { route with status := .withdrawn }

def expireLifecycleRoute
    (route : LifecycleRoute) : LifecycleRoute :=
  { route with status := .expired }

def refreshLifecycleRoute
    (candidate : PublishedCandidate)
    (_installed : LifecycleRoute) : LifecycleRoute :=
  { candidate := candidate, status := .refreshed }

/-! ## Maintenance Rules -/

def lifecycleMaintenance
    (route : LifecycleRoute) : LifecycleRoute :=
  if route.candidate.support = 0 then
    expireLifecycleRoute route
  else if route.candidate.shape = CorridorShape.opaque then
    withdrawLifecycleRoute route
  else
    refreshLifecycleRoute route.candidate route

def maintainLifecycle (routes : List LifecycleRoute) : List LifecycleRoute :=
  routes.map lifecycleMaintenance

def LifecycleHonest (route : LifecycleRoute) : Prop :=
  PublicationWellFormed route.candidate

theorem install_lifecycle_traces_to_admitted_candidate
    (admitted : AdmittedCandidate) :
    (installCandidateLifecycle admitted).candidate = admitted.candidate ∧
      (installCandidateLifecycle admitted).status = .installed := by
  simp [installCandidateLifecycle]

theorem lifecycle_transition_preserves_candidate_fields
    (route : LifecycleRoute) :
    (withdrawLifecycleRoute route).candidate = route.candidate ∧
      (expireLifecycleRoute route).candidate = route.candidate := by
  simp [withdrawLifecycleRoute, expireLifecycleRoute]

theorem lifecycle_transitions_never_strengthen_claim
    (route : LifecycleRoute) :
    (withdrawLifecycleRoute route).candidate.shape = route.candidate.shape ∧
      (withdrawLifecycleRoute route).candidate.support = route.candidate.support ∧
      (expireLifecycleRoute route).candidate.shape = route.candidate.shape ∧
      (expireLifecycleRoute route).candidate.support = route.candidate.support := by
  simp [withdrawLifecycleRoute, expireLifecycleRoute]

theorem refresh_preserves_conservativity_when_publication_unchanged
    (candidate : PublishedCandidate)
    (route : LifecycleRoute)
    (hSame : candidate = route.candidate) :
    (refreshLifecycleRoute candidate route).candidate.shape = route.candidate.shape ∧
      (refreshLifecycleRoute candidate route).candidate.support = route.candidate.support := by
  subst hSame
  simp [refreshLifecycleRoute]

theorem lifecycle_maintenance_preserves_candidate
    (route : LifecycleRoute) :
    (lifecycleMaintenance route).candidate = route.candidate := by
  unfold lifecycleMaintenance
  by_cases hSupport : route.candidate.support = 0
  · simp [hSupport, expireLifecycleRoute]
  · by_cases hOpaque : route.candidate.shape = CorridorShape.opaque
    · simp [hSupport, hOpaque, withdrawLifecycleRoute]
    · simp [hSupport, hOpaque, refreshLifecycleRoute]

theorem lifecycleMaintenance_idempotent
    (route : LifecycleRoute) :
    lifecycleMaintenance (lifecycleMaintenance route) = lifecycleMaintenance route := by
  unfold lifecycleMaintenance
  by_cases hSupport : route.candidate.support = 0
  · simp [hSupport, expireLifecycleRoute]
  · by_cases hOpaque : route.candidate.shape = CorridorShape.opaque
    · simp [hSupport, hOpaque, withdrawLifecycleRoute]
    · simp [hSupport, hOpaque, refreshLifecycleRoute]

theorem lifecycleMaintenance_refreshes_positive_nonopaque_route
    (route : LifecycleRoute)
    (hSupport : route.candidate.support ≠ 0)
    (hShape : route.candidate.shape ≠ CorridorShape.opaque) :
    lifecycleMaintenance route = { route with status := .refreshed } := by
  unfold lifecycleMaintenance
  simp [hSupport, hShape, refreshLifecycleRoute]

theorem maintain_lifecycle_preserves_candidate_view
    (routes : List LifecycleRoute) :
    (maintainLifecycle routes).map LifecycleRoute.candidate = routes.map LifecycleRoute.candidate := by
  induction routes with
  | nil =>
      simp [maintainLifecycle]
  | cons route rest ih =>
      simp [maintainLifecycle, lifecycle_maintenance_preserves_candidate]

theorem maintainLifecycle_idempotent
    (routes : List LifecycleRoute) :
    maintainLifecycle (maintainLifecycle routes) = maintainLifecycle routes := by
  induction routes with
  | nil =>
      simp [maintainLifecycle]
  | cons route rest ih =>
      simp [maintainLifecycle, lifecycleMaintenance_idempotent]

theorem observed_route_is_honest_when_publication_is_well_formed
    (candidate : PublishedCandidate)
    (hWellFormed : PublicationWellFormed candidate) :
    LifecycleHonest (observeCandidateLifecycle candidate) := by
  simpa [LifecycleHonest, observeCandidateLifecycle] using hWellFormed

theorem admitted_route_is_honest
    (admitted : AdmittedCandidate) :
    LifecycleHonest (admitCandidateLifecycle admitted) := by
  rcases admitted.admissible with ⟨_, hWellFormed, _⟩
  simpa [LifecycleHonest, admitCandidateLifecycle] using hWellFormed

theorem installed_route_is_honest
    (admitted : AdmittedCandidate) :
    LifecycleHonest (installCandidateLifecycle admitted) := by
  rcases admitted.admissible with ⟨_, hWellFormed, _⟩
  simpa [LifecycleHonest, installCandidateLifecycle] using hWellFormed

end FieldRouterLifecycle
