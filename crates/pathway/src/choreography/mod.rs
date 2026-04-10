//! Pathway-local Telltale choreography surface.
//!
//! This module is the internal boundary between Jacquard pathway
//! planning/runtime code and Telltale's generated protocol surfaces. Pathway
//! protocols are defined inline with `tell!` so the generated
//! session/effect code lives next to the Rust host logic that enters those
//! protocols.

mod activation;
mod anti_entropy;
mod artifacts;
mod effects;
mod forwarding;
mod handoff;
mod hold_replay;
mod neighbor_advertisement;
mod repair;
mod route_export;
mod runtime;

pub(crate) use runtime::{
    activation_handshake, anti_entropy_exchange, clear_route_protocols, forwarding_hop,
    handoff_exchange, neighbor_advertisement_exchange, record_tick_ingress, recover_held_payload,
    repair_exchange, replay_to_next_hop, retain_for_replay, route_export_exchange,
    PathwayAntiEntropySnapshot, PathwayNeighborAdvertisementSnapshot, PathwayRouteExportSnapshot,
};
