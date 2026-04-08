//! Transport-protocol constants and `LinkEndpoint` constructors: the BLE
//! GATT MTU, the `ble_endpoint(device_byte)` helper, and the `opaque_endpoint`
//! escape hatch for tests that need a non-BLE protocol tag.

use jacquard_core::{
    BleDeviceId, BleProfileId, ByteCount, EndpointAddress, LinkEndpoint,
    TransportProtocol,
};

/// BLE GATT MTU in bytes.
pub const BLE_MTU_BYTES: ByteCount = ByteCount(256);

#[must_use]
pub fn ble_endpoint(device_byte: u8) -> LinkEndpoint {
    LinkEndpoint {
        protocol: TransportProtocol::BleGatt,
        address: EndpointAddress::Ble {
            device_id: BleDeviceId(vec![device_byte]),
            profile_id: BleProfileId([device_byte; 16]),
        },
        mtu_bytes: BLE_MTU_BYTES,
    }
}

#[must_use]
pub fn opaque_endpoint(
    protocol: TransportProtocol,
    bytes: Vec<u8>,
    mtu: ByteCount,
) -> LinkEndpoint {
    LinkEndpoint {
        protocol,
        address: EndpointAddress::Opaque(bytes),
        mtu_bytes: mtu,
    }
}
