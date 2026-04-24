//! Observer-visible trace projections for coded diffusion experiments.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::diffusion::coded_inference::{
    CodedContactTraceEvent, CodedForwardingEvent, CodedInferenceReadinessLog,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum ObserverProjectionKind {
    Global,
    Regional,
    Endpoint,
    Blind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverProjectionConfig {
    pub projection_kind: ObserverProjectionKind,
    pub observed_node_ids: Vec<u32>,
    pub endpoint_node_id: Option<u32>,
    pub erase_forwarding_choices: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum ObserverEventKind {
    Contact,
    Forwarding,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ObserverTraceEvent {
    pub projection_kind: ObserverProjectionKind,
    pub event_kind: ObserverEventKind,
    pub round_index: u32,
    pub event_index: u32,
    pub artifact_namespace: String,
    pub scenario_id: String,
    pub node_a: Option<u32>,
    pub node_b: Option<u32>,
    pub sender_node_id: Option<u32>,
    pub receiver_node_id: Option<u32>,
    pub cluster_a: Option<u8>,
    pub cluster_b: Option<u8>,
    pub evidence_id: Option<u32>,
    pub fragment_id: Option<u32>,
    pub policy_id: Option<String>,
    pub byte_count: Option<u32>,
}

impl ObserverProjectionConfig {
    #[must_use]
    pub(crate) fn global() -> Self {
        Self {
            projection_kind: ObserverProjectionKind::Global,
            observed_node_ids: Vec::new(),
            endpoint_node_id: None,
            erase_forwarding_choices: false,
        }
    }

    #[must_use]
    pub(crate) fn regional(observed_node_ids: Vec<u32>) -> Self {
        Self {
            projection_kind: ObserverProjectionKind::Regional,
            observed_node_ids,
            endpoint_node_id: None,
            erase_forwarding_choices: false,
        }
    }

    #[must_use]
    pub(crate) fn endpoint(endpoint_node_id: u32) -> Self {
        Self {
            projection_kind: ObserverProjectionKind::Endpoint,
            observed_node_ids: Vec::new(),
            endpoint_node_id: Some(endpoint_node_id),
            erase_forwarding_choices: false,
        }
    }

    #[must_use]
    pub(crate) fn blind() -> Self {
        Self {
            projection_kind: ObserverProjectionKind::Blind,
            observed_node_ids: Vec::new(),
            endpoint_node_id: None,
            erase_forwarding_choices: true,
        }
    }
}

pub(crate) fn project_observer_trace(
    log: &CodedInferenceReadinessLog,
    config: &ObserverProjectionConfig,
) -> Vec<ObserverTraceEvent> {
    let observed_nodes = config
        .observed_node_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut rows = Vec::new();
    append_contact_rows(log, config, &observed_nodes, &mut rows);
    append_forwarding_rows(log, config, &observed_nodes, &mut rows);
    rows.sort_by(compare_trace_events);
    rows
}

fn append_contact_rows(
    log: &CodedInferenceReadinessLog,
    config: &ObserverProjectionConfig,
    observed_nodes: &BTreeSet<u32>,
    rows: &mut Vec<ObserverTraceEvent>,
) {
    for contact in &log.contact_events {
        if contact_visible(contact.node_a, contact.node_b, config, observed_nodes) {
            rows.push(contact_row(log, config, contact));
        }
    }
}

fn append_forwarding_rows(
    log: &CodedInferenceReadinessLog,
    config: &ObserverProjectionConfig,
    observed_nodes: &BTreeSet<u32>,
    rows: &mut Vec<ObserverTraceEvent>,
) {
    for (index, event) in log.forwarding_events.iter().enumerate() {
        if forwarding_visible(event, config, observed_nodes) {
            rows.push(forwarding_row(
                log,
                config,
                u32::try_from(index).unwrap_or(u32::MAX),
                event,
            ));
        }
    }
}

fn contact_visible(
    node_a: u32,
    node_b: u32,
    config: &ObserverProjectionConfig,
    observed_nodes: &BTreeSet<u32>,
) -> bool {
    match config.projection_kind {
        ObserverProjectionKind::Global | ObserverProjectionKind::Blind => true,
        ObserverProjectionKind::Regional => {
            observed_nodes.contains(&node_a) || observed_nodes.contains(&node_b)
        }
        ObserverProjectionKind::Endpoint => config
            .endpoint_node_id
            .is_some_and(|endpoint| node_a == endpoint || node_b == endpoint),
    }
}

fn forwarding_visible(
    event: &CodedForwardingEvent,
    config: &ObserverProjectionConfig,
    observed_nodes: &BTreeSet<u32>,
) -> bool {
    contact_visible(
        event.sender_node_id,
        event.receiver_node_id,
        config,
        observed_nodes,
    )
}

fn contact_row(
    log: &CodedInferenceReadinessLog,
    config: &ObserverProjectionConfig,
    contact: &CodedContactTraceEvent,
) -> ObserverTraceEvent {
    ObserverTraceEvent {
        projection_kind: config.projection_kind,
        event_kind: ObserverEventKind::Contact,
        round_index: contact.round_index,
        event_index: contact.contact_id,
        artifact_namespace: log.artifact_namespace.clone(),
        scenario_id: log.family_id.clone(),
        node_a: Some(contact.node_a),
        node_b: Some(contact.node_b),
        sender_node_id: None,
        receiver_node_id: None,
        cluster_a: Some(contact.cluster_a),
        cluster_b: Some(contact.cluster_b),
        evidence_id: None,
        fragment_id: None,
        policy_id: None,
        byte_count: Some(contact.bandwidth_bytes),
    }
}

fn forwarding_row(
    log: &CodedInferenceReadinessLog,
    config: &ObserverProjectionConfig,
    event_index: u32,
    event: &CodedForwardingEvent,
) -> ObserverTraceEvent {
    let erase_choices = config.erase_forwarding_choices;
    ObserverTraceEvent {
        projection_kind: config.projection_kind,
        event_kind: ObserverEventKind::Forwarding,
        round_index: event.round_index,
        event_index,
        artifact_namespace: log.artifact_namespace.clone(),
        scenario_id: log.family_id.clone(),
        node_a: None,
        node_b: None,
        sender_node_id: Some(event.sender_node_id),
        receiver_node_id: if erase_choices {
            None
        } else {
            Some(event.receiver_node_id)
        },
        cluster_a: Some(event.sender_cluster_id),
        cluster_b: if erase_choices {
            None
        } else {
            Some(event.receiver_cluster_id)
        },
        evidence_id: if erase_choices {
            None
        } else {
            Some(event.evidence_id)
        },
        fragment_id: if erase_choices {
            None
        } else {
            event.fragment_id
        },
        policy_id: if erase_choices {
            None
        } else {
            Some(event.policy_id.clone())
        },
        byte_count: Some(event.byte_count),
    }
}

fn compare_trace_events(
    left: &ObserverTraceEvent,
    right: &ObserverTraceEvent,
) -> std::cmp::Ordering {
    (
        left.round_index,
        left.event_kind,
        left.event_index,
        left.sender_node_id,
        left.receiver_node_id,
        left.node_a,
        left.node_b,
    )
        .cmp(&(
            right.round_index,
            right.event_kind,
            right.event_index,
            right.sender_node_id,
            right.receiver_node_id,
            right.node_a,
            right.node_b,
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diffusion::{
        catalog::scenarios::build_coded_inference_readiness_scenario,
        coded_inference::build_coded_inference_readiness_log,
    };

    fn log() -> CodedInferenceReadinessLog {
        let scenario = build_coded_inference_readiness_scenario();
        build_coded_inference_readiness_log(41, &scenario)
    }

    #[test]
    fn observer_projection_global_sees_contacts_and_forwarding() {
        let log = log();
        let rows = project_observer_trace(&log, &ObserverProjectionConfig::global());

        assert!(rows
            .iter()
            .any(|row| row.event_kind == ObserverEventKind::Contact));
        assert!(rows
            .iter()
            .any(|row| row.event_kind == ObserverEventKind::Forwarding));
        assert_eq!(
            rows.iter()
                .filter(|row| row.event_kind == ObserverEventKind::Contact)
                .count(),
            log.contact_events.len()
        );
    }

    #[test]
    fn observer_projection_regional_erases_unobserved_node_events() {
        let log = log();
        let rows = project_observer_trace(&log, &ObserverProjectionConfig::regional(vec![100]));

        assert!(!rows.is_empty());
        assert!(rows.iter().all(|row| row.node_a == Some(100)
            || row.node_b == Some(100)
            || row.sender_node_id == Some(100)
            || row.receiver_node_id == Some(100)));
    }

    #[test]
    fn observer_projection_endpoint_sees_only_endpoint_local_events() {
        let log = log();
        let rows = project_observer_trace(&log, &ObserverProjectionConfig::endpoint(100));

        assert!(!rows.is_empty());
        assert!(rows.iter().all(|row| row.node_a == Some(100)
            || row.node_b == Some(100)
            || row.sender_node_id == Some(100)
            || row.receiver_node_id == Some(100)));
    }

    #[test]
    fn observer_projection_blind_erases_forwarding_choices() {
        let log = log();
        let rows = project_observer_trace(&log, &ObserverProjectionConfig::blind());
        let forwarding = rows
            .iter()
            .find(|row| row.event_kind == ObserverEventKind::Forwarding)
            .expect("forwarding row");

        assert_eq!(forwarding.receiver_node_id, None);
        assert_eq!(forwarding.fragment_id, None);
        assert_eq!(forwarding.policy_id, None);
        assert!(forwarding.byte_count.is_some());
    }

    #[test]
    fn observer_projection_replay_rows_are_deterministic() {
        let log = log();
        let config = ObserverProjectionConfig::regional(vec![1, 100]);
        let first = project_observer_trace(&log, &config);
        let second = project_observer_trace(&log, &config);

        assert_eq!(first, second);
    }
}
