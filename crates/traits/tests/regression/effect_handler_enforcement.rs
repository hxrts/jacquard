#[test]
fn missing_effect_handler_attribute_is_rejected() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/regression/ui/missing_effect_handler.rs");
}
