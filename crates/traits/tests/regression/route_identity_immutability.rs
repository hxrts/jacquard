#[test]
fn routing_engine_cannot_mutate_router_owned_identity_in_maintenance() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/regression/ui/route_identity_mutation_in_maintenance.rs");
}
