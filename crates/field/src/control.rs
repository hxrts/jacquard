//! Control plane: regime observation, posture selection, and PI price updates.
//!
//! Measures network pressure signals from topology observations — congestion,
//! relay load, retention demand, churn rate, and adversarial risk — and
//! compresses destination state into a `MeanFieldState`. A
//! proportional-integral control loop produces price signals for each pressure
//! dimension.
//!
//! `observe_regime` detects transitions among five operating regimes (Sparse,
//! Congested, RetentionFavorable, Unstable, Adversarial) with configurable
//! dwell-time hysteresis to prevent oscillation. `choose_posture` maps the
//! current regime and field conditions to one of four routing postures:
//! Opportunistic, Structured, RetentionBiased, or RiskSuppressed.
//!
//! `advance_control_plane` is the entry point called from `engine_tick`; it
//! runs all four phases and returns a `ControlTickOutcome` carrying the new
//! regime, posture, and price vector for use by the attractor and observer.

use jacquard_core::{Configuration, NodeId, Tick};

use crate::state::{
    ControlState, DestinationFieldState, DivergenceBucket, EntropyBucket, FieldEngineState,
    MeanFieldState, OperatingRegime, PostureControllerState, RegimeObserverState, ResidualBucket,
    RoutingPosture, SupportBucket,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ControlMeasurements {
    pub(crate) congestion_pressure: u16,
    pub(crate) relay_pressure: u16,
    pub(crate) retention_pressure: u16,
    pub(crate) churn_pressure: u16,
    pub(crate) risk_pressure: u16,
}

impl ControlMeasurements {
    #[must_use]
    pub(crate) fn new(
        congestion_pressure: u16,
        relay_pressure: u16,
        retention_pressure: u16,
        churn_pressure: u16,
        risk_pressure: u16,
    ) -> Self {
        Self {
            congestion_pressure: congestion_pressure.min(1000),
            relay_pressure: relay_pressure.min(1000),
            retention_pressure: retention_pressure.min(1000),
            churn_pressure: churn_pressure.min(1000),
            risk_pressure: risk_pressure.min(1000),
        }
    }

    #[must_use]
    // long-block-exception: this is one bounded projection from topology
    // observations into the control-plane pressure vector.
    pub(crate) fn from_topology(topology: &Configuration, local_node_id: NodeId) -> Self {
        let congestion_pressure = topology.environment.contention_permille.0;
        let churn_pressure = topology.environment.churn_permille.0;
        let fallback_risk =
            if topology.environment.reachable_neighbor_count == 0 && topology.nodes.len() > 1 {
                800
            } else {
                0
            };

        let Some(local_node) = topology.nodes.get(&local_node_id) else {
            return Self::new(
                congestion_pressure,
                0,
                0,
                churn_pressure,
                churn_pressure.max(fallback_risk),
            );
        };

        let relay_pressure = local_node
            .state
            .relay_budget
            .value()
            .map(|budget| budget.utilization_permille.0)
            .unwrap_or_else(|| {
                utilization_pressure(
                    local_node.profile.connection_count_max,
                    local_node
                        .state
                        .available_connection_count
                        .value()
                        .unwrap_or(local_node.profile.connection_count_max),
                )
            });

        let retention_pressure = byte_pressure(
            local_node.profile.hold_capacity_bytes_max.0,
            local_node
                .state
                .hold_capacity_available_bytes
                .value()
                .map(|bytes| bytes.0)
                .unwrap_or(local_node.profile.hold_capacity_bytes_max.0),
        );

        let risk_pressure = churn_pressure
            .max(fallback_risk)
            .max(retention_pressure / 2)
            .max(congestion_pressure / 3);

        Self::new(
            congestion_pressure,
            relay_pressure,
            retention_pressure,
            churn_pressure,
            risk_pressure,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ControlTickOutcome {
    pub(crate) changed: bool,
    pub(crate) measurements: ControlMeasurements,
}

#[must_use]
pub(crate) fn advance_control_plane(
    state: &mut FieldEngineState,
    measurements: ControlMeasurements,
    now_tick: Tick,
) -> ControlTickOutcome {
    let next_mean_field = compress_mean_field(state.destinations.values(), measurements);
    let next_control = update_control_state(&state.controller, &next_mean_field, measurements);
    let next_regime = observe_regime(
        &state.regime,
        &next_mean_field,
        &next_control,
        measurements,
        now_tick,
    );
    let next_posture = choose_posture(
        &state.posture,
        &next_regime,
        &next_mean_field,
        &next_control,
        now_tick,
    );

    let changed = state.mean_field != next_mean_field
        || state.controller != next_control
        || state.regime != next_regime
        || state.posture != next_posture;

    state.mean_field = next_mean_field;
    state.controller = next_control;
    state.regime = next_regime;
    state.posture = next_posture;
    sync_regime_belief(state);

    ControlTickOutcome {
        changed,
        measurements,
    }
}

#[must_use]
// long-block-exception: mean-field compression is one projection pass from
// bounded destination observations into the low-order control state.
pub(crate) fn compress_mean_field<'a, I>(
    destinations: I,
    measurements: ControlMeasurements,
) -> MeanFieldState
where
    I: IntoIterator<Item = &'a DestinationFieldState>,
{
    let mut destination_count = 0_u32;
    let mut support_sum = 0_u32;
    let mut entropy_sum = 0_u32;
    let mut congestion_sum = 0_u32;
    let mut retention_sum = 0_u32;

    for destination in destinations {
        destination_count = destination_count.saturating_add(1);
        support_sum =
            support_sum.saturating_add(u32::from(destination.posterior.top_corridor_mass.value()));
        entropy_sum =
            entropy_sum.saturating_add(u32::from(destination.posterior.usability_entropy.value()));
        congestion_sum = congestion_sum.saturating_add(u32::from(
            destination.corridor_belief.congestion_penalty.value(),
        ));
        retention_sum = retention_sum.saturating_add(u32::from(
            destination.corridor_belief.retention_affinity.value(),
        ));
    }

    if destination_count == 0 {
        return MeanFieldState {
            relay_alignment: SupportBucket::new(
                1000_u16.saturating_sub(measurements.relay_pressure / 2),
            ),
            congestion_alignment: SupportBucket::new(
                1000_u16.saturating_sub(measurements.congestion_pressure / 2),
            ),
            retention_alignment: SupportBucket::new(
                1000_u16.saturating_sub(measurements.retention_pressure / 2),
            ),
            risk_alignment: SupportBucket::new(
                1000_u16.saturating_sub(
                    measurements.risk_pressure.max(measurements.churn_pressure) / 2,
                ),
            ),
            field_strength: SupportBucket::new(1000_u16.saturating_sub(
                average_u16(&[
                    measurements.relay_pressure,
                    measurements.congestion_pressure,
                    measurements.retention_pressure,
                    measurements.risk_pressure,
                ]) / 2,
            )),
        };
    }

    let destination_count_u16 =
        u16::try_from(destination_count).expect("bounded destination count fits u16");
    let avg_support =
        u16::try_from(support_sum / destination_count).expect("posterior support average fits u16");
    let avg_entropy =
        u16::try_from(entropy_sum / destination_count).expect("entropy average fits u16");
    let avg_congestion =
        u16::try_from(congestion_sum / destination_count).expect("congestion average fits u16");
    let avg_retention =
        u16::try_from(retention_sum / destination_count).expect("retention average fits u16");

    let relay_alignment = SupportBucket::new(average_u16(&[
        avg_support,
        1000_u16.saturating_sub(measurements.relay_pressure),
        1000_u16.saturating_sub(avg_entropy / 2),
    ]));
    let congestion_alignment =
        SupportBucket::new(alignment(avg_congestion, measurements.congestion_pressure));
    let retention_alignment =
        SupportBucket::new(alignment(avg_retention, measurements.retention_pressure));
    let risk_alignment = SupportBucket::new(alignment(
        avg_entropy,
        measurements.risk_pressure.max(measurements.churn_pressure),
    ));
    let field_strength = SupportBucket::new(average_u16(&[
        avg_support,
        1000_u16.saturating_sub(avg_entropy),
        relay_alignment.value(),
        congestion_alignment.value(),
        retention_alignment.value(),
        risk_alignment.value(),
        destination_count_u16.saturating_mul(20).min(1000),
    ]));

    MeanFieldState {
        relay_alignment,
        congestion_alignment,
        retention_alignment,
        risk_alignment,
        field_strength,
    }
}

#[must_use]
pub(crate) fn update_control_state(
    previous: &ControlState,
    mean_field: &MeanFieldState,
    measurements: ControlMeasurements,
) -> ControlState {
    let congestion_residual = bounded_residual(
        measurements.congestion_pressure,
        mean_field.congestion_alignment,
    );
    let relay_residual = bounded_residual(measurements.relay_pressure, mean_field.relay_alignment);
    let retention_residual = bounded_residual(
        measurements.retention_pressure,
        mean_field.retention_alignment,
    );
    let churn_residual = bounded_residual(measurements.churn_pressure, mean_field.risk_alignment);
    let risk_residual = bounded_residual(measurements.risk_pressure, mean_field.risk_alignment);

    ControlState {
        congestion_price: bounded_price(
            previous.congestion_price,
            measurements.congestion_pressure,
            mean_field.congestion_alignment,
        ),
        relay_price: bounded_price(
            previous.relay_price,
            measurements.relay_pressure,
            mean_field.relay_alignment,
        ),
        retention_price: bounded_price(
            previous.retention_price,
            measurements.retention_pressure,
            mean_field.retention_alignment,
        ),
        risk_price: bounded_price(
            previous.risk_price,
            measurements.risk_pressure.max(measurements.churn_pressure),
            mean_field.risk_alignment,
        ),
        congestion_error_integral: bounded_integral(
            previous.congestion_error_integral,
            congestion_residual,
        ),
        retention_error_integral: bounded_integral(
            previous.retention_error_integral,
            retention_residual,
        ),
        relay_error_integral: bounded_integral(previous.relay_error_integral, relay_residual),
        churn_error_integral: bounded_integral(
            previous.churn_error_integral,
            churn_residual.max(risk_residual),
        ),
    }
}

#[must_use]
// long-block-exception: regime observation intentionally keeps residual,
// evidence, and switching logic in one deterministic evaluation pass.
pub(crate) fn observe_regime(
    previous: &RegimeObserverState,
    mean_field: &MeanFieldState,
    control: &ControlState,
    measurements: ControlMeasurements,
    now_tick: Tick,
) -> RegimeObserverState {
    let scored = scored_regimes(mean_field, control, measurements);
    let candidate = scored[0];
    let current_support = regime_support(previous.current, mean_field, control, measurements);
    let current_residual = 1000_u16.saturating_sub(current_support);
    let candidate_residual = 1000_u16.saturating_sub(candidate.1);
    let margin = candidate.1.saturating_sub(scored[1].1);

    if candidate.0 == previous.current
        || current_residual <= previous.regime_hysteresis_threshold.value()
    {
        return RegimeObserverState {
            current: previous.current,
            current_regime_score: SupportBucket::new(current_support),
            regime_error_residual: ResidualBucket::new(
                previous
                    .regime_error_residual
                    .value()
                    .saturating_sub(previous.regime_hysteresis_threshold.value() / 4)
                    .saturating_add(current_residual / 5),
            ),
            log_likelihood_margin: DivergenceBucket::new(margin),
            regime_change_threshold: previous.regime_change_threshold,
            regime_hysteresis_threshold: previous.regime_hysteresis_threshold,
            dwell_until_tick: previous.dwell_until_tick,
        };
    }

    let accumulated = previous
        .regime_error_residual
        .value()
        .saturating_add(current_residual.saturating_sub(candidate_residual / 2));

    if now_tick < previous.dwell_until_tick {
        return RegimeObserverState {
            current: previous.current,
            current_regime_score: SupportBucket::new(current_support),
            regime_error_residual: ResidualBucket::new(accumulated),
            log_likelihood_margin: DivergenceBucket::new(margin),
            regime_change_threshold: previous.regime_change_threshold,
            regime_hysteresis_threshold: previous.regime_hysteresis_threshold,
            dwell_until_tick: previous.dwell_until_tick,
        };
    }

    if accumulated >= previous.regime_change_threshold.value() {
        return RegimeObserverState {
            current: candidate.0,
            current_regime_score: SupportBucket::new(candidate.1),
            regime_error_residual: ResidualBucket::new(candidate_residual / 2),
            log_likelihood_margin: DivergenceBucket::new(margin),
            regime_change_threshold: previous.regime_change_threshold,
            regime_hysteresis_threshold: previous.regime_hysteresis_threshold,
            dwell_until_tick: Tick(now_tick.0.saturating_add(3)),
        };
    }

    RegimeObserverState {
        current: previous.current,
        current_regime_score: SupportBucket::new(current_support),
        regime_error_residual: ResidualBucket::new(accumulated),
        log_likelihood_margin: DivergenceBucket::new(margin),
        regime_change_threshold: previous.regime_change_threshold,
        regime_hysteresis_threshold: previous.regime_hysteresis_threshold,
        dwell_until_tick: previous.dwell_until_tick,
    }
}

#[must_use]
// long-block-exception: posture choice keeps hysteresis, dwell, and
// regime-primary preference in one deterministic transition rule.
pub(crate) fn choose_posture(
    previous: &PostureControllerState,
    regime: &RegimeObserverState,
    mean_field: &MeanFieldState,
    control: &ControlState,
    now_tick: Tick,
) -> PostureControllerState {
    let preferred = preferred_posture(regime.current, mean_field, control);
    let preferred_score = posture_score(preferred, regime.current, mean_field, control);
    let current_score = posture_score(previous.current, regime.current, mean_field, control);
    let in_dwell = now_tick < Tick(previous.last_transition_tick.0.saturating_add(2));
    let primary_posture = primary_posture_for_regime(regime.current);
    let effective_threshold = if preferred == primary_posture
        && previous.current != primary_posture
        && regime.current_regime_score.value() >= 850
    {
        previous.posture_switch_threshold.value() / 4
    } else {
        previous.posture_switch_threshold.value()
    };

    if in_dwell && preferred != previous.current {
        return previous.clone();
    }

    if preferred == RoutingPosture::RiskSuppressed
        && previous.current != RoutingPosture::RiskSuppressed
        && mean_field.field_strength.value() >= 260
        && mean_field.retention_alignment.value() >= 340
        && mean_field.relay_alignment.value() >= 300
        && control.risk_price.value() <= 760
        && regime.current != OperatingRegime::Adversarial
    {
        return PostureControllerState {
            current: previous.current,
            stability_margin: SupportBucket::new(
                previous
                    .stability_margin
                    .value()
                    .max(mean_field.field_strength.value()),
            ),
            convergence_score: SupportBucket::new(
                regime.current_regime_score.value().max(current_score),
            ),
            posture_switch_threshold: previous.posture_switch_threshold,
            last_transition_tick: previous.last_transition_tick,
        };
    }

    let preference_gap = preferred_score.saturating_sub(current_score);
    if preferred == previous.current || preference_gap < effective_threshold {
        return PostureControllerState {
            current: previous.current,
            stability_margin: SupportBucket::new(
                previous
                    .stability_margin
                    .value()
                    .max(mean_field.field_strength.value()),
            ),
            convergence_score: SupportBucket::new(
                regime.current_regime_score.value().max(current_score),
            ),
            posture_switch_threshold: previous.posture_switch_threshold,
            last_transition_tick: previous.last_transition_tick,
        };
    }

    PostureControllerState {
        current: preferred,
        stability_margin: SupportBucket::new(mean_field.field_strength.value()),
        convergence_score: SupportBucket::new(preferred_score),
        posture_switch_threshold: previous.posture_switch_threshold,
        last_transition_tick: now_tick,
    }
}

fn scored_regimes(
    mean_field: &MeanFieldState,
    control: &ControlState,
    measurements: ControlMeasurements,
) -> [(OperatingRegime, u16); 5] {
    let mut scored = [
        (
            OperatingRegime::Sparse,
            regime_support(OperatingRegime::Sparse, mean_field, control, measurements),
        ),
        (
            OperatingRegime::Congested,
            regime_support(
                OperatingRegime::Congested,
                mean_field,
                control,
                measurements,
            ),
        ),
        (
            OperatingRegime::RetentionFavorable,
            regime_support(
                OperatingRegime::RetentionFavorable,
                mean_field,
                control,
                measurements,
            ),
        ),
        (
            OperatingRegime::Unstable,
            regime_support(OperatingRegime::Unstable, mean_field, control, measurements),
        ),
        (
            OperatingRegime::Adversarial,
            regime_support(
                OperatingRegime::Adversarial,
                mean_field,
                control,
                measurements,
            ),
        ),
    ];
    scored.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    scored
}

fn regime_support(
    regime: OperatingRegime,
    mean_field: &MeanFieldState,
    control: &ControlState,
    measurements: ControlMeasurements,
) -> u16 {
    match regime {
        OperatingRegime::Sparse => average_u16(&[
            mean_field.field_strength.value(),
            1000_u16.saturating_sub(measurements.congestion_pressure),
            1000_u16.saturating_sub(measurements.risk_pressure.max(measurements.churn_pressure)),
            1000_u16.saturating_sub(control.retention_price.value() / 2),
        ]),
        OperatingRegime::Congested => average_u16(&[
            measurements
                .congestion_pressure
                .max(control.congestion_price.value()),
            mean_field.congestion_alignment.value(),
            mean_field.relay_alignment.value(),
            1000_u16.saturating_sub(measurements.risk_pressure / 2),
        ]),
        OperatingRegime::RetentionFavorable => average_u16(&[
            measurements
                .retention_pressure
                .max(control.retention_price.value()),
            mean_field.retention_alignment.value(),
            mean_field.field_strength.value(),
            1000_u16.saturating_sub(measurements.risk_pressure / 2),
        ]),
        OperatingRegime::Unstable => average_u16(&[
            measurements.churn_pressure.max(measurements.risk_pressure),
            mean_field.risk_alignment.value(),
            control.risk_price.value(),
            1000_u16.saturating_sub(mean_field.field_strength.value() / 2),
        ]),
        OperatingRegime::Adversarial => average_u16(&[
            measurements.risk_pressure,
            control.risk_price.value(),
            measurements.churn_pressure,
            1000_u16.saturating_sub(mean_field.field_strength.value() / 2),
        ]),
    }
}

fn preferred_posture(
    regime: OperatingRegime,
    mean_field: &MeanFieldState,
    control: &ControlState,
) -> RoutingPosture {
    let postures = [
        RoutingPosture::Opportunistic,
        RoutingPosture::Structured,
        RoutingPosture::RetentionBiased,
        RoutingPosture::RiskSuppressed,
    ];
    *postures
        .iter()
        .max_by_key(|posture| posture_score(**posture, regime, mean_field, control))
        .expect("at least one posture")
}

fn posture_score(
    posture: RoutingPosture,
    regime: OperatingRegime,
    mean_field: &MeanFieldState,
    control: &ControlState,
) -> u16 {
    let regime_bonus = match (regime, posture) {
        (OperatingRegime::Sparse, RoutingPosture::Opportunistic) => 250,
        (OperatingRegime::Sparse, RoutingPosture::Structured) => 150,
        (OperatingRegime::Congested, RoutingPosture::Structured) => 200,
        (OperatingRegime::Congested, RoutingPosture::RetentionBiased) => 250,
        (OperatingRegime::RetentionFavorable, RoutingPosture::RetentionBiased) => 350,
        (OperatingRegime::Unstable, RoutingPosture::RiskSuppressed)
        | (OperatingRegime::Adversarial, RoutingPosture::RiskSuppressed) => 350,
        _ => 0,
    };

    match posture {
        RoutingPosture::Opportunistic => average_u16(&[
            mean_field.field_strength.value(),
            mean_field.relay_alignment.value(),
            1000_u16.saturating_sub(control.risk_price.value()),
            regime_bonus,
        ]),
        RoutingPosture::Structured => average_u16(&[
            mean_field.field_strength.value(),
            mean_field.relay_alignment.value(),
            mean_field.congestion_alignment.value(),
            regime_bonus,
        ]),
        RoutingPosture::RetentionBiased => average_u16(&[
            mean_field.retention_alignment.value(),
            mean_field.retention_alignment.value(),
            control.retention_price.value(),
            mean_field.field_strength.value(),
            regime_bonus,
        ]),
        RoutingPosture::RiskSuppressed => average_u16(&[
            control.risk_price.value(),
            mean_field.risk_alignment.value(),
            1000_u16.saturating_sub(mean_field.field_strength.value() / 2),
            regime_bonus,
        ]),
    }
}

fn primary_posture_for_regime(regime: OperatingRegime) -> RoutingPosture {
    match regime {
        OperatingRegime::Sparse => RoutingPosture::Opportunistic,
        OperatingRegime::Congested => RoutingPosture::Structured,
        OperatingRegime::RetentionFavorable => RoutingPosture::RetentionBiased,
        OperatingRegime::Unstable | OperatingRegime::Adversarial => RoutingPosture::RiskSuppressed,
    }
}

fn sync_regime_belief(state: &mut FieldEngineState) {
    for destination in state.destinations.values_mut() {
        destination.posterior.regime_belief.sparse =
            regime_support_bucket(state.regime.current, OperatingRegime::Sparse, &state.regime);
        destination.posterior.regime_belief.congested = regime_support_bucket(
            state.regime.current,
            OperatingRegime::Congested,
            &state.regime,
        );
        destination.posterior.regime_belief.retention_favorable = regime_support_bucket(
            state.regime.current,
            OperatingRegime::RetentionFavorable,
            &state.regime,
        );
        destination.posterior.regime_belief.unstable = regime_support_bucket(
            state.regime.current,
            OperatingRegime::Unstable,
            &state.regime,
        );
        destination.posterior.regime_belief.adversarial = regime_support_bucket(
            state.regime.current,
            OperatingRegime::Adversarial,
            &state.regime,
        );
    }
}

fn regime_support_bucket(
    current: OperatingRegime,
    target: OperatingRegime,
    regime: &RegimeObserverState,
) -> SupportBucket {
    if current == target {
        regime.current_regime_score
    } else {
        SupportBucket::new(
            regime
                .current_regime_score
                .value()
                .saturating_sub(regime.log_likelihood_margin.value() / 2),
        )
    }
}

fn bounded_price(
    previous: EntropyBucket,
    measurement: u16,
    alignment: SupportBucket,
) -> EntropyBucket {
    let next = average_u16(&[
        previous.value(),
        previous.value(),
        measurement,
        1000_u16.saturating_sub(alignment.value()),
    ]);
    EntropyBucket::new(next)
}

fn bounded_integral(previous: ResidualBucket, residual: u16) -> ResidualBucket {
    let integrated = previous
        .value()
        .saturating_add(residual / 6)
        .saturating_sub(8);
    ResidualBucket::new(integrated)
}

fn bounded_residual(measurement: u16, alignment: SupportBucket) -> u16 {
    average_u16(&[measurement, 1000_u16.saturating_sub(alignment.value())])
}

fn alignment(left: u16, right: u16) -> u16 {
    1000_u16.saturating_sub(left.abs_diff(right))
}

fn average_u16(values: &[u16]) -> u16 {
    let sum: u32 = values.iter().map(|value| u32::from(*value)).sum();
    let len = u32::try_from(values.len()).expect("slice length fits u32");
    u16::try_from(sum / len).expect("average fits u16")
}

fn utilization_pressure(capacity: u32, available: u32) -> u16 {
    if capacity == 0 {
        return 0;
    }
    let used = capacity.saturating_sub(available.min(capacity));
    let scaled = (u64::from(used) * 1000) / u64::from(capacity);
    u16::try_from(scaled).expect("scaled utilization fits u16")
}

fn byte_pressure(capacity: u64, available: u64) -> u16 {
    if capacity == 0 {
        return 0;
    }
    let used = capacity.saturating_sub(available.min(capacity));
    let scaled = (used.saturating_mul(1000)) / capacity;
    u16::try_from(scaled).unwrap_or(u16::MAX).min(1000)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        ByteCount, Configuration, ControllerId, Environment, HoldItemCount, MaintenanceWorkBudget,
        NodeBuilder, NodeId, NodeProfileBuilder, NodeStateBuilder, RatioPermille, RelayWorkBudget,
        Tick,
    };

    use super::*;
    use crate::state::{
        CorridorBeliefEnvelope, DestinationFieldState, DestinationKey, DestinationPosterior,
        HopBand,
    };

    fn sample_destination(
        support: u16,
        entropy: u16,
        congestion: u16,
        retention: u16,
    ) -> DestinationFieldState {
        let mut state = DestinationFieldState::new(DestinationKey::Node(NodeId([9; 32])), Tick(1));
        state.posterior = DestinationPosterior {
            usability_entropy: EntropyBucket::new(entropy),
            top_corridor_mass: SupportBucket::new(support),
            ..DestinationPosterior::default()
        };
        state.corridor_belief = CorridorBeliefEnvelope {
            expected_hop_band: HopBand::new(1, 3),
            delivery_support: SupportBucket::new(support),
            congestion_penalty: EntropyBucket::new(congestion),
            retention_affinity: SupportBucket::new(retention),
            validity_window: state.corridor_belief.validity_window,
        };
        state
    }

    #[test]
    fn mean_field_compression_uses_posterior_and_local_evidence() {
        let destinations = [
            sample_destination(850, 150, 300, 250),
            sample_destination(900, 100, 250, 300),
        ];
        let mean_field = compress_mean_field(
            destinations.iter(),
            ControlMeasurements::new(250, 200, 300, 100, 100),
        );
        assert!(mean_field.field_strength.value() >= 700);
        assert!(mean_field.congestion_alignment.value() >= 900);
        assert!(mean_field.retention_alignment.value() >= 900);
    }

    #[test]
    fn control_state_updates_are_bounded_and_accumulative() {
        let measurements = ControlMeasurements::new(900, 800, 700, 600, 500);
        let mean_field = MeanFieldState {
            relay_alignment: SupportBucket::new(100),
            congestion_alignment: SupportBucket::new(100),
            retention_alignment: SupportBucket::new(150),
            risk_alignment: SupportBucket::new(200),
            field_strength: SupportBucket::new(150),
        };
        let control = update_control_state(&ControlState::default(), &mean_field, measurements);
        assert!(control.congestion_price.value() > 0);
        assert!(control.congestion_error_integral.value() > 0);
        assert!(control.risk_price.value() <= 1000);
    }

    #[test]
    fn topology_measurements_capture_local_resource_pressure() {
        let local_node = NodeId([1; 32]);
        let local_node_object = NodeBuilder::new(
            ControllerId([7; 32]),
            NodeProfileBuilder::new()
                .with_connection_limits(8, 4, 4, 4)
                .with_work_budgets(RelayWorkBudget(8), MaintenanceWorkBudget(4))
                .with_hold_limits(HoldItemCount(4), ByteCount(1000))
                .build(),
            NodeStateBuilder::new()
                .with_available_connections(2, Tick(1))
                .with_hold_capacity(ByteCount(250), Tick(1))
                .with_relay_budget(
                    RelayWorkBudget(8),
                    RatioPermille(700),
                    jacquard_core::DurationMs(100),
                    Tick(1),
                )
                .build(),
        )
        .build();
        let topology = Configuration {
            epoch: jacquard_core::RouteEpoch(1),
            nodes: BTreeMap::from([(local_node, local_node_object)]),
            links: BTreeMap::new(),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(250),
                contention_permille: RatioPermille(600),
            },
        };
        let measurements = ControlMeasurements::from_topology(&topology, local_node);
        assert_eq!(measurements.congestion_pressure, 600);
        assert_eq!(measurements.relay_pressure, 700);
        assert_eq!(measurements.retention_pressure, 750);
        assert!(measurements.risk_pressure >= 250);
    }

    #[test]
    fn stable_sparse_measurements_keep_sparse_regime() {
        let measurements = ControlMeasurements::new(100, 120, 90, 80, 70);
        let mean_field = MeanFieldState {
            relay_alignment: SupportBucket::new(900),
            congestion_alignment: SupportBucket::new(900),
            retention_alignment: SupportBucket::new(900),
            risk_alignment: SupportBucket::new(900),
            field_strength: SupportBucket::new(900),
        };
        let control = update_control_state(&ControlState::default(), &mean_field, measurements);
        let regime = observe_regime(
            &RegimeObserverState::default(),
            &mean_field,
            &control,
            measurements,
            Tick(1),
        );
        assert_eq!(regime.current, OperatingRegime::Sparse);
    }

    #[test]
    fn sustained_congestion_triggers_regime_transition() {
        let measurements = ControlMeasurements::new(900, 700, 200, 200, 100);
        let mean_field = MeanFieldState {
            relay_alignment: SupportBucket::new(700),
            congestion_alignment: SupportBucket::new(900),
            retention_alignment: SupportBucket::new(300),
            risk_alignment: SupportBucket::new(600),
            field_strength: SupportBucket::new(450),
        };
        let control = update_control_state(&ControlState::default(), &mean_field, measurements);
        let previous = RegimeObserverState {
            regime_error_residual: ResidualBucket::new(680),
            ..RegimeObserverState::default()
        };
        let regime = observe_regime(&previous, &mean_field, &control, measurements, Tick(5));
        assert_eq!(regime.current, OperatingRegime::Congested);
    }

    #[test]
    fn retention_regime_prefers_retention_biased_posture() {
        let measurements = ControlMeasurements::new(300, 300, 900, 200, 100);
        let mean_field = MeanFieldState {
            relay_alignment: SupportBucket::new(600),
            congestion_alignment: SupportBucket::new(700),
            retention_alignment: SupportBucket::new(950),
            risk_alignment: SupportBucket::new(750),
            field_strength: SupportBucket::new(700),
        };
        let control = update_control_state(&ControlState::default(), &mean_field, measurements);
        let regime = RegimeObserverState {
            current: OperatingRegime::RetentionFavorable,
            current_regime_score: SupportBucket::new(900),
            ..RegimeObserverState::default()
        };
        let posture = choose_posture(
            &PostureControllerState::default(),
            &regime,
            &mean_field,
            &control,
            Tick(5),
        );
        assert_eq!(posture.current, RoutingPosture::RetentionBiased);
    }

    #[test]
    fn dwell_time_prevents_immediate_regime_oscillation() {
        let measurements = ControlMeasurements::new(950, 300, 100, 100, 100);
        let mean_field = MeanFieldState {
            relay_alignment: SupportBucket::new(500),
            congestion_alignment: SupportBucket::new(950),
            retention_alignment: SupportBucket::new(500),
            risk_alignment: SupportBucket::new(700),
            field_strength: SupportBucket::new(500),
        };
        let control = update_control_state(&ControlState::default(), &mean_field, measurements);
        let previous = RegimeObserverState {
            current: OperatingRegime::Sparse,
            dwell_until_tick: Tick(10),
            regime_error_residual: ResidualBucket::new(680),
            ..RegimeObserverState::default()
        };
        let regime = observe_regime(&previous, &mean_field, &control, measurements, Tick(5));
        assert_eq!(regime.current, OperatingRegime::Sparse);
    }

    #[test]
    fn posture_hysteresis_prevents_one_tick_flapping() {
        let measurements = ControlMeasurements::new(200, 200, 850, 100, 100);
        let mean_field = MeanFieldState {
            relay_alignment: SupportBucket::new(700),
            congestion_alignment: SupportBucket::new(650),
            retention_alignment: SupportBucket::new(900),
            risk_alignment: SupportBucket::new(800),
            field_strength: SupportBucket::new(650),
        };
        let control = update_control_state(&ControlState::default(), &mean_field, measurements);
        let posture = choose_posture(
            &PostureControllerState {
                current: RoutingPosture::Structured,
                last_transition_tick: Tick(5),
                ..PostureControllerState::default()
            },
            &RegimeObserverState {
                current: OperatingRegime::RetentionFavorable,
                current_regime_score: SupportBucket::new(900),
                ..RegimeObserverState::default()
            },
            &mean_field,
            &control,
            Tick(6),
        );
        assert_eq!(posture.current, RoutingPosture::Structured);
    }
}
