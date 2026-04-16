//! Declarative maintained-family catalogs for route-visible experiments.

use jacquard_core::SimulationSeed;

use super::{
    regime, ExperimentParameterSet, JacquardScenario, RegimeDescriptor, RegimeFields,
    ScriptedEnvironmentModel,
};

pub(in crate::experiments) mod batman;
pub(in crate::experiments) mod comparative;

pub(in crate::experiments) type FamilyBuilder =
    fn(&ExperimentParameterSet, SimulationSeed) -> (JacquardScenario, ScriptedEnvironmentModel);

#[derive(Clone, Copy)]
pub(in crate::experiments) struct FamilyDescriptor {
    pub family_id: &'static str,
    pub regime: RegimeFields<'static>,
    pub builder: FamilyBuilder,
}

pub(in crate::experiments) fn materialize_families(
    descriptors: &[FamilyDescriptor],
) -> Vec<(&'static str, RegimeDescriptor, FamilyBuilder)> {
    descriptors
        .iter()
        .map(|descriptor| {
            (
                descriptor.family_id,
                regime(descriptor.regime),
                descriptor.builder,
            )
        })
        .collect()
}
