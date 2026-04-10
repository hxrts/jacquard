//! `NodeStateSnapshot`, a mutable in-memory node-state simulator for tests.
//!
//! This module tracks the runtime state half of a simulated node: relay work
//! budget and utilization, retention horizon, available connection count,
//! hold-capacity headroom, and information-set summary (item count, byte count,
//! Bloom false-positive rate). These fields map directly onto `NodeState` from
//! `jacquard-core`.
//!
//! Callers can vary each dimension independently via builder methods, then call
//! `build` to produce a fully specified `NodeState` snapshot. Three imperative
//! mutation helpers (`consume_relay_budget`, `reserve_hold_capacity`,
//! `open_connection`, `close_connection`) let tests simulate incremental state
//! changes without rebuilding from scratch.
//!
//! `route_capable` is a preset constructor that sets generous defaults suitable
//! for most routing-engine tests. Use the builder methods to override
//! individual fields when a test scenario requires tighter or looser capacity
//! constraints.

use jacquard_core::{
    ByteCount, DurationMs, HoldItemCount, NodeState, NodeStateBuilder, RatioPermille,
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
    pub fn with_relay_state(
        mut self,
        relay_work_budget: u32,
        relay_utilization_permille: RatioPermille,
        retention_horizon_ms: DurationMs,
    ) -> Self {
        self.relay_work_budget = relay_work_budget;
        self.relay_utilization_permille = relay_utilization_permille;
        self.retention_horizon_ms = retention_horizon_ms;
        self
    }

    #[must_use]
    pub fn with_information_set(
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
        self.available_connection_count = self.available_connection_count.saturating_sub(1);
    }

    pub fn close_connection(&mut self) {
        self.available_connection_count = self.available_connection_count.saturating_add(1);
    }

    #[must_use]
    pub fn build(&self) -> NodeState {
        NodeStateBuilder::new()
            .with_relay_budget(
                RelayWorkBudget(self.relay_work_budget),
                self.relay_utilization_permille,
                self.retention_horizon_ms,
                self.observed_at_tick,
            )
            .with_available_connections(self.available_connection_count, self.observed_at_tick)
            .with_hold_capacity(self.hold_capacity_available_bytes, self.observed_at_tick)
            .with_information_summary(
                HoldItemCount(self.information_item_count),
                self.information_byte_count,
                self.information_false_positive_permille,
                self.observed_at_tick,
            )
            .build()
    }

    #[must_use]
    pub fn route_capable(observed_at_tick: Tick) -> Self {
        Self::new()
            .with_relay_state(8, RatioPermille(0), DurationMs(500))
            .with_available_connections(4)
            .with_hold_capacity(ByteCount(4096))
            .with_information_set(4, ByteCount(2048), RatioPermille(10))
            .with_observed_at_tick(observed_at_tick)
    }
}
