//! Regression test: `#[effect_handler]` attribute must be present on every
//! effect trait implementation.
//!
//! Without `#[effect_handler]`, the compiler cannot prove that a concrete type
//! satisfies `HandlerDefinition<E>`, which is required by `EffectHandler<E>`.
//! This trybuild test confirms that omitting the attribute produces a compile
//! error rather than silently compiling and producing an incomplete handler.

#[test]
fn missing_effect_handler_attribute_is_rejected() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/regression/ui/missing_effect_handler.rs");
}
