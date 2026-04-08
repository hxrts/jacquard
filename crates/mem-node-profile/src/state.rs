use jacquard_core::{
    Belief, ByteCount, DurationMs, Estimate, HoldItemCount, InformationSetSummary,
    InformationSummaryEncoding, NodeRelayBudget, NodeState, RatioPermille,
    RelayWorkBudget, Tick,
};

/// Mutable in-memory node-state simulator for tests.
#[derive(Clone, Debug)]
pub struct NodeStateSnapshot {
    relay_work_budget: u32,
    relay_utilization_permille: RatioPermille,
    retention_horizon_ms: DurationMs,
    available_connection_count: u32,
    hold_capacity_available_bytes: ByteCount,
    information_item_count: u32,
    information_byte_count: ByteCount,
    information_false_positive_permille: RatioPermille,
    observed_at_tick: Tick,
}

impl Default for NodeStateSnapshot {
    fn default() -> Self {
        Self {
            relay_work_budget: 4,
            relay_utilization_permille: RatioPermille(0),
            retention_horizon_ms: DurationMs(500),
            available_connection_count: 4,
            hold_capacity_available_bytes: ByteCount(4096),
            information_item_count: 0,
            information_byte_count: ByteCount(0),
            information_false_positive_permille: RatioPermille(0),
            observed_at_tick: Tick(0),
        }
    }
}

impl NodeStateSnapshot {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_relay_budget(mut self, budget: u32) -> Self {
        self.relay_work_budget = budget;
        self
    }

    #[must_use]
    pub fn with_available_connections(mut self, count: u32) -> Self {
        self.available_connection_count = count;
        self
    }

    #[must_use]
    pub fn with_hold_capacity(mut self, bytes: ByteCount) -> Self {
        self.hold_capacity_available_bytes = bytes;
        self
    }

    #[must_use]
    pub fn with_information_summary(
        mut self,
        item_count: u32,
        byte_count: ByteCount,
        false_positive_permille: RatioPermille,
    ) -> Self {
        self.information_item_count = item_count;
        self.information_byte_count = byte_count;
        self.information_false_positive_permille = false_positive_permille;
        self
    }

    #[must_use]
    pub fn with_observed_at_tick(mut self, observed_at_tick: Tick) -> Self {
        self.observed_at_tick = observed_at_tick;
        self
    }

    pub fn consume_relay_budget(&mut self, work_units: u32) {
        self.relay_work_budget = self.relay_work_budget.saturating_sub(work_units);
    }

    pub fn reserve_hold_capacity(&mut self, bytes: ByteCount) {
        self.hold_capacity_available_bytes = self.hold_capacity_available_bytes - bytes;
    }

    pub fn open_connection(&mut self) {
        self.available_connection_count =
            self.available_connection_count.saturating_sub(1);
    }

    pub fn close_connection(&mut self) {
        self.available_connection_count =
            self.available_connection_count.saturating_add(1);
    }

    #[must_use]
    pub fn build(&self) -> NodeState {
        NodeState {
            relay_budget: Belief::Estimated(Estimate {
                value: NodeRelayBudget {
                    relay_work_budget: Belief::Estimated(Estimate {
                        value: RelayWorkBudget(self.relay_work_budget),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: self.observed_at_tick,
                    }),
                    utilization_permille: self.relay_utilization_permille,
                    retention_horizon_ms: Belief::Estimated(Estimate {
                        value: self.retention_horizon_ms,
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: self.observed_at_tick,
                    }),
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: self.observed_at_tick,
            }),
            available_connection_count: Belief::Estimated(Estimate {
                value: self.available_connection_count,
                confidence_permille: RatioPermille(1000),
                updated_at_tick: self.observed_at_tick,
            }),
            hold_capacity_available_bytes: Belief::Estimated(Estimate {
                value: self.hold_capacity_available_bytes,
                confidence_permille: RatioPermille(1000),
                updated_at_tick: self.observed_at_tick,
            }),
            information_summary: Belief::Estimated(Estimate {
                value: InformationSetSummary {
                    summary_encoding: InformationSummaryEncoding::BloomFilter,
                    item_count: Belief::Estimated(Estimate {
                        value: HoldItemCount(self.information_item_count),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: self.observed_at_tick,
                    }),
                    byte_count: Belief::Estimated(Estimate {
                        value: self.information_byte_count,
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: self.observed_at_tick,
                    }),
                    false_positive_permille: Belief::Estimated(Estimate {
                        value: self.information_false_positive_permille,
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: self.observed_at_tick,
                    }),
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: self.observed_at_tick,
            }),
        }
    }
}
