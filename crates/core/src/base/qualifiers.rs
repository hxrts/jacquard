//! Explicit wrappers for bounded values, observations, estimates, and facts.

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{RatioPermille, Tick};

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Limit<T> {
    Unbounded,
    Bounded(T),
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Belief<T> {
    Absent,
    Estimated(Estimate<T>),
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Belief update derived from one or more observations.
pub struct Estimate<T> {
    pub value: T,
    pub confidence_permille: RatioPermille,
    pub updated_at_tick: Tick,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FactSourceClass {
    Local,
    Remote,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RoutingEvidenceClass {
    DirectObservation,
    PeerClaim,
    AdmissionWitnessed,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OriginAuthenticationClass {
    Controlled,
    Authenticated,
    Unauthenticated,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Raw local observation or received report with provenance attached.
pub struct Observation<T> {
    pub value: T,
    pub source_class: FactSourceClass,
    pub evidence_class: RoutingEvidenceClass,
    pub origin_authentication: OriginAuthenticationClass,
    pub observed_at_tick: Tick,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Stronger basis for a routing fact the system treats as established.
pub enum FactBasis {
    Observed,
    Estimated,
    Admitted,
    Published,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Definitive routing truth established from observations, estimates, or publication.
pub struct Fact<T> {
    pub value: T,
    pub basis: FactBasis,
    pub established_at_tick: Tick,
}
