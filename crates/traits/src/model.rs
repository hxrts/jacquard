//! Engine-owned pure model surfaces for snapshot, reducer, and restore
//! execution.
//!
//! These traits standardize deterministic engine-model operations without
//! turning the simulator into a second runtime stack. Engines keep ownership of
//! private protocol state and expose only the typed model surfaces they support.

use alloc::vec::Vec;

use jacquard_core::{
    Configuration, MaterializedRoute, Observation, RouteError, RoutingObjective,
    SelectedRoutingParameters,
};
use jacquard_macros::purity;

#[purity(pure)]
/// Pure planner execution over an explicit planner snapshot.
pub trait RoutingEnginePlannerModel {
    type PlannerSnapshot;
    type PlannerCandidate;
    type PlannerAdmission;

    #[must_use]
    fn candidate_routes_from_snapshot(
        snapshot: &Self::PlannerSnapshot,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<Self::PlannerCandidate>;

    #[must_use = "unused planner admission silently discards admission evidence"]
    fn admit_route_from_snapshot(
        snapshot: &Self::PlannerSnapshot,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &Self::PlannerCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<Self::PlannerAdmission, RouteError>;
}

#[purity(pure)]
/// Pure round transition over explicit reducer state and input.
pub trait RoutingEngineRoundModel {
    type RoundState;
    type RoundInput;
    type RoundOutput;

    #[must_use]
    fn reduce_round_state(state: &Self::RoundState, input: &Self::RoundInput) -> Self::RoundOutput;
}

#[purity(pure)]
/// Pure maintenance transition over explicit reducer state and input.
pub trait RoutingEngineMaintenanceModel {
    type MaintenanceState;
    type MaintenanceInput;
    type MaintenanceOutput;

    #[must_use]
    fn reduce_maintenance_state(
        state: &Self::MaintenanceState,
        input: &Self::MaintenanceInput,
    ) -> Self::MaintenanceOutput;
}

#[purity(read_only)]
/// Read-only reconstruction of route-private runtime from router-owned
/// materialized route state.
pub trait RoutingEngineRestoreModel {
    type RestoredRoute;

    #[must_use]
    fn restore_route_runtime(route: &MaterializedRoute) -> Option<Self::RestoredRoute>;
}
