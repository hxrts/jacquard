//! Route family capabilities, admission checks, candidates, and witnesses.

use contour_macros::public_model;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    AdaptiveRoutingProfile, BackendRouteId, Belief, Estimate, Limit, RouteConnectivityProfile,
    RouteCost, RouteEpoch, RouteEstimate, RouteFamilyId, RouteId, RouteProtectionClass,
    RoutingObjective, TimeWindow, TransportProtocol,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingFamilyCapabilities {
    pub family: RouteFamilyId,
    pub max_protection: RouteProtectionClass,
    pub max_connectivity: RouteConnectivityProfile,
    pub repair_support: RepairSupport,
    pub hold_support: HoldSupport,
    pub decidable_admission: DecidableSupport,
    pub quantitative_bounds: QuantitativeBoundSupport,
    pub reconfiguration_support: ReconfigurationSupport,
    pub route_shape_visibility: RouteShapeVisibility,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RepairSupport {
    Unsupported,
    Supported,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HoldSupport {
    Unsupported,
    Supported,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DecidableSupport {
    Unsupported,
    Supported,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum QuantitativeBoundSupport {
    Unsupported,
    ProductiveOnly,
    ProductiveAndSchedulerLifted,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReconfigurationSupport {
    ReplaceOnly,
    LinkAndDelegate,
    FamilyDefined,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteShapeVisibility {
    Explicit,
    Opaque,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Premise profile: delivery, failure, and runtime assumptions under which
/// the admission claim holds. Families declare these, the router compares them.
pub struct RoutingAdmissionProfile {
    pub delivery_assumption: DeliveryAssumptionClass,
    pub failure_model: FailureModelClass,
    pub runtime_envelope: RuntimeEnvelopeClass,
    pub node_density_class: NodeDensityClass,
    pub connectivity_regime: ConnectivityRegime,
    pub adversary_regime: AdversaryRegime,
    pub claim_strength: ClaimStrength,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeliveryAssumptionClass {
    FifoPerLink,
    CausalPerNeighborhood,
    LossyBestEffort,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FailureModelClass {
    Benign,
    CrashStop,
    ByzantineInterface,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RuntimeEnvelopeClass {
    Canonical,
    EnvelopeAdmitted,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NodeDensityClass {
    Sparse,
    Moderate,
    Dense,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConnectivityRegime {
    Stable,
    HighChurn,
    PartitionProne,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AdversaryRegime {
    Cooperative,
    BenignUntrusted,
    ActiveAdversarial,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ClaimStrength {
    ExactUnderAssumptions,
    ConservativeUnderProfile,
    InterfaceOnly,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteSummary {
    pub family: RouteFamilyId,
    pub protection: RouteProtectionClass,
    pub connectivity: RouteConnectivityProfile,
    pub protocol_mix: Vec<TransportProtocol>,
    pub hop_count_hint: Belief<u8>,
    pub valid_for: TimeWindow,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCandidate {
    pub summary: RouteSummary,
    /// Candidate enumeration is observational/advisory. It must not be treated
    /// as proof-bearing admission evidence.
    pub estimate: Estimate<RouteEstimate>,
    pub backend_ref: BackendRouteRef,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteAdmissionCheck {
    pub decision: AdmissionDecision,
    pub profile: RoutingAdmissionProfile,
    pub productive_step_bound: Limit<u32>,
    pub total_step_bound: Limit<u32>,
    pub route_cost: RouteCost,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AdmissionDecision {
    Admissible,
    Rejected(RouteAdmissionRejection),
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Error, Serialize, Deserialize)]
pub enum RouteAdmissionRejection {
    #[error("protection floor unsatisfied")]
    ProtectionFloorUnsatisfied,
    #[error("delivery model unsupported")]
    DeliveryAssumptionUnsupported,
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

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteAdmission {
    pub route_id: RouteId,
    pub objective: RoutingObjective,
    pub profile: AdaptiveRoutingProfile,
    pub admission_check: RouteAdmissionCheck,
    pub summary: RouteSummary,
    pub witness: RouteWitness,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Proof-bearing explanation of what the admitted route actually delivers.
/// If protection was reduced for connectivity, that fact is explicit here.
pub struct RouteWitness {
    pub objective_protection: RouteProtectionClass,
    pub delivered_protection: RouteProtectionClass,
    pub objective_connectivity: RouteConnectivityProfile,
    pub delivered_connectivity: RouteConnectivityProfile,
    pub admission_profile: RoutingAdmissionProfile,
    pub topology_epoch: RouteEpoch,
    pub degradation: RouteDegradation,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteDegradation {
    None,
    Degraded(DegradationReason),
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DegradationReason {
    SparseTopology,
    LinkInstability,
    CapacityPressure,
    PartitionRisk,
    BackendUnavailable,
    PolicyPreference,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Family-owned opaque handle. Contour core never inspects the contents.
/// This is a weak advisory reference and is not a canonical installed-route handle.
pub struct BackendRouteRef {
    pub family: RouteFamilyId,
    pub backend_route_id: BackendRouteId,
}
