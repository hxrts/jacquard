//! Shared OGM receive-window primitive for sequence-number-driven engines.
//!
//! Used by the classic B.A.T.M.A.N.-style engines to track per-neighbor
//! originator sequences inside a bounded staleness window and report receive
//! quality as an occupancy permille.
// proc-macro-scope: host support primitive intentionally stays outside #[public_model].

use alloc::collections::BTreeSet;

use jacquard_core::RatioPermille;
use serde::{Deserialize, Serialize};

/// Per-neighbor OGM receive window keyed by originator sequence number.
///
/// Tracks a sliding window of recently received sequences and the last
/// monotonic observation step recorded for the window. Pruning is driven by
/// the engine's staleness configuration and the window span it advertises to
/// the occupancy calculation.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct OgmReceiveWindow {
    pub latest_sequence: Option<u64>,
    pub received_sequences: BTreeSet<u64>,
    pub last_observed_step: Option<u64>,
}

impl OgmReceiveWindow {
    pub fn observe(&mut self, sequence: u64, observed_step: u64, window_span: u64) {
        self.latest_sequence = Some(
            self.latest_sequence
                .map_or(sequence, |known| known.max(sequence)),
        );
        self.received_sequences.insert(sequence);
        self.last_observed_step = Some(observed_step);
        self.prune(observed_step, window_span, window_span);
    }

    pub fn prune(&mut self, current_step: u64, stale_after_steps: u64, window_span: u64) {
        if let Some(last_seen_step) = self.last_observed_step {
            if current_step.saturating_sub(last_seen_step) > stale_after_steps {
                self.latest_sequence = None;
                self.received_sequences.clear();
                self.last_observed_step = None;
                return;
            }
        }
        if let Some(latest_sequence) = self.latest_sequence {
            let lower_bound = latest_sequence.saturating_sub(window_span.saturating_sub(1));
            self.received_sequences
                .retain(|sequence| *sequence >= lower_bound);
            if self.received_sequences.is_empty() {
                self.latest_sequence = None;
                self.last_observed_step = None;
            }
        }
    }

    #[must_use]
    pub fn would_be_live_after_prune(
        &self,
        current_step: u64,
        stale_after_steps: u64,
        window_span: u64,
    ) -> bool {
        if let Some(last_seen_step) = self.last_observed_step {
            if current_step.saturating_sub(last_seen_step) > stale_after_steps {
                return false;
            }
        }
        if let Some(latest_sequence) = self.latest_sequence {
            let lower_bound = latest_sequence.saturating_sub(window_span.saturating_sub(1));
            self.received_sequences
                .iter()
                .any(|seq| *seq >= lower_bound)
        } else {
            false
        }
    }

    #[must_use]
    pub fn packet_count(&self) -> usize {
        self.received_sequences.len()
    }

    #[must_use]
    pub fn occupancy_permille(&self, window_span: u64) -> RatioPermille {
        if window_span == 0 {
            return RatioPermille(0);
        }
        let count = u64::try_from(self.packet_count()).unwrap_or(u64::MAX);
        let value = count.saturating_mul(1000) / window_span;
        RatioPermille(u16::try_from(value.min(1000)).expect("permille occupancy"))
    }

    #[must_use]
    pub fn is_live(&self) -> bool {
        !self.received_sequences.is_empty()
    }
}
