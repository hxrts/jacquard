use jacquard_macros::{bounded_value, id_type, must_use_handle, public_model};
use serde::{Deserialize, Serialize};

#[id_type]
struct ExampleId(u16);

#[bounded_value(max = 10)]
struct RetryBudget(u8);

#[must_use_handle]
struct LeaseReceipt(u8);

#[public_model]
#[derive(Clone, Copy, PartialOrd, Ord, Hash)]
enum RouteMode {
    Pathway,
    Deferred,
}

#[public_model]
struct RouteDescriptor {
    route_id: ExampleId,
    mode: RouteMode,
}

#[test]
fn id_type_exposes_construction_helpers() {
    let route_id = ExampleId::new(7);
    assert_eq!(route_id.get(), 7);
}

#[test]
fn bounded_value_rejects_out_of_range_values() {
    assert_eq!(RetryBudget::MAX, 10);
    assert_eq!(RetryBudget::new(10).map(RetryBudget::get), Some(10));
    assert_eq!(RetryBudget::new(11), None);
}

#[test]
fn public_model_preserves_existing_derives() {
    let descriptor = RouteDescriptor {
        route_id: ExampleId::new(1),
        mode: RouteMode::Pathway,
    };

    assert_eq!(descriptor.route_id.get(), 1);
    assert!(RouteMode::Pathway < RouteMode::Deferred);
}

#[test]
fn must_use_handle_leaves_the_type_constructible() {
    let receipt = LeaseReceipt(3);
    assert_eq!(receipt.0, 3);
}
