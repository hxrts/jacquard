//! `RoutingEnginePlanner` implementation: candidate generation and route
//! admission.
//!
//! Translates the private attractor view and destination belief state into
//! public routing decisions satisfying the shared framework planning contract.
//! `candidate_routes` returns one corridor candidate for the requested
//! objective: field stays a single private-selector engine even though it may
//! consider multiple admissible continuations internally. `admit_route`
//! verifies the candidate against the routing objective and returns a
//! `RouteAdmission` with a full witness.
//!
//! Admission is rejected when delivery support is below 300 permille, posterior
//! entropy exceeds 850 permille, the protection floor is unsatisfied, or the
//! connectivity posture is incompatible with the objective.
//! `route_degradation_for` classifies the degradation reason
//! (LinkInstability, CapacityConstrained, or None) from field belief state.
//! Backend tokens are encoded by `route::encode_backend_token` and embedded in the
//! returned `BackendRouteRef`. They carry one selected runtime realization plus
//! a bounded continuation envelope, not several planner-visible field candidates.

pub(crate) mod admission;
pub(crate) mod promotion;
pub(crate) mod publication;
mod surface;
