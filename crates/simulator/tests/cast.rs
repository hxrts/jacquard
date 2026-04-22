use jacquard_core::NodeId;
use jacquard_simulator::{
    broadcast_cast_evidence_scenario, cast_report_surface_decision,
    multicast_cast_evidence_scenario, unicast_cast_evidence_scenario, CastEvidenceScenarioKind,
};

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

#[test]
fn cast_unicast_asymmetry_stays_directional_and_bounded() {
    let outcome = unicast_cast_evidence_scenario();

    assert_eq!(outcome.kind, CastEvidenceScenarioKind::Unicast);
    assert_eq!(outcome.output_count, 1);
    assert!(outcome.delivery_supported);
    assert!(!outcome.connected_bidirectional_support);
    assert_eq!(outcome.deterministic_receivers, vec![node(2)]);
}

#[test]
fn cast_multicast_partial_coverage_remains_bounded() {
    let outcome = multicast_cast_evidence_scenario();

    assert_eq!(outcome.kind, CastEvidenceScenarioKind::Multicast);
    assert_eq!(outcome.bounded_receiver_count, 2);
    assert!(outcome.delivery_supported);
    assert_eq!(outcome.deterministic_receivers, vec![node(2), node(4)]);
}

#[test]
fn cast_broadcast_lora_like_scenario_supports_storage_not_connected_support() {
    let outcome = broadcast_cast_evidence_scenario();

    assert_eq!(outcome.kind, CastEvidenceScenarioKind::Broadcast);
    assert_eq!(outcome.bounded_receiver_count, 1);
    assert!(outcome.delivery_supported);
    assert!(outcome.storage_supported);
    assert!(!outcome.connected_bidirectional_support);
}

#[test]
fn cast_report_surface_decision_is_documented() {
    assert_eq!(
        cast_report_surface_decision(),
        "cast helpers are surfaced through Mercator and diffusion outcomes"
    );
}
