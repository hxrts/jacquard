//! Mesh choreography artifact catalog and compile path.
//!
//! Mesh runtime code reaches for choreography protocols by kind, not by file
//! name. This module is the lookup layer that maps a protocol kind to its
//! source text, compiles it through Telltale's normal choreography pipeline,
//! and keeps the forwarding helper protocol and the larger `.tell` artifacts on
//! one internal path.

use serde::{Deserialize, Serialize};
#[cfg(test)]
use telltale::{compile_choreography, CompileArtifactsError, CompiledChoreography};

#[cfg(test)]
const FORWARDING_HOP_DSL: &str = include_str!("forwarding.tell");
#[cfg(test)]
const ACTIVATION_DSL: &str = include_str!("activation.tell");
#[cfg(test)]
const REPAIR_DSL: &str = include_str!("repair.tell");
#[cfg(test)]
const HANDOFF_DSL: &str = include_str!("handoff.tell");
#[cfg(test)]
const HOLD_REPLAY_DSL: &str = include_str!("hold_replay.tell");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum MeshProtocolKind {
    ForwardingHop,
    Activation,
    Repair,
    Handoff,
    HoldReplay,
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
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MeshProtocolSessionKey(pub(crate) String);

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MeshProtocolArtifact {
    pub(crate) kind: MeshProtocolKind,
    pub(crate) source_path: &'static str,
    pub(crate) source: &'static str,
}

#[cfg(test)]
impl MeshProtocolArtifact {
    #[must_use]
    pub(crate) const fn for_kind(kind: MeshProtocolKind) -> Self {
        match kind {
            | MeshProtocolKind::ForwardingHop => Self {
                kind,
                source_path: "crates/mesh/src/choreography/forwarding.tell",
                source: FORWARDING_HOP_DSL,
            },
            | MeshProtocolKind::Activation => Self {
                kind,
                source_path: "crates/mesh/src/choreography/activation.tell",
                source: ACTIVATION_DSL,
            },
            | MeshProtocolKind::Repair => Self {
                kind,
                source_path: "crates/mesh/src/choreography/repair.tell",
                source: REPAIR_DSL,
            },
            | MeshProtocolKind::Handoff => Self {
                kind,
                source_path: "crates/mesh/src/choreography/handoff.tell",
                source: HANDOFF_DSL,
            },
            | MeshProtocolKind::HoldReplay => Self {
                kind,
                source_path: "crates/mesh/src/choreography/hold_replay.tell",
                source: HOLD_REPLAY_DSL,
            },
        }
    }
}

#[cfg(test)]
pub(crate) fn compile_protocol(
    kind: MeshProtocolKind,
) -> Result<CompiledChoreography, CompileArtifactsError> {
    compile_choreography(MeshProtocolArtifact::for_kind(kind).source)
}

#[cfg(test)]
mod tests {
    use super::{
        compile_choreography, compile_protocol, MeshProtocolArtifact, MeshProtocolKind,
    };

    fn protocol_name(kind: MeshProtocolKind) -> &'static str {
        match kind {
            | MeshProtocolKind::ForwardingHop => "ForwardingHop",
            | MeshProtocolKind::Activation => "ActivationHandshake",
            | MeshProtocolKind::Repair => "BoundedSuffixRepair",
            | MeshProtocolKind::Handoff => "SemanticHandoff",
            | MeshProtocolKind::HoldReplay => "HoldReplayExchange",
        }
    }

    #[test]
    fn every_mesh_protocol_artifact_compiles() {
        for kind in [
            MeshProtocolKind::ForwardingHop,
            MeshProtocolKind::Activation,
            MeshProtocolKind::Repair,
            MeshProtocolKind::Handoff,
            MeshProtocolKind::HoldReplay,
        ] {
            let artifact = MeshProtocolArtifact::for_kind(kind);
            let compiled = compile_protocol(kind).unwrap_or_else(|err| {
                panic!("compile {}: {err}", artifact.source_path)
            });
            assert_eq!(compiled.choreography.name, protocol_name(kind));
            assert!(
                !compiled.role_names().is_empty(),
                "compiled choreography should declare roles"
            );
        }
    }

    #[test]
    fn forwarding_hop_source_compiles() {
        let compiled = compile_choreography(super::FORWARDING_HOP_DSL)
            .expect("compile forwarding hop");
        assert_eq!(compiled.choreography.name, "ForwardingHop");
        assert!(compiled.role_names().contains(&"CurrentOwner".to_string()));
        assert!(compiled.role_names().contains(&"NextHop".to_string()));
        assert!(compiled.role_names().contains(&"Observer".to_string()));
    }
}
