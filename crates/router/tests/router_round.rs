mod common;

use std::sync::{Arc, Mutex};

use common::{profile, sample_configuration, sample_policy_inputs, LOCAL_NODE_ID};
use jacquard_core::{
    Configuration, ConnectivityPosture, EndpointLocator, LinkEndpoint, Observation,
    RouteError, RouteMaintenanceResult, RouteMaintenanceTrigger, RouteProtectionClass,
    RouteRuntimeState, RoutingEngineCapabilities, RoutingEngineId, RoutingTickChange,
    RoutingTickContext, RoutingTickHint, RoutingTickOutcome, SelectedRoutingParameters,
    Tick, TransportKind, TransportObservation,
};
use jacquard_mem_link_profile::InMemoryRuntimeEffects;
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::{
    RouterManagedEngine, RoutingControlPlane, RoutingEngine, RoutingEnginePlanner,
};

#[derive(Clone, Default)]
struct SharedIngressLog(Arc<Mutex<Vec<Vec<u8>>>>);

impl SharedIngressLog {
    fn entries(&self) -> Vec<Vec<u8>> {
        self.0.lock().expect("shared ingress log").clone()
    }
}

struct RecordingIngressEngine {
    local_node_id: jacquard_core::NodeId,
    pending: Vec<Vec<u8>>,
    observed: SharedIngressLog,
}

impl RecordingIngressEngine {
    fn new(local_node_id: jacquard_core::NodeId, observed: SharedIngressLog) -> Self {
        Self {
            local_node_id,
            pending: Vec::new(),
            observed,
        }
    }
}

impl RoutingEnginePlanner for RecordingIngressEngine {
    fn engine_id(&self) -> RoutingEngineId {
        RoutingEngineId::from_contract_bytes(*b"jacquard.round.1")
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        RoutingEngineCapabilities {
            engine: self.engine_id(),
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: ConnectivityPosture {
                repair: jacquard_core::RouteRepairClass::BestEffort,
                partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            repair_support: jacquard_core::RepairSupport::Unsupported,
            hold_support: jacquard_core::HoldSupport::Unsupported,
            decidable_admission: jacquard_core::DecidableSupport::Supported,
            quantitative_bounds:
                jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility: jacquard_core::RouteShapeVisibility::NextHopOnly,
        }
    }

    fn candidate_routes(
        &self,
        _objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Configuration>,
    ) -> Vec<jacquard_core::RouteCandidate> {
        Vec::new()
    }

    fn check_candidate(
        &self,
        _objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: &jacquard_core::RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<jacquard_core::RouteAdmissionCheck, RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn admit_route(
        &self,
        _objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: jacquard_core::RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<jacquard_core::RouteAdmission, RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }
}

impl RoutingEngine for RecordingIngressEngine {
    fn materialize_route(
        &mut self,
        _input: jacquard_core::RouteMaterializationInput,
    ) -> Result<jacquard_core::RouteInstallation, RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn route_commitments(
        &self,
        _route: &jacquard_core::MaterializedRoute,
    ) -> Vec<jacquard_core::RouteCommitment> {
        Vec::new()
    }

    fn engine_tick(
        &mut self,
        tick: &RoutingTickContext,
    ) -> Result<RoutingTickOutcome, RouteError> {
        let change = if self.pending.is_empty() {
            RoutingTickChange::NoChange
        } else {
            self.observed
                .0
                .lock()
                .expect("shared ingress log")
                .extend(self.pending.drain(..));
            RoutingTickChange::PrivateStateUpdated
        };
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change,
            next_tick_hint: if change == RoutingTickChange::PrivateStateUpdated {
                RoutingTickHint::Immediate
            } else {
                RoutingTickHint::HostDefault
            },
        })
    }

    fn maintain_route(
        &mut self,
        _identity: &jacquard_core::PublishedRouteRecord,
        _runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn teardown(&mut self, _route_id: &jacquard_core::RouteId) {}
}

impl RouterManagedEngine for RecordingIngressEngine {
    fn local_node_id_for_router(&self) -> jacquard_core::NodeId {
        self.local_node_id
    }

    fn ingest_transport_observation_for_router(
        &mut self,
        observation: &TransportObservation,
    ) -> Result<(), RouteError> {
        if let TransportObservation::PayloadReceived { payload, .. } = observation {
            self.pending.push(payload.clone());
        }
        Ok(())
    }

    fn forward_payload_for_router(
        &mut self,
        _route_id: &jacquard_core::RouteId,
        _payload: &[u8],
    ) -> Result<(), RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn restore_route_runtime_for_router(
        &mut self,
        _route_id: &jacquard_core::RouteId,
    ) -> Result<bool, RouteError> {
        Ok(false)
    }
}

fn payload_observation(byte: u8) -> TransportObservation {
    TransportObservation::PayloadReceived {
        from_node_id: LOCAL_NODE_ID,
        endpoint: LinkEndpoint::new(
            TransportKind::WifiAware,
            EndpointLocator::Opaque(vec![byte]),
            jacquard_core::ByteCount(128),
        ),
        payload: vec![byte],
        observed_at_tick: Tick(2),
    }
}

fn build_round_router(
    log: SharedIngressLog,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let mut router = MultiEngineRouter::new(
        LOCAL_NODE_ID,
        FixedPolicyEngine::new(profile()),
        InMemoryRuntimeEffects { now: Tick(2), ..Default::default() },
        topology.clone(),
        sample_policy_inputs(&topology),
    );
    router
        .register_engine(Box::new(RecordingIngressEngine::new(LOCAL_NODE_ID, log)))
        .expect("register recording ingress engine");
    router
}

#[test]
fn explicit_transport_ingress_is_delivered_to_router_rounds_in_fifo_order() {
    let log = SharedIngressLog::default();
    let mut router = build_round_router(log.clone());

    router
        .ingest_transport_observation(&payload_observation(1))
        .expect("ingest first");
    router
        .ingest_transport_observation(&payload_observation(2))
        .expect("ingest second");

    let outcome = router.advance_round().expect("advance round");

    assert_eq!(
        outcome.engine_change,
        RoutingTickChange::PrivateStateUpdated
    );
    assert_eq!(outcome.next_round_hint, RoutingTickHint::Immediate);
    assert_eq!(log.entries(), vec![vec![1], vec![2]]);
}

#[test]
fn explicit_router_round_progression_is_deterministic_for_equal_ingress() {
    let left_log = SharedIngressLog::default();
    let right_log = SharedIngressLog::default();
    let mut left = build_round_router(left_log.clone());
    let mut right = build_round_router(right_log.clone());

    for router in [&mut left, &mut right] {
        router
            .ingest_transport_observation(&payload_observation(7))
            .expect("ingest payload");
        router
            .ingest_transport_observation(&payload_observation(9))
            .expect("ingest payload");
    }

    let left_outcome = left.advance_round().expect("left round");
    let right_outcome = right.advance_round().expect("right round");

    assert_eq!(left_outcome, right_outcome);
    assert_eq!(left_log.entries(), right_log.entries());
}

#[test]
fn advance_round_without_ingress_reports_no_change() {
    let mut router = build_round_router(SharedIngressLog::default());

    let outcome = router.advance_round().expect("advance round");

    assert_eq!(outcome.engine_change, RoutingTickChange::NoChange);
    assert_eq!(outcome.next_round_hint, RoutingTickHint::HostDefault);
}
