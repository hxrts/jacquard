use std::collections::BTreeMap;

use jacquard_core::{
    ByteCount, Configuration, Link, NodeId, Observation, RatioPermille, RouteEpoch, Tick,
};
use jacquard_traits::RoutingEnvironmentModel;

// The simulator models links as directed edges keyed by `(from, to)` in
// `Configuration.links`. That means environment hooks can legitimately mutate
// one direction without touching the reverse edge when both are present.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvironmentHook {
    ReplaceTopology {
        configuration: Configuration,
    },
    MediumDegradation {
        left: NodeId,
        right: NodeId,
        confidence: RatioPermille,
        loss: RatioPermille,
    },
    AsymmetricDegradation {
        left: NodeId,
        right: NodeId,
        forward_confidence: RatioPermille,
        forward_loss: RatioPermille,
        reverse_confidence: RatioPermille,
        reverse_loss: RatioPermille,
    },
    Partition {
        left: NodeId,
        right: NodeId,
    },
    CascadePartition {
        cuts: Vec<(NodeId, NodeId)>,
    },
    MobilityRelink {
        left: NodeId,
        from_right: NodeId,
        to_right: NodeId,
        link: Box<Link>,
    },
    IntrinsicLimit {
        node_id: NodeId,
        connection_count_max: u32,
        hold_capacity_bytes_max: ByteCount,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledEnvironmentHook {
    pub at_tick: Tick,
    pub hook: EnvironmentHook,
}

impl ScheduledEnvironmentHook {
    #[must_use]
    pub fn new(at_tick: Tick, hook: EnvironmentHook) -> Self {
        Self { at_tick, hook }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliedEnvironmentHook {
    pub at_tick: Tick,
    pub hook: EnvironmentHook,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptedEnvironmentModel {
    hooks_by_tick: BTreeMap<Tick, Vec<EnvironmentHook>>,
}

impl ScriptedEnvironmentModel {
    #[must_use]
    pub fn new(hooks: Vec<ScheduledEnvironmentHook>) -> Self {
        let mut hooks_by_tick = BTreeMap::new();
        for scheduled in hooks {
            hooks_by_tick
                .entry(scheduled.at_tick)
                .or_insert_with(Vec::new)
                .push(scheduled.hook);
        }
        Self { hooks_by_tick }
    }

    #[must_use]
    pub fn is_quiescent_after(&self, tick: Tick) -> bool {
        self.hooks_by_tick
            .keys()
            .all(|scheduled| *scheduled <= tick)
    }
}

impl RoutingEnvironmentModel for ScriptedEnvironmentModel {
    type EnvironmentArtifact = AppliedEnvironmentHook;

    fn advance_environment(
        &self,
        configuration: &Configuration,
        at_tick: Tick,
    ) -> (Observation<Configuration>, Vec<Self::EnvironmentArtifact>) {
        let mut next = configuration.clone();
        let mut applied = Vec::new();
        let Some(hooks) = self.hooks_by_tick.get(&at_tick) else {
            return (
                Observation {
                    value: next,
                    source_class: jacquard_core::FactSourceClass::Local,
                    evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
                    origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
                    observed_at_tick: at_tick,
                },
                applied,
            );
        };

        for hook in hooks {
            apply_hook(&mut next, hook, at_tick);
            applied.push(AppliedEnvironmentHook {
                at_tick,
                hook: hook.clone(),
            });
        }
        next.epoch = RouteEpoch(next.epoch.0.saturating_add(1));

        (
            Observation {
                value: next,
                source_class: jacquard_core::FactSourceClass::Local,
                evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
                origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
                observed_at_tick: at_tick,
            },
            applied,
        )
    }
}

// long-block-exception: one environment hook dispatcher keeps the directed-link
// simulation semantics explicit and replay-auditable in a single match.
fn apply_hook(configuration: &mut Configuration, hook: &EnvironmentHook, at_tick: Tick) {
    match hook {
        EnvironmentHook::ReplaceTopology {
            configuration: replacement,
        } => {
            *configuration = replacement.clone();
        }
        EnvironmentHook::MediumDegradation {
            left,
            right,
            confidence,
            loss,
        } => {
            apply_degradation(configuration, *left, *right, *confidence, *loss, at_tick);
        }
        EnvironmentHook::AsymmetricDegradation {
            left,
            right,
            forward_confidence,
            forward_loss,
            reverse_confidence,
            reverse_loss,
        } => {
            apply_degradation(
                configuration,
                *left,
                *right,
                *forward_confidence,
                *forward_loss,
                at_tick,
            );
            apply_degradation(
                configuration,
                *right,
                *left,
                *reverse_confidence,
                *reverse_loss,
                at_tick,
            );
        }
        EnvironmentHook::Partition { left, right } => {
            configuration.links.remove(&(*left, *right));
        }
        EnvironmentHook::CascadePartition { cuts } => {
            for (left, right) in cuts {
                configuration.links.remove(&(*left, *right));
            }
        }
        EnvironmentHook::MobilityRelink {
            left,
            from_right,
            to_right,
            link,
        } => {
            configuration.links.remove(&(*left, *from_right));
            configuration
                .links
                .insert((*left, *to_right), link.as_ref().clone());
        }
        EnvironmentHook::IntrinsicLimit {
            node_id,
            connection_count_max,
            hold_capacity_bytes_max,
        } => {
            if let Some(node) = configuration.nodes.get_mut(node_id) {
                node.profile.connection_count_max = *connection_count_max;
                node.profile.hold_capacity_bytes_max = *hold_capacity_bytes_max;
            }
        }
    }
}

fn apply_degradation(
    configuration: &mut Configuration,
    left: NodeId,
    right: NodeId,
    confidence: RatioPermille,
    loss: RatioPermille,
    at_tick: Tick,
) {
    if let Some(link) = configuration.links.get_mut(&(left, right)) {
        link.state.delivery_confidence_permille =
            jacquard_core::Belief::certain(confidence, at_tick);
        link.state.loss_permille = loss;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{Configuration, Environment, RatioPermille, RouteEpoch, Tick};
    use jacquard_traits::RoutingEnvironmentModel;

    use super::{EnvironmentHook, ScriptedEnvironmentModel};
    use crate::topology;

    fn node(byte: u8) -> jacquard_core::NodeId {
        jacquard_core::NodeId([byte; 32])
    }

    fn configuration() -> Configuration {
        Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([
                (node(1), topology::node(1).all_engines().build()),
                (node(2), topology::node(2).all_engines().build()),
                (node(3), topology::node(3).all_engines().build()),
            ]),
            links: BTreeMap::from([
                ((node(1), node(2)), topology::link(2).build()),
                ((node(2), node(1)), topology::link(1).build()),
                ((node(2), node(3)), topology::link(3).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        }
    }

    #[test]
    fn asymmetric_degradation_mutates_each_direction_independently() {
        let environment =
            ScriptedEnvironmentModel::new(vec![super::ScheduledEnvironmentHook::new(
                Tick(2),
                EnvironmentHook::AsymmetricDegradation {
                    left: node(1),
                    right: node(2),
                    forward_confidence: RatioPermille(300),
                    forward_loss: RatioPermille(700),
                    reverse_confidence: RatioPermille(900),
                    reverse_loss: RatioPermille(100),
                },
            )]);

        let (next, artifacts) = environment.advance_environment(&configuration(), Tick(2));

        assert_eq!(artifacts.len(), 1);
        assert_eq!(
            next.value.links[&(node(1), node(2))]
                .state
                .delivery_confidence_permille
                .value(),
            Some(RatioPermille(300))
        );
        assert_eq!(
            next.value.links[&(node(1), node(2))].state.loss_permille.0,
            700
        );
        assert_eq!(
            next.value.links[&(node(2), node(1))]
                .state
                .delivery_confidence_permille
                .value(),
            Some(RatioPermille(900))
        );
        assert_eq!(
            next.value.links[&(node(2), node(1))].state.loss_permille.0,
            100
        );
    }

    #[test]
    fn cascade_partition_removes_multiple_directed_edges_in_one_round() {
        let environment =
            ScriptedEnvironmentModel::new(vec![super::ScheduledEnvironmentHook::new(
                Tick(2),
                EnvironmentHook::CascadePartition {
                    cuts: vec![(node(1), node(2)), (node(2), node(3))],
                },
            )]);

        let (next, artifacts) = environment.advance_environment(&configuration(), Tick(2));

        assert_eq!(artifacts.len(), 1);
        assert!(!next.value.links.contains_key(&(node(1), node(2))));
        assert!(!next.value.links.contains_key(&(node(2), node(3))));
        assert!(next.value.links.contains_key(&(node(2), node(1))));
    }

    #[test]
    fn quiescent_after_only_counts_future_ticks() {
        let environment =
            ScriptedEnvironmentModel::new(vec![super::ScheduledEnvironmentHook::new(
                Tick(3),
                EnvironmentHook::ReplaceTopology {
                    configuration: configuration(),
                },
            )]);

        assert!(!environment.is_quiescent_after(Tick(2)));
        assert!(environment.is_quiescent_after(Tick(3)));
    }
}
