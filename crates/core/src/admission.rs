//! Route family capabilities, admission checks, candidates, and witnesses.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    AdaptiveRoutingProfile, RouteConnectivityClass, RouteCost, RouteEpoch, RouteId,
    RoutePrivacyClass, RoutingFamilyId, RoutingObjective, Tick, TransportClass,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingFamilyCapabilities {
    pub family: RoutingFamilyId,
    pub max_privacy: RoutePrivacyClass,
    pub max_connectivity: RouteConnectivityClass,
    pub repair_support: RepairSupport,
    pub hold_support: HoldSupport,
    pub decidable_admission: DecidableSupport,
    pub quantitative_bounds: QuantitativeBoundSupport,
    pub reconfiguration_support: ReconfigurationSupport,
    pub route_shape_visibility: RouteShapeVisibility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RepairSupport {
    Unsupported,
    Supported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HoldSupport {
    Unsupported,
    Supported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DecidableSupport {
    Unsupported,
    Supported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum QuantitativeBoundSupport {
    Unsupported,
    ProductiveOnly,
    ProductiveAndSchedulerLifted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReconfigurationSupport {
    ReplaceOnly,
    LinkAndDelegate,
    FamilyDefined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteShapeVisibility {
    Explicit,
    Opaque,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingAdmissionProfile {
    pub delivery_model: DeliveryModelClass,
    pub failure_model: FailureModelClass,
    pub runtime_envelope: RuntimeEnvelopeClass,
    pub node_density_class: NodeDensityClass,
    pub connectivity_regime: ConnectivityRegime,
    pub adversary_regime: AdversaryRegime,
    pub claim_strength: ClaimStrength,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeliveryModelClass {
    FifoPerLink,
    CausalPerNeighborhood,
    LossyBestEffort,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FailureModelClass {
    Benign,
    CrashStop,
    ByzantineInterface,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RuntimeEnvelopeClass {
    Canonical,
    EnvelopeAdmitted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NodeDensityClass {
    Sparse,
    Moderate,
    Dense,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConnectivityRegime {
    Stable,
    HighChurn,
    PartitionProne,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AdversaryRegime {
    Cooperative,
    BenignUntrusted,
    ActiveAdversarial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ClaimStrength {
    ExactUnderAssumptions,
    ConservativeUnderProfile,
    InterfaceOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteSummary {
    pub family: RoutingFamilyId,
    pub privacy: RoutePrivacyClass,
    pub connectivity: RouteConnectivityClass,
    pub transport_mix: Vec<TransportClass>,
    pub hop_count_hint: Option<u8>,
    pub expires_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCandidate {
    pub summary: RouteSummary,
    pub witness: RouteWitness,
    pub backend_ref: BackendRouteRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteAdmissionCheck {
    pub admissible: bool,
    pub profile: RoutingAdmissionProfile,
    pub productive_step_bound: Option<u32>,
    pub total_step_bound: Option<u32>,
    pub route_cost: RouteCost,
    pub rejection_reason: Option<RouteAdmissionRejection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Error, Serialize, Deserialize)]
pub enum RouteAdmissionRejection {
    #[error("privacy floor unsatisfied")]
    PrivacyFloorUnsatisfied,
    #[error("delivery model unsupported")]
    DeliveryModelUnsupported,
    #[error("capacity exceeded")]
    CapacityExceeded,
    #[error("budget exceeded")]
    BudgetExceeded,
    #[error("branching infeasible")]
    BranchingInfeasible,
    #[error("crash tolerance unsatisfied")]
    CrashToleranceUnsatisfied,
    #[error("reconfiguration unsafe")]
    ReconfigurationUnsafe,
    #[error("backend unavailable")]
    BackendUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteAdmission {
    pub route_id: RouteId,
    pub objective: RoutingObjective,
    pub profile: AdaptiveRoutingProfile,
    pub admission_check: RouteAdmissionCheck,
    pub summary: RouteSummary,
    pub witness: RouteWitness,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteWitness {
    pub objective_privacy: RoutePrivacyClass,
    pub delivered_privacy: RoutePrivacyClass,
    pub objective_connectivity: RouteConnectivityClass,
    pub delivered_connectivity: RouteConnectivityClass,
    pub admission_profile: RoutingAdmissionProfile,
    pub topology_epoch: RouteEpoch,
    pub degradation_reason: Option<DegradationReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DegradationReason {
    SparseTopology,
    LinkInstability,
    CapacityPressure,
    PartitionRisk,
    BackendUnavailable,
    PolicyPreference,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendRouteRef {
    pub family: RoutingFamilyId,
    pub opaque_id: Vec<u8>,
}
