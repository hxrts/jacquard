//! Transport-neutral endpoint construction helpers for adapter/profile authors.
//!
//! These helpers are intentionally small conveniences around shared core types.
//! They exist so human-facing profile/client examples do not need to repeat the
//! full `LinkEndpoint::new(TransportKind, EndpointLocator, ByteCount)` shape
//! for the common opaque-locator path.

use jacquard_core::{ByteCount, EndpointLocator, LinkEndpoint, TransportKind};

/// Build a shared `LinkEndpoint` with an opaque locator payload.
#[must_use]
pub fn opaque_endpoint(
    transport_kind: TransportKind,
    locator_bytes: impl Into<Vec<u8>>,
    mtu_bytes: ByteCount,
) -> LinkEndpoint {
    LinkEndpoint::new(
        transport_kind,
        EndpointLocator::Opaque(locator_bytes.into()),
        mtu_bytes,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_endpoint_preserves_transport_kind_and_mtu() {
        let endpoint = opaque_endpoint(TransportKind::WifiAware, [1_u8, 2, 3], ByteCount(128));

        assert_eq!(endpoint.transport_kind, TransportKind::WifiAware);
        assert_eq!(endpoint.mtu_bytes, ByteCount(128));
        assert_eq!(endpoint.locator, EndpointLocator::Opaque(vec![1, 2, 3]));
    }
}
