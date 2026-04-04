#[test]
fn macro_regressions() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/regression/ui/*.rs");
}
