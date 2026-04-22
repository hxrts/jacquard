use jacquard_core::DurationMs;

// proc-macro-scope: Cast bounds are helper constants and do not need proc macros.

pub const CAST_RECEIVER_COUNT_MAX: u32 = 32;
pub const CAST_GROUP_COVERAGE_COUNT_MAX: u32 = 32;
pub const CAST_FANOUT_COUNT_MAX: u32 = 8;
pub const CAST_COPY_BUDGET_MAX: u32 = 8;
pub const CAST_EVIDENCE_AGE_MS_MAX: DurationMs = DurationMs(30_000);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CastEvidenceBounds {
    pub receiver_count_max: u32,
    pub group_coverage_count_max: u32,
    pub fanout_count_max: u32,
    pub copy_budget_max: u32,
    pub evidence_age_ms_max: DurationMs,
}

impl Default for CastEvidenceBounds {
    fn default() -> Self {
        Self {
            receiver_count_max: CAST_RECEIVER_COUNT_MAX,
            group_coverage_count_max: CAST_GROUP_COVERAGE_COUNT_MAX,
            fanout_count_max: CAST_FANOUT_COUNT_MAX,
            copy_budget_max: CAST_COPY_BUDGET_MAX,
            evidence_age_ms_max: CAST_EVIDENCE_AGE_MS_MAX,
        }
    }
}
