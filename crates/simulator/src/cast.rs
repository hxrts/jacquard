//! Simulator-facing cast evidence scenarios.
//!
//! These scenarios exercise the shared cast profile helpers without introducing
//! a transport-specific simulator lane. The report surface remains Mercator and
//! diffusion outcomes because the helpers are profile evidence shaping support.

// proc-macro-scope: Cast simulator scenarios use plain helper outputs.

use jacquard_cast_profile::{
    shape_broadcast_evidence, shape_multicast_evidence, shape_unicast_evidence,
    BroadcastObservation, BroadcastReverseConfirmation, CastEvidenceBounds, CastEvidenceMeta,
    CastEvidencePolicy, CastGroupId, MulticastObservation, ReceiverCoverageObservation,
    UnicastObservation,
};
use jacquard_core::{
    ByteCount, DurationMs, NodeId, OrderStamp as CastOrderStamp, RatioPermille, Tick,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastEvidenceScenarioKind {
    Unicast,
    Multicast,
    Broadcast,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastEvidenceScenarioOutcome {
    pub scenario_id: &'static str,
    pub kind: CastEvidenceScenarioKind,
    pub output_count: u32,
    pub bounded_receiver_count: u32,
    pub delivery_supported: bool,
    pub storage_supported: bool,
    pub connected_bidirectional_support: bool,
    pub deterministic_receivers: Vec<NodeId>,
}

#[must_use]
pub fn cast_report_surface_decision() -> &'static str {
    "cast helpers are surfaced through Mercator and diffusion outcomes"
}

#[must_use]
pub fn unicast_cast_evidence_scenario() -> CastEvidenceScenarioOutcome {
    let (evidence, _report) =
        shape_unicast_evidence([unicast_observation()], policy(ByteCount(128)));
    let first = evidence[0];
    CastEvidenceScenarioOutcome {
        scenario_id: "cast-unicast-asymmetric",
        kind: CastEvidenceScenarioKind::Unicast,
        output_count: u32::try_from(evidence.len()).unwrap_or(u32::MAX),
        bounded_receiver_count: 1,
        delivery_supported: first.directional_confidence_permille >= RatioPermille(500),
        storage_supported: false,
        connected_bidirectional_support: first.bidirectional_confidence_permille > RatioPermille(0),
        deterministic_receivers: vec![first.to],
    }
}

#[must_use]
pub fn multicast_cast_evidence_scenario() -> CastEvidenceScenarioOutcome {
    let (evidence, _report) =
        shape_multicast_evidence([multicast_observation()], policy(ByteCount(128)));
    let first = &evidence[0];
    CastEvidenceScenarioOutcome {
        scenario_id: "cast-multicast-partial-coverage",
        kind: CastEvidenceScenarioKind::Multicast,
        output_count: u32::try_from(evidence.len()).unwrap_or(u32::MAX),
        bounded_receiver_count: first.covered_receiver_count,
        delivery_supported: first.covered_receiver_count > 0,
        storage_supported: false,
        connected_bidirectional_support: false,
        deterministic_receivers: first
            .receivers
            .iter()
            .map(|receiver| receiver.receiver)
            .collect(),
    }
}

#[must_use]
pub fn broadcast_cast_evidence_scenario() -> CastEvidenceScenarioOutcome {
    let (evidence, _report) =
        shape_broadcast_evidence([broadcast_observation()], policy(ByteCount(128)));
    let first = &evidence[0];
    CastEvidenceScenarioOutcome {
        scenario_id: "cast-broadcast-lora-like",
        kind: CastEvidenceScenarioKind::Broadcast,
        output_count: u32::try_from(evidence.len()).unwrap_or(u32::MAX),
        bounded_receiver_count: u32::try_from(first.receivers.len()).unwrap_or(u32::MAX),
        delivery_supported: first.coverage_confidence_permille >= RatioPermille(500),
        storage_supported: first.custody_improvement_score() > RatioPermille(0),
        connected_bidirectional_support: first.connected_bidirectional_confidence()
            > RatioPermille(0),
        deterministic_receivers: first
            .receivers
            .iter()
            .map(|receiver| receiver.receiver)
            .collect(),
    }
}

fn policy(payload_bytes_required: ByteCount) -> CastEvidencePolicy {
    CastEvidencePolicy {
        payload_bytes_required,
        confidence_floor: RatioPermille(500),
        bounds: CastEvidenceBounds {
            receiver_count_max: 4,
            group_coverage_count_max: 4,
            fanout_count_max: 2,
            copy_budget_max: 2,
            evidence_age_ms_max: DurationMs(1_000),
        },
    }
}

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn meta(order: u64) -> CastEvidenceMeta {
    CastEvidenceMeta::new(
        Tick(1),
        DurationMs(10),
        DurationMs(1_000),
        CastOrderStamp { 0: order },
    )
}

fn receiver(byte: u8, confidence: u16) -> ReceiverCoverageObservation {
    ReceiverCoverageObservation {
        receiver: node(byte),
        confidence_permille: RatioPermille(confidence),
    }
}

fn unicast_observation() -> UnicastObservation {
    UnicastObservation {
        from: node(1),
        to: node(2),
        directional_confidence_permille: RatioPermille(850),
        reverse_confirmation_permille: None,
        payload_bytes_max: ByteCount(512),
        meta: meta(1),
    }
}

fn multicast_observation() -> MulticastObservation {
    MulticastObservation {
        sender: node(1),
        group_id: CastGroupId(b"partial".to_vec()),
        receivers: vec![receiver(4, 700), receiver(2, 850), receiver(3, 400)],
        group_pressure_permille: RatioPermille(100),
        fanout_limit: 2,
        payload_bytes_max: ByteCount(512),
        meta: meta(2),
    }
}

fn broadcast_observation() -> BroadcastObservation {
    BroadcastObservation {
        sender: node(1),
        receivers: vec![receiver(5, 800)],
        reverse_confirmation: BroadcastReverseConfirmation::Unavailable,
        transmission_window_quality_permille: RatioPermille(800),
        channel_pressure_permille: RatioPermille(100),
        copy_budget: 1,
        payload_bytes_max: ByteCount(512),
        meta: meta(3),
    }
}
