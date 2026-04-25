//! Rolling reproduction-pressure accounting.

use serde::{Deserialize, Serialize};

const WINDOW_MAX: usize = 32;
const PERMILLE_MAX: u32 = 1_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ReproductionPressureEvent {
    pub active_forwarding_opportunities: u32,
    pub innovative_successor_opportunities: u32,
    pub raw_copies: u32,
    pub innovative_copies: u32,
    pub receiver_arrival_opportunities: u32,
    pub duplicate_arrivals: u32,
    pub decision_quality_improvements: u32,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ReproductionPressureSummary {
    pub window_len: u32,
    pub active_forwarding_opportunities: u32,
    pub innovative_successor_opportunities: u32,
    pub r_est_permille: u32,
    pub raw_copies: u32,
    pub innovative_copies: u32,
    pub receiver_arrival_opportunities: u32,
    pub duplicate_arrivals: u32,
    pub decision_quality_improvements: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RollingReproductionPressure {
    window_capacity: usize,
    events: Vec<ReproductionPressureEvent>,
}

impl RollingReproductionPressure {
    pub(crate) fn new(window_capacity: usize) -> Self {
        assert!(window_capacity > 0);
        assert!(window_capacity <= WINDOW_MAX);
        Self {
            window_capacity,
            events: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, event: ReproductionPressureEvent) {
        if self.events.len() == self.window_capacity {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    pub(crate) fn summary(&self) -> ReproductionPressureSummary {
        let mut summary = ReproductionPressureSummary {
            window_len: u32::try_from(self.events.len()).unwrap_or(u32::MAX),
            ..ReproductionPressureSummary::default()
        };
        for event in &self.events {
            summary.active_forwarding_opportunities = summary
                .active_forwarding_opportunities
                .saturating_add(event.active_forwarding_opportunities);
            summary.innovative_successor_opportunities = summary
                .innovative_successor_opportunities
                .saturating_add(event.innovative_successor_opportunities);
            summary.raw_copies = summary.raw_copies.saturating_add(event.raw_copies);
            summary.innovative_copies = summary
                .innovative_copies
                .saturating_add(event.innovative_copies);
            summary.receiver_arrival_opportunities = summary
                .receiver_arrival_opportunities
                .saturating_add(event.receiver_arrival_opportunities);
            summary.duplicate_arrivals = summary
                .duplicate_arrivals
                .saturating_add(event.duplicate_arrivals);
            summary.decision_quality_improvements = summary
                .decision_quality_improvements
                .saturating_add(event.decision_quality_improvements);
        }
        summary.r_est_permille = ratio_permille(
            summary.innovative_successor_opportunities,
            summary.active_forwarding_opportunities,
        );
        summary
    }
}

pub(crate) fn reproduction_pressure_from_trace(
    window_capacity: usize,
    events: &[ReproductionPressureEvent],
) -> ReproductionPressureSummary {
    let mut pressure = RollingReproductionPressure::new(window_capacity);
    for event in events {
        pressure.push(*event);
    }
    pressure.summary()
}

fn ratio_permille(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    let scaled = u64::from(numerator)
        .saturating_mul(u64::from(PERMILLE_MAX))
        .saturating_div(u64::from(denominator));
    u32::try_from(scaled.min(u64::from(PERMILLE_MAX))).unwrap_or(PERMILLE_MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        reproduction_pressure_from_trace, ReproductionPressureEvent, RollingReproductionPressure,
    };

    fn event(active: u32, innovative: u32, duplicate: u32) -> ReproductionPressureEvent {
        ReproductionPressureEvent {
            active_forwarding_opportunities: active,
            innovative_successor_opportunities: innovative,
            raw_copies: active,
            innovative_copies: innovative,
            receiver_arrival_opportunities: active,
            duplicate_arrivals: duplicate,
            decision_quality_improvements: innovative,
        }
    }

    #[test]
    fn reproduction_pressure_empty_window_is_zero() {
        let pressure = RollingReproductionPressure::new(4);

        assert_eq!(pressure.summary().r_est_permille, 0);
        assert_eq!(pressure.summary().window_len, 0);
    }

    #[test]
    fn reproduction_pressure_full_and_rollover_windows_are_bounded() {
        let mut pressure = RollingReproductionPressure::new(2);
        pressure.push(event(4, 1, 3));
        pressure.push(event(4, 2, 2));
        assert_eq!(pressure.summary().r_est_permille, 375);

        pressure.push(event(4, 4, 0));
        let summary = pressure.summary();
        assert_eq!(summary.window_len, 2);
        assert_eq!(summary.active_forwarding_opportunities, 8);
        assert_eq!(summary.innovative_successor_opportunities, 6);
        assert_eq!(summary.r_est_permille, 750);
    }

    #[test]
    fn reproduction_pressure_all_duplicate_and_all_innovative_windows() {
        let duplicate = reproduction_pressure_from_trace(4, &[event(4, 0, 4), event(2, 0, 2)]);
        let innovative = reproduction_pressure_from_trace(4, &[event(4, 4, 0), event(2, 2, 0)]);

        assert_eq!(duplicate.r_est_permille, 0);
        assert_eq!(duplicate.duplicate_arrivals, 6);
        assert_eq!(innovative.r_est_permille, 1000);
        assert_eq!(innovative.innovative_copies, 6);
    }

    #[test]
    fn reproduction_pressure_mixed_window_tracks_separate_counters() {
        let summary = reproduction_pressure_from_trace(4, &[event(10, 4, 6), event(10, 6, 4)]);

        assert_eq!(summary.r_est_permille, 500);
        assert_eq!(summary.raw_copies, 20);
        assert_eq!(summary.innovative_copies, 10);
        assert_eq!(summary.receiver_arrival_opportunities, 20);
        assert_eq!(summary.duplicate_arrivals, 10);
        assert_eq!(summary.decision_quality_improvements, 10);
    }

    #[test]
    fn reproduction_pressure_maximum_bound_saturates_without_float_state() {
        let summary = reproduction_pressure_from_trace(4, &[event(u32::MAX, u32::MAX, u32::MAX)]);

        assert_eq!(summary.r_est_permille, 1000);
        assert_eq!(summary.raw_copies, u32::MAX);
    }

    #[test]
    fn reproduction_pressure_replay_is_deterministic() {
        let trace = vec![event(3, 1, 2), event(5, 4, 1), event(2, 1, 1)];

        let first = reproduction_pressure_from_trace(3, &trace);
        let second = reproduction_pressure_from_trace(3, &trace);

        assert_eq!(first, second);
    }
}
// proc-macro-scope: near-critical reproduction rows are artifact schema, not shared model vocabulary.
