//! Routing-engine capabilities, admission checks, candidates, and witnesses.
//!
//! This module defines the shared types that flow across the engine-to-router
//! admission boundary. [`RoutingEngineCapabilities`] declares what one engine
//! can provide. [`RouteCandidate`] is the advisory pre-admission object an
//! engine surfaces for a given objective. [`RouteAdmissionCheck`] records the
//! engine's admission decision plus cost bounds and assumption profile.
//! [`RouteAdmission`] is the proof-bearing artifact produced after a successful
//! admission check; it carries the objective, selected parameters, check
//! result, route summary, and a [`RouteWitness`] that makes the
//! objective-vs-delivered gap explicit.
//!
//! Assumption vocabulary: [`AdmissionAssumptions`] describes the message-flow,
//! failure, density, and adversary regime under which an admission claim holds.
//! [`RouteDegradation`] and [`DegradationReason`] represent cases where the
//! admitted route falls short of the objective protection or connectivity.

use alloc::vec::Vec;
use core::fmt;

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    BackendRouteId, Belief, ConnectivityPosture, Estimate, Limit, RouteCost, RouteEpoch,
    RouteEstimate, RouteId, RouteProtectionClass, RoutingEngineId, RoutingObjective,
    SelectedRoutingParameters, TimeWindow, TransportKind,
};

/// Generates a binary capability enum with `Unsupported` / `Supported`
/// variants, full shared-model derives, and a `Default` impl that returns
/// `Unsupported`.
macro_rules! capability_enum {
    ($name:ident) => {
        #[public_model]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        pub enum $name {
            Unsupported,
            Supported,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::Unsupported
            }
        }
    };
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingEngineCapabilities {
    pub engine: RoutingEngineId,
    pub max_protection: RouteProtectionClass,
    pub max_connectivity: ConnectivityPosture,
    pub repair_support: RepairSupport,
    pub hold_support: HoldSupport,
    pub decidable_admission: DecidableSupport,
    pub quantitative_bounds: QuantitativeBoundSupport,
    pub reconfiguration_support: ReconfigurationSupport,
    pub route_shape_visibility: RouteShapeVisibility,
}

capability_enum!(RepairSupport);
capability_enum!(HoldSupport);
capability_enum!(DecidableSupport);

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
    ExplicitPath,
    CorridorEnvelope,
    NextHopOnly,
    Opaque,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Assumption envelope: message-flow, failure, and runtime assumptions under
/// which the admission claim holds. Routing engines declare these, the router
/// compares them.
pub struct AdmissionAssumptions {
    pub message_flow_assumption: MessageFlowAssumptionClass,
    pub failure_model: FailureModelClass,
    pub runtime_envelope: RuntimeEnvelopeClass,
    pub node_density_class: NodeDensityClass,
    pub connectivity_regime: ConnectivityRegime,
    pub adversary_regime: AdversaryRegime,
    pub claim_strength: ClaimStrength,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessageFlowAssumptionClass {
    BestEffort,
    PerRouteSequenced,
    NeighborhoodCausal,
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
    pub engine: RoutingEngineId,
    pub protection: RouteProtectionClass,
    pub connectivity: ConnectivityPosture,
    pub protocol_mix: Vec<TransportKind>,
    /// Bounded by [`ROUTE_HOP_COUNT_MAX`](crate::ROUTE_HOP_COUNT_MAX).
    pub hop_count_hint: Belief<u8>,
    pub valid_for: TimeWindow,
}

// Advisory only: a RouteCandidate is never proof-bearing evidence.
// RouteAdmission is the proof-bearing counterpart after the admission check.
#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCandidate {
    /// Advisory identity derived deterministically from the backend token.
    /// This is not yet canonical publication proof; the router promotes it
    /// into a `RouteIdentityStamp` only after materialization succeeds.
    pub route_id: RouteId,
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
    pub profile: AdmissionAssumptions,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteAdmissionRejection {
    ProtectionFloorUnsatisfied,
    DeliveryAssumptionUnsupported,
    CapacityExceeded,
    BudgetExceeded,
    BranchingInfeasible,
    CrashToleranceUnsatisfied,
    ReconfigurationUnsafe,
    BackendUnavailable,
}

impl fmt::Display for RouteAdmissionRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtectionFloorUnsatisfied => f.write_str("protection floor unsatisfied"),
            Self::DeliveryAssumptionUnsupported => f.write_str("delivery model unsupported"),
            Self::CapacityExceeded => f.write_str("capacity exceeded"),
            Self::BudgetExceeded => f.write_str("budget exceeded"),
            Self::BranchingInfeasible => f.write_str("branching infeasible"),
            Self::CrashToleranceUnsatisfied => f.write_str("crash tolerance unsatisfied"),
            Self::ReconfigurationUnsafe => f.write_str("reconfiguration unsafe"),
            Self::BackendUnavailable => f.write_str("backend unavailable"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RouteAdmissionRejection {}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Engine's decision record about an objective/candidate pair.
///
/// Admission carries proof-bearing decision artifacts only. Canonical route
/// identity lives in `RouteIdentityStamp`, and the pre-publication advisory
/// route ID lives on `RouteCandidate`.
pub struct RouteAdmission {
    pub backend_ref: BackendRouteRef,
    pub objective: RoutingObjective,
    pub profile: SelectedRoutingParameters,
    pub admission_check: RouteAdmissionCheck,
    pub summary: RouteSummary,
    pub witness: RouteWitness,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Pairs an objective value with the actually-delivered value. Used in
/// `RouteWitness` to make the objective-vs-delivered gap explicit.
pub struct ObjectiveVsDelivered<T> {
    pub objective: T,
    pub delivered: T,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Proof-bearing explanation of what the admitted route actually delivers.
/// If protection was reduced for connectivity, that fact is explicit here.
pub struct RouteWitness {
    pub protection: ObjectiveVsDelivered<RouteProtectionClass>,
    pub connectivity: ObjectiveVsDelivered<ConnectivityPosture>,
    pub admission_profile: AdmissionAssumptions,
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
/// Engine-owned opaque handle. Jacquard core never inspects the contents.
/// This is a weak advisory reference and is not a canonical installed-route
/// handle.
pub struct BackendRouteRef {
    pub engine: RoutingEngineId,
    pub backend_route_id: BackendRouteId,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify that capability_enum! emits the expected shape:
    // both variants, Default == Unsupported, Clone, PartialEq, PartialOrd.
    #[test]
    #[allow(clippy::clone_on_copy)]
    fn capability_enum_repair_support_shape() {
        assert_eq!(RepairSupport::default(), RepairSupport::Unsupported);
        assert_ne!(RepairSupport::Unsupported, RepairSupport::Supported);
        assert!(RepairSupport::Unsupported < RepairSupport::Supported);
        // Explicit `.clone()` on a Copy type verifies Clone is still derived
        // by the `capability_enum!` macro expansion.
        assert_eq!(RepairSupport::Supported.clone(), RepairSupport::Supported);
    }

    #[test]
    fn capability_enum_hold_support_shape() {
        assert_eq!(HoldSupport::default(), HoldSupport::Unsupported);
        assert!(HoldSupport::Unsupported < HoldSupport::Supported);
    }

    #[test]
    fn capability_enum_decidable_support_shape() {
        assert_eq!(DecidableSupport::default(), DecidableSupport::Unsupported);
        assert!(DecidableSupport::Unsupported < DecidableSupport::Supported);
    }

    #[test]
    fn route_shape_visibility_orders_by_specificity() {
        assert!(RouteShapeVisibility::ExplicitPath < RouteShapeVisibility::CorridorEnvelope);
        assert!(RouteShapeVisibility::CorridorEnvelope < RouteShapeVisibility::NextHopOnly);
        assert!(RouteShapeVisibility::NextHopOnly < RouteShapeVisibility::Opaque);
    }

    #[test]
    fn route_shape_visibility_serializes_new_variants() {
        let encoded = serde_json::to_string(&RouteShapeVisibility::NextHopOnly)
            .expect("serialize next-hop visibility");
        assert_eq!(encoded, "\"NextHopOnly\"");
        let decoded: RouteShapeVisibility =
            serde_json::from_str(&encoded).expect("deserialize next-hop visibility");
        assert_eq!(decoded, RouteShapeVisibility::NextHopOnly);
    }
}
