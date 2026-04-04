#![forbid(unsafe_code)]

use core::fmt;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const ROUTE_HOP_COUNT_MAX: u8 = 16;
pub const PROVIDER_CANDIDATE_COUNT_MAX: u16 = 32;
pub const SERVICE_ENDPOINT_COUNT_MAX: u16 = 16;
pub const ROUTE_PAYLOAD_BYTES_MAX: u32 = 64 * 1024;
pub const REPAIR_STEP_COUNT_MAX: u8 = 8;
pub const ENVELOPE_SIZE: usize = 1024;

macro_rules! bytes_newtype {
    ($name:ident, $size:expr) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            Default,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
        )]
        pub struct $name(pub [u8; $size]);
    };
}

bytes_newtype!(NodeId, 32);
bytes_newtype!(ControllerId, 32);
bytes_newtype!(NeighborhoodId, 16);
bytes_newtype!(HomeId, 16);
bytes_newtype!(ClusterId, 16);
bytes_newtype!(GatewayDomainId, 16);
bytes_newtype!(RouteId, 16);
bytes_newtype!(RouteOperationId, 16);
bytes_newtype!(PathId, 16);
bytes_newtype!(Blake3Digest, 32);

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ServiceId(pub Vec<u8>);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Tick(pub u64);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct DurationMs(pub u32);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct OrderStamp(pub u64);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct RouteEpoch(pub u64);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct RatioPermille(pub u16);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct PriorityPoints(pub u32);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct HealthScore(pub u32);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct PenaltyPoints(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start_tick: Tick,
    pub end_tick: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    pub attempt_count_max: u32,
    pub initial_backoff_ms: DurationMs,
    pub backoff_multiplier_permille: RatioPermille,
    pub backoff_ms_max: DurationMs,
    pub overall_deadline_ms: DurationMs,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestinationId {
    Node(NodeId),
    Service(ServiceId),
    Gateway(GatewayDomainId),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeBinding {
    pub node_id: NodeId,
    pub controller_id: ControllerId,
    pub binding_epoch: RouteEpoch,
    pub proof: NodeBindingProof,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeBindingProof {
    Signature {
        key_id: [u8; 32],
        signature_bytes: Vec<u8>,
    },
    Opaque(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RoutingFamilyId {
    Mesh,
    External { name: String, contract_id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ServiceFamily {
    Discover,
    Establish,
    Move,
    Repair,
    Hold,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TransportProtocol {
    BleGatt,
    BleL2cap,
    WifiAware,
    WifiLan,
    Quic,
    TcpRelay,
    Custom(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TransportClass {
    Proximity,
    LocalArea,
    Backbone,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EndpointAddress {
    Ble {
        device_id: Vec<u8>,
        service_uuid: [u8; 16],
    },
    Ip {
        host: String,
        port: u16,
    },
    Opaque(Vec<u8>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeliverySemantics {
    UnorderedBestEffort,
    ReliableOrdered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LinkRuntimeState {
    Active,
    Degraded,
    Suspended,
    Faulted,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LinkEndpoint {
    pub protocol: TransportProtocol,
    pub class: TransportClass,
    pub address: EndpointAddress,
    pub mtu_bytes: u32,
    pub delivery_semantics: DeliverySemantics,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceDescriptor {
    pub provider_node_id: NodeId,
    pub controller_id: ControllerId,
    pub family: ServiceFamily,
    pub endpoints: Vec<LinkEndpoint>,
    pub routing_families: Vec<RoutingFamilyId>,
    pub scope: ServiceScope,
    pub valid_for: TimeWindow,
    pub capacity: Option<CapacityHint>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceScope {
    Neighborhood(NeighborhoodId),
    Home(HomeId),
    Cluster(ClusterId),
    GatewayDomain(GatewayDomainId),
    Introduction { scope_token: Vec<u8> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityHint {
    pub saturation_permille: RatioPermille,
    pub repair_capacity: Option<u32>,
    pub hold_capacity_bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySnapshot {
    pub epoch: RouteEpoch,
    pub nodes: BTreeMap<NodeId, TopologyNodeObservation>,
    pub links: BTreeMap<(NodeId, NodeId), TopologyLinkObservation>,
    pub last_updated_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyNodeObservation {
    pub controller_id: ControllerId,
    pub services: Vec<ServiceDescriptor>,
    pub endpoints: Vec<LinkEndpoint>,
    pub trust_class: PeerTrustClass,
    pub last_seen_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyLinkObservation {
    pub endpoint: LinkEndpoint,
    pub state: LinkRuntimeState,
    pub median_rtt: DurationMs,
    pub loss_permille: RatioPermille,
    pub last_seen_at: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RoutingEvidenceClass {
    Observed,
    ControllerAuthenticated,
    AdmissionWitnessed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PeerTrustClass {
    LocalOwned,
    ControllerBound,
    UnauthenticatedObserved,
    LowTrustRelay,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingFact<T> {
    pub value: T,
    pub evidence_class: RoutingEvidenceClass,
    pub trust_class: PeerTrustClass,
    pub observed_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingObjective {
    pub destination: DestinationId,
    pub service_family: ServiceFamily,
    pub target_privacy: RoutePrivacyClass,
    pub privacy_floor: RoutePrivacyClass,
    pub target_connectivity: RouteConnectivityClass,
    pub hold_fallback_policy: HoldFallbackPolicy,
    pub latency_budget: Option<DurationMs>,
    pub privacy_priority: PriorityPoints,
    pub connectivity_priority: PriorityPoints,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HoldFallbackPolicy {
    Forbidden,
    Allowed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RoutePrivacyClass {
    None,
    LinkConfidential,
    TopologyObscured,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteConnectivityClass {
    BestEffort,
    Repairable,
    PartitionTolerant,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingObservations {
    pub reachable_neighbor_count: u32,
    pub route_family_count: u32,
    pub median_rtt: DurationMs,
    pub loss_permille: RatioPermille,
    pub topology_churn_permille: RatioPermille,
    pub congestion_penalty_points: PenaltyPoints,
    pub partition_risk_permille: RatioPermille,
    pub direct_reachability_score: HealthScore,
    pub available_hold_capacity_bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdaptiveRoutingProfile {
    pub selected_privacy: RoutePrivacyClass,
    pub selected_connectivity: RouteConnectivityClass,
    pub deployment_profile: DeploymentProfileId,
    pub diversity_floor: u8,
    pub family_fallback_policy: FamilyFallbackPolicy,
    pub route_replacement_policy: RouteReplacementPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FamilyFallbackPolicy {
    Forbidden,
    Allowed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteReplacementPolicy {
    Forbidden,
    Allowed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeploymentProfileId {
    SparseLowPower,
    DenseInteractive,
    PartitionTolerantField,
    HostileRelay,
}

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteOrderingKey {
    pub priority: PriorityPoints,
    pub topology_epoch: RouteEpoch,
    pub tie_break: OrderStamp,
}

impl Ord for RouteOrderingKey {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (self.priority, self.topology_epoch, self.tie_break).cmp(&(
            other.priority,
            other.topology_epoch,
            other.tie_break,
        ))
    }
}

impl PartialOrd for RouteOrderingKey {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteLease {
    pub owner_node_id: NodeId,
    pub lease_epoch: RouteEpoch,
    pub leased_at: Tick,
    pub expires_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCost {
    pub message_count_max: Option<u32>,
    pub byte_count_max: Option<u64>,
    pub hop_count: u8,
    pub repair_attempt_count_max: Option<u32>,
    pub hold_bytes_reserved: Option<u64>,
    pub cpu_work_units_max: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteTransition {
    Established,
    Repaired,
    Replaced,
    HandedOff,
    EnteredPartitionMode,
    RecoveredFromPartition,
    Expired,
    Teardown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteOperationInstance {
    pub operation_id: RouteOperationId,
    pub route_id: Option<RouteId>,
    pub service_family: ServiceFamily,
    pub issued_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteOutstandingEffect {
    pub operation_id: RouteOperationId,
    pub owner_node_id: NodeId,
    pub deadline: Tick,
    pub retry_policy: TimeoutPolicy,
    pub state: RouteEffectState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteEffectState {
    Pending,
    Blocked,
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
    Invalidated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteSemanticHandoff {
    pub route_id: RouteId,
    pub from_node_id: NodeId,
    pub to_node_id: NodeId,
    pub handoff_epoch: RouteEpoch,
    pub receipt_id: [u8; 16],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteProgressContract {
    pub productive_step_count_max: Option<u32>,
    pub total_step_count_max: Option<u32>,
    pub last_progress_at: Tick,
    pub state: RouteProgressState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteProgressState {
    Pending,
    Blocked,
    Degraded,
    TimedOut,
    Satisfied,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledRoute {
    pub admission: RouteAdmission,
    pub lease: RouteLease,
    pub current_transition: RouteTransition,
    pub health: RouteHealth,
    pub progress: RouteProgressContract,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteHealth {
    pub reachability_state: ReachabilityState,
    pub stability_score: HealthScore,
    pub congestion_penalty_points: PenaltyPoints,
    pub last_validated_at: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReachabilityState {
    Reachable,
    Unreachable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteMaintenanceTrigger {
    LinkDegraded,
    CapacityExceeded,
    LeaseExpiring,
    EpochAdvanced,
    PolicyShift,
    RouteExpired,
    PartitionDetected,
    AntiEntropyRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteMaintenanceDisposition {
    Continue,
    Repaired,
    ReplaceRoute,
    HoldFallback,
    Fail,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ContentId<D> {
    pub digest: D,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct RouteAdmissionCapability(pub u64);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct RouteOwnershipCapability(pub u64);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct RouteEvidenceCapability(pub u64);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct RouteTransitionCapability(pub u64);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct BloomFilter;

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum ContentEncodingError {
    #[error("artifact is still open and cannot be canonically addressed")]
    OpenArtifact,
    #[error("artifact bytes are not in canonical form")]
    InvalidCanonicalForm,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum RouteError {
    #[error("route selection error: {0}")]
    Selection(#[from] RouteSelectionError),
    #[error("route runtime error: {0}")]
    Runtime(#[from] RouteRuntimeError),
    #[error("route policy error: {0}")]
    Policy(#[from] RoutePolicyError),
    #[error("capability error: {0}")]
    Capability(#[from] CapabilityError),
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum RouteSelectionError {
    #[error("no candidate route was available")]
    NoCandidate,
    #[error("privacy floor was not satisfied")]
    PrivacyFloorUnsatisfied,
    #[error("candidate was inadmissible: {0}")]
    Inadmissible(RouteAdmissionRejection),
    #[error("routing policy conflict")]
    PolicyConflict,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum RouteRuntimeError {
    #[error("route lease expired")]
    LeaseExpired,
    #[error("stale owner attempted a mutation")]
    StaleOwner,
    #[error("route transition was rejected")]
    TransitionRejected,
    #[error("route maintenance failed")]
    MaintenanceFailed,
    #[error("route operation timed out")]
    TimedOut,
    #[error("route state was invalidated")]
    Invalidated,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum RoutePolicyError {
    #[error("fallback is forbidden")]
    FallbackForbidden,
    #[error("profile is unsupported")]
    ProfileUnsupported,
    #[error("budget exceeded")]
    BudgetExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum CapabilityError {
    #[error("capability is unsupported")]
    Unsupported,
    #[error("capability was rejected")]
    Rejected,
    #[error("capability budget exceeded")]
    BudgetExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum TransportError {
    #[error("transport is unavailable")]
    Unavailable,
    #[error("transport operation timed out")]
    TimedOut,
    #[error("transport rejected the operation")]
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum MediumError {
    #[error("medium rejected the frame")]
    Rejected,
    #[error("medium data was corrupted")]
    Corrupted,
    #[error("medium operation timed out")]
    TimedOut,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum CustodyError {
    #[error("custody store is unavailable")]
    Unavailable,
    #[error("custody store is full")]
    Full,
    #[error("custody operation was rejected")]
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum HoldError {
    #[error("hold service is unavailable")]
    Unavailable,
    #[error("held object expired")]
    Expired,
    #[error("hold operation was rejected")]
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum PathSetupError {
    #[error("path setup is unsupported")]
    Unsupported,
    #[error("path setup was rejected")]
    Rejected,
    #[error("path setup was invalid")]
    Invalid,
}

impl fmt::Display for Blake3Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_ordering_key_is_total() {
        let low = RouteOrderingKey {
            priority: PriorityPoints(1),
            topology_epoch: RouteEpoch(2),
            tie_break: OrderStamp(3),
        };
        let high = RouteOrderingKey {
            priority: PriorityPoints(2),
            topology_epoch: RouteEpoch(2),
            tie_break: OrderStamp(3),
        };

        assert!(low < high);
    }

    #[test]
    fn topology_snapshot_has_deterministic_key_order() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            NodeId([2; 32]),
            TopologyNodeObservation {
                controller_id: ControllerId([9; 32]),
                services: Vec::new(),
                endpoints: Vec::new(),
                trust_class: PeerTrustClass::ControllerBound,
                last_seen_at: Tick(2),
            },
        );
        nodes.insert(
            NodeId([1; 32]),
            TopologyNodeObservation {
                controller_id: ControllerId([8; 32]),
                services: Vec::new(),
                endpoints: Vec::new(),
                trust_class: PeerTrustClass::ControllerBound,
                last_seen_at: Tick(1),
            },
        );

        let keys: Vec<_> = nodes.keys().copied().collect();
        assert_eq!(keys, vec![NodeId([1; 32]), NodeId([2; 32])]);
    }
}
