//! Shared test helpers for jacquard-traits integration tests.

use jacquard_traits::jacquard_core::{
    Belief, ByteCount, ControllerId, EndpointAddress, Estimate, FactSourceClass,
    HoldItemCount, InformationSetSummary, LinkEndpoint, MaintenanceWorkBudget, Node,
    NodeProfile, NodeRelayBudget, NodeState, Observation, OriginAuthenticationClass,
    RatioPermille, RelayWorkBudget, RoutingEvidenceClass, Tick, TransportProtocol,
};

/// Construct a local, directly-observed, controller-bound `Observation<T>`.
///
/// This is the canonical "we saw this ourselves" evidence shape used across
/// integration tests that need a plausible observation wrapper without caring
/// about the evidence provenance details.
pub fn local_observation<T>(value: T, tick: Tick) -> Observation<T> {
    Observation {
        value,
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: tick,
    }
}

/// A minimal `LinkEndpoint` suitable for use in test fixtures.
pub fn sample_endpoint() -> LinkEndpoint {
    LinkEndpoint {
        protocol: TransportProtocol::BleGatt,
        address: EndpointAddress::Opaque(vec![1, 2, 3]),
        mtu_bytes: ByteCount(512),
    }
}

/// A minimal `Node` with all-absent state beliefs, suitable for test fixtures.
pub fn sample_node() -> Node {
    Node {
        controller_id: ControllerId([3; 32]),
        profile: NodeProfile {
            services: Vec::new(),
            endpoints: vec![sample_endpoint()],
            connection_count_max: 4,
            neighbor_state_count_max: 8,
            simultaneous_transfer_count_max: 2,
            active_route_count_max: 4,
            relay_work_budget_max: RelayWorkBudget(16),
            maintenance_work_budget_max: MaintenanceWorkBudget(8),
            hold_item_count_max: HoldItemCount(8),
            hold_capacity_bytes_max: ByteCount(1024),
        },
        state: NodeState {
            relay_budget: Belief::Estimated(Estimate {
                value: NodeRelayBudget {
                    relay_work_budget: Belief::Estimated(Estimate {
                        value: RelayWorkBudget(8),
                        confidence_permille: RatioPermille(900),
                        updated_at_tick: Tick(2),
                    }),
                    utilization_permille: RatioPermille(250),
                    retention_horizon_ms: Belief::Estimated(Estimate {
                        value: jacquard_traits::jacquard_core::DurationMs(500),
                        confidence_permille: RatioPermille(900),
                        updated_at_tick: Tick(2),
                    }),
                },
                confidence_permille: RatioPermille(900),
                updated_at_tick: Tick(2),
            }),
            available_connection_count: Belief::Absent,
            hold_capacity_available_bytes: Belief::Absent,
            information_summary: Belief::Estimated(Estimate {
                value: InformationSetSummary {
                    summary_encoding:
                        jacquard_traits::jacquard_core::InformationSummaryEncoding::BloomFilter,
                    item_count: Belief::Absent,
                    byte_count: Belief::Absent,
                    false_positive_permille: Belief::Absent,
                },
                confidence_permille: RatioPermille(900),
                updated_at_tick: Tick(2),
            }),
        },
    }
}

/// Construct a `Belief::Estimated` value with the given confidence and tick.
///
/// Use this in tests instead of spelling out the full
/// `Belief::Estimated(Estimate { ... })` inline, especially when the exact
/// confidence/tick values are incidental to the test.
pub fn estimated<T>(value: T, confidence_permille: u16, tick: Tick) -> Belief<T> {
    Belief::Estimated(Estimate {
        value,
        confidence_permille: RatioPermille(confidence_permille),
        updated_at_tick: tick,
    })
}
