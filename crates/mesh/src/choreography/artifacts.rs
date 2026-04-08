//! Mesh choreography metadata catalog.
//!
//! Control flow intuition: runtime code refers to protocols by
//! `MeshProtocolKind` and resolves small stable metadata from this module. The
//! actual protocol bodies live inline in sibling modules via `tell!`; this
//! catalog only keeps the names and role lists that checkpoints and
//! observations need.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::{
    activation, anti_entropy, forwarding, handoff, hold_replay, neighbor_advertisement,
    repair, route_export,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum MeshProtocolKind {
    ForwardingHop,
    Activation,
    Repair,
    Handoff,
    HoldReplay,
    RouteExport,
    NeighborAdvertisement,
    AntiEntropy,
}

impl MeshProtocolKind {
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            | Self::ForwardingHop => "forwarding",
            | Self::Activation => "activation",
            | Self::Repair => "repair",
            | Self::Handoff => "handoff",
            | Self::HoldReplay => "hold-replay",
            | Self::RouteExport => "route-export",
            | Self::NeighborAdvertisement => "neighbor-advertisement",
            | Self::AntiEntropy => "anti-entropy",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MeshProtocolSessionKey(pub(crate) String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshProtocolSpec {
    pub(crate) kind: MeshProtocolKind,
    pub(crate) source_path: &'static str,
    pub(crate) protocol_name: String,
    pub(crate) role_names: Vec<String>,
}

pub(crate) fn protocol_spec(
    kind: MeshProtocolKind,
) -> Result<&'static MeshProtocolSpec, String> {
    static FORWARDING: OnceLock<Result<MeshProtocolSpec, String>> = OnceLock::new();
    static ACTIVATION: OnceLock<Result<MeshProtocolSpec, String>> = OnceLock::new();
    static REPAIR: OnceLock<Result<MeshProtocolSpec, String>> = OnceLock::new();
    static HANDOFF: OnceLock<Result<MeshProtocolSpec, String>> = OnceLock::new();
    static HOLD_REPLAY: OnceLock<Result<MeshProtocolSpec, String>> = OnceLock::new();
    static ROUTE_EXPORT: OnceLock<Result<MeshProtocolSpec, String>> = OnceLock::new();
    static NEIGHBOR_ADVERTISEMENT: OnceLock<Result<MeshProtocolSpec, String>> =
        OnceLock::new();
    static ANTI_ENTROPY: OnceLock<Result<MeshProtocolSpec, String>> = OnceLock::new();

    let slot = match kind {
        | MeshProtocolKind::ForwardingHop => &FORWARDING,
        | MeshProtocolKind::Activation => &ACTIVATION,
        | MeshProtocolKind::Repair => &REPAIR,
        | MeshProtocolKind::Handoff => &HANDOFF,
        | MeshProtocolKind::HoldReplay => &HOLD_REPLAY,
        | MeshProtocolKind::RouteExport => &ROUTE_EXPORT,
        | MeshProtocolKind::NeighborAdvertisement => &NEIGHBOR_ADVERTISEMENT,
        | MeshProtocolKind::AntiEntropy => &ANTI_ENTROPY,
    };

    slot.get_or_init(|| Ok(build_spec(kind)))
        .as_ref()
        .map_err(Clone::clone)
}

fn build_spec(kind: MeshProtocolKind) -> MeshProtocolSpec {
    match kind {
        | MeshProtocolKind::ForwardingHop => spec_from(
            kind,
            forwarding::SOURCE_PATH,
            forwarding::PROTOCOL_NAME,
            forwarding::ROLE_NAMES,
        ),
        | MeshProtocolKind::Activation => spec_from(
            kind,
            activation::SOURCE_PATH,
            activation::PROTOCOL_NAME,
            activation::ROLE_NAMES,
        ),
        | MeshProtocolKind::Repair => spec_from(
            kind,
            repair::SOURCE_PATH,
            repair::PROTOCOL_NAME,
            repair::ROLE_NAMES,
        ),
        | MeshProtocolKind::Handoff => spec_from(
            kind,
            handoff::SOURCE_PATH,
            handoff::PROTOCOL_NAME,
            handoff::ROLE_NAMES,
        ),
        | MeshProtocolKind::HoldReplay => spec_from(
            kind,
            hold_replay::SOURCE_PATH,
            hold_replay::PROTOCOL_NAME,
            hold_replay::ROLE_NAMES,
        ),
        | MeshProtocolKind::RouteExport => spec_from(
            kind,
            route_export::SOURCE_PATH,
            route_export::PROTOCOL_NAME,
            route_export::ROLE_NAMES,
        ),
        | MeshProtocolKind::NeighborAdvertisement => spec_from(
            kind,
            neighbor_advertisement::SOURCE_PATH,
            neighbor_advertisement::PROTOCOL_NAME,
            neighbor_advertisement::ROLE_NAMES,
        ),
        | MeshProtocolKind::AntiEntropy => spec_from(
            kind,
            anti_entropy::SOURCE_PATH,
            anti_entropy::PROTOCOL_NAME,
            anti_entropy::ROLE_NAMES,
        ),
    }
}

fn spec_from(
    kind: MeshProtocolKind,
    source_path: &'static str,
    protocol_name: &'static str,
    role_names: &[&'static str],
) -> MeshProtocolSpec {
    MeshProtocolSpec {
        kind,
        source_path,
        protocol_name: protocol_name.to_owned(),
        role_names: role_names.iter().map(|role| (*role).to_owned()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{protocol_spec, MeshProtocolKind};

    #[test]
    fn runtime_protocol_specs_match_inline_generated_protocols() {
        let forwarding = protocol_spec(MeshProtocolKind::ForwardingHop)
            .expect("forwarding protocol spec");
        assert_eq!(forwarding.protocol_name, "ForwardingHop");
        assert!(forwarding
            .role_names
            .iter()
            .any(|role| role == "CurrentOwner"));
        assert_eq!(
            forwarding.source_path,
            "crates/mesh/src/choreography/forwarding.rs"
        );

        let repair =
            protocol_spec(MeshProtocolKind::Repair).expect("repair protocol spec");
        assert_eq!(repair.protocol_name, "BoundedSuffixRepair");
        assert!(repair
            .role_names
            .iter()
            .any(|role| role == "CandidateRelay"));

        let export = protocol_spec(MeshProtocolKind::RouteExport)
            .expect("route export protocol spec");
        assert_eq!(export.protocol_name, "RouteExportExchange");

        let neighbor = protocol_spec(MeshProtocolKind::NeighborAdvertisement)
            .expect("neighbor advertisement protocol spec");
        assert_eq!(neighbor.protocol_name, "NeighborAdvertisementExchange");

        let anti_entropy = protocol_spec(MeshProtocolKind::AntiEntropy)
            .expect("anti-entropy protocol spec");
        assert_eq!(anti_entropy.protocol_name, "AntiEntropyExchange");
    }
}
