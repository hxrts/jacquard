//! Upper bounds for route, candidate, and payload dimensions.
//!
//! This module defines the shared capacity constants that establish hard limits
//! on routing object sizes and collection lengths throughout the workspace.
//! Bounded collections in `ServiceDescriptor`, `CommitteeSelection`, and other
//! types reference these constants in their doc comments so enforcement points
//! and limits stay traceable to one place.
//!
//! Constants defined here:
//! - [`ROUTE_HOP_COUNT_MAX`]: maximum hops on a single route path.
//! - [`PROVIDER_CANDIDATE_COUNT_MAX`]: maximum committee or candidate entries.
//! - [`SERVICE_ENDPOINT_COUNT_MAX`]: maximum endpoints on a service descriptor.
//! - [`ROUTE_PAYLOAD_BYTES_MAX`]: maximum payload size per route operation.
//! - [`REPAIR_STEP_COUNT_MAX`]: maximum repair steps for one route.
//! - [`ENVELOPE_BYTES_MAX`]: maximum envelope size for framed messages.

pub const ROUTE_HOP_COUNT_MAX: u8 = 16;
pub const PROVIDER_CANDIDATE_COUNT_MAX: u16 = 32;
pub const SERVICE_ENDPOINT_COUNT_MAX: u16 = 16;
pub const ROUTE_PAYLOAD_BYTES_MAX: crate::ByteCount = crate::ByteCount(64 * 1024);
pub const REPAIR_STEP_COUNT_MAX: u8 = 8;
pub const ENVELOPE_BYTES_MAX: crate::ByteCount = crate::ByteCount(1024);
