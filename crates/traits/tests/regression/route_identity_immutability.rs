//! Regression test: routing engines must not mutate router-owned canonical
//! identity fields during the `maintain_route` call.
//!
//! The `maintain_route` signature receives the router-owned
//! `PublishedRouteRecord` by immutable reference and only the engine-owned
//! `RouteRuntimeState` by mutable reference. This trybuild test confirms that
//! an attempt to mutate the identity record through the maintenance call fails
//! to compile, guarding the ownership invariant against future signature drift.

#[test]
fn routing_engine_cannot_mutate_router_owned_identity_in_maintenance() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/regression/ui/route_identity_mutation_in_maintenance.rs");
}
