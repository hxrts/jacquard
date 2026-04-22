//! Router-owned delivery compatibility helpers.
//!
//! These helpers keep cast objective admission above engine internals. Profiles
//! surface delivery support, the router checks objective compatibility, and
//! host effects receive an explicit delivery intent only after that check.

use jacquard_core::{
    DeliveryCompatibility, DeliveryCompatibilityPolicy, RouteAdmissionRejection,
    RouteDeliveryObjective, TransportDeliveryIntent, TransportDeliverySupport,
};

// proc-macro-scope: Router delivery admission is plain helper logic over shared delivery models.

#[must_use = "delivery admission must be checked before host send intent is used"]
pub fn admitted_delivery_intent(
    objective: &RouteDeliveryObjective,
    support: &TransportDeliverySupport,
    policy: DeliveryCompatibilityPolicy,
) -> Result<TransportDeliveryIntent, RouteAdmissionRejection> {
    match objective.compatible_with(support, policy) {
        DeliveryCompatibility::Compatible => Ok(intent_for_support(support)),
        DeliveryCompatibility::Rejected(reason) => Err(reason),
    }
}

fn intent_for_support(support: &TransportDeliverySupport) -> TransportDeliveryIntent {
    match support {
        TransportDeliverySupport::IsolatedUnicast { endpoint, .. } => {
            TransportDeliveryIntent::Unicast {
                endpoint: endpoint.clone(),
            }
        }
        TransportDeliverySupport::Multicast {
            endpoint,
            group_id,
            receivers,
        } => TransportDeliveryIntent::Multicast {
            endpoint: endpoint.clone(),
            group_id: *group_id,
            receivers: receivers.clone(),
        },
        TransportDeliverySupport::Broadcast {
            endpoint,
            domain_id,
            ..
        } => TransportDeliveryIntent::Broadcast {
            endpoint: endpoint.clone(),
            domain_id: *domain_id,
        },
    }
}
