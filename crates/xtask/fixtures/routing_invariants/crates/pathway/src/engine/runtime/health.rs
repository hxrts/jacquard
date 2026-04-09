pub fn observed_pressure_score(summary: Summary) -> u32 {
    let quiet_pressure = 100;
    quiet_pressure
        + summary.congestion_penalty_points.0.saturating_mul(50)
        + 1000_u32.saturating_sub(summary.stability_score.0) / 2
}

pub fn degradation_for_candidate(configuration: Configuration) -> u32 {
    if configuration.environment.contention_permille.get() > 600 {
        1
    } else {
        0
    }
}
