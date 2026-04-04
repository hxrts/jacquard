//! Upper bounds for route, candidate, and payload dimensions.

pub const ROUTE_HOP_COUNT_MAX: u8 = 16;
pub const PROVIDER_CANDIDATE_COUNT_MAX: u16 = 32;
pub const SERVICE_ENDPOINT_COUNT_MAX: u16 = 16;
pub const ROUTE_PAYLOAD_BYTES_MAX: crate::ByteCount = crate::ByteCount(64 * 1024);
pub const REPAIR_STEP_COUNT_MAX: u8 = 8;
pub const ENVELOPE_BYTES_MAX: crate::ByteCount = crate::ByteCount(1024);
