use std::collections::BTreeMap;

use jacquard_core::{
    ByteCount, Configuration, Link, NodeId, Observation, RatioPermille, RouteEpoch,
    Tick,
};
use jacquard_traits::RoutingEnvironmentModel;

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
    Partition {
        left: NodeId,
        right: NodeId,
    },
    MobilityRelink {
        left: NodeId,
        from_right: NodeId,
        to_right: NodeId,
        link: Link,
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
                    evidence_class:
                        jacquard_core::RoutingEvidenceClass::DirectObservation,
                    origin_authentication:
                        jacquard_core::OriginAuthenticationClass::Controlled,
                    observed_at_tick: at_tick,
                },
                applied,
            );
        };

        for hook in hooks {
            apply_hook(&mut next, hook, at_tick);
            applied.push(AppliedEnvironmentHook { at_tick, hook: hook.clone() });
        }
        next.epoch = RouteEpoch(next.epoch.0.saturating_add(1));

        (
            Observation {
                value: next,
                source_class: jacquard_core::FactSourceClass::Local,
                evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
                origin_authentication:
                    jacquard_core::OriginAuthenticationClass::Controlled,
                observed_at_tick: at_tick,
            },
            applied,
        )
    }
}

fn apply_hook(
    configuration: &mut Configuration,
    hook: &EnvironmentHook,
    at_tick: Tick,
) {
    match hook {
        | EnvironmentHook::ReplaceTopology { configuration: replacement } => {
            *configuration = replacement.clone();
        },
        | EnvironmentHook::MediumDegradation { left, right, confidence, loss } => {
            if let Some(link) = configuration.links.get_mut(&(*left, *right)) {
                link.state.delivery_confidence_permille =
                    jacquard_core::Belief::certain(*confidence, at_tick);
                link.state.loss_permille = *loss;
            }
        },
        | EnvironmentHook::Partition { left, right } => {
            configuration.links.remove(&(*left, *right));
        },
        | EnvironmentHook::MobilityRelink { left, from_right, to_right, link } => {
            configuration.links.remove(&(*left, *from_right));
            configuration.links.insert((*left, *to_right), link.clone());
        },
        | EnvironmentHook::IntrinsicLimit {
            node_id,
            connection_count_max,
            hold_capacity_bytes_max,
        } => {
            if let Some(node) = configuration.nodes.get_mut(node_id) {
                node.profile.connection_count_max = *connection_count_max;
                node.profile.hold_capacity_bytes_max = *hold_capacity_bytes_max;
            }
        },
    }
}
