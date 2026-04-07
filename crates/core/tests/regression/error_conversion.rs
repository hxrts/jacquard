//! Verify error conversion preserves rejection context through From impls.

use jacquard_core::{RouteAdmissionRejection, RouteError, RouteSelectionError};

#[test]
fn route_error_preserves_selection_rejection_context() {
    let error = RouteError::from(RouteSelectionError::Inadmissible(
        RouteAdmissionRejection::ProtectionFloorUnsatisfied,
    ));

    match error {
        | RouteError::Selection(RouteSelectionError::Inadmissible(reason)) => {
            assert_eq!(reason, RouteAdmissionRejection::ProtectionFloorUnsatisfied);
        },
        | other => panic!("unexpected error shape: {other}"),
    }
}
