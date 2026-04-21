//! Bounded corridor search and router-facing candidate construction.

// proc-macro-scope: Mercator engine-private route planning stays outside #[public_model].

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteId, BackendRouteRef,
    Belief, ByteCount, ClaimStrength, Configuration, ConnectivityPosture, DestinationId, Estimate,
    Fact, FactBasis, Limit, Link, LinkRuntimeState, MessageFlowAssumptionClass, NodeDensityClass,
    NodeId, ObjectiveVsDelivered, Observation, PenaltyPoints, RatioPermille, ReachabilityState,
    RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCost, RouteDegradation, RouteEpoch,
    RouteEstimate, RouteHealth, RouteId, RouteInstallation, RouteLifecycleEvent,
    RouteMaterializationInput, RouteMaterializationProof, RouteProgressContract,
    RouteProgressState, RouteProtectionClass, RouteRepairClass, RouteRuntimeError,
    RouteSelectionError, RouteSummary, RouteWitness, RoutingObjective, RuntimeEnvelopeClass,
    SelectedRoutingParameters, Tick, TimeWindow, TransportKind,
};
use jacquard_traits::{Blake3Hashing, Hashing};

use crate::{
    evidence::{
        support_state_rank, MercatorEvidenceGraph, MercatorObjectiveKey, MercatorSupportState,
    },
    MercatorEngineConfig, MERCATOR_ENGINE_ID,
};

const MERCATOR_ROUTE_ID_DOMAIN: &[u8] = b"jacquard.mercator.route";
const MERCATOR_TOKEN_VERSION: u8 = 2;
const DEFAULT_VALIDITY_TICKS: u64 = 8;
const DEFAULT_WORK_STEP_BOUND: u32 = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MercatorCorridor {
    pub objective: MercatorObjectiveKey,
    pub primary: MercatorRouteRealization,
    pub alternates: Vec<MercatorRouteRealization>,
    pub topology_epoch: RouteEpoch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MercatorRouteRealization {
    pub path: Vec<NodeId>,
    pub support_score: u16,
    pub broker_pressure: u16,
    pub observed_at_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MercatorPlanningOutcome {
    Selected(MercatorCorridor),
    NoCandidate,
    Inadmissible,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MercatorPlanningContext {
    pub reserve_for_underserved_objective: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MercatorBackendToken {
    topology_epoch: RouteEpoch,
    destination: DestinationId,
    primary_path: Vec<NodeId>,
    alternate_next_hops: Vec<NodeId>,
    alternate_paths: Vec<Vec<NodeId>>,
    support_score: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveMercatorRoute {
    pub(crate) destination: DestinationId,
    pub(crate) topology_epoch: RouteEpoch,
    pub(crate) primary_path: Vec<NodeId>,
    pub(crate) alternate_next_hops: Vec<NodeId>,
    pub(crate) alternate_paths: Vec<Vec<NodeId>>,
    pub(crate) backend_route_id: BackendRouteId,
    pub(crate) support_score: u16,
    pub(crate) stale_started_at: Option<Tick>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CorridorOrderingKey {
    freshness_rank: u8,
    overload_rank: u8,
    support_score: u16,
    broker_pressure_inverse: std::cmp::Reverse<u16>,
    continuity_rank: u8,
    hop_count_inverse: std::cmp::Reverse<u8>,
    tie_break: NodeId,
}

impl MercatorCorridor {
    pub fn candidate(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
    ) -> Result<RouteCandidate, RouteSelectionError> {
        let backend_route_id = encode_backend_token(&MercatorBackendToken {
            topology_epoch: self.topology_epoch,
            destination: objective.destination.clone(),
            primary_path: self.primary.path.clone(),
            alternate_next_hops: self
                .alternates
                .iter()
                .filter_map(|alternate| alternate.path.get(1).copied())
                .collect(),
            alternate_paths: self
                .alternates
                .iter()
                .map(|alternate| alternate.path.clone())
                .collect(),
            support_score: self.primary.support_score,
        })?;
        let route_id = route_id_for_backend(&backend_route_id);
        let hop_count = hop_count_for_path(&self.primary.path);
        Ok(RouteCandidate {
            route_id,
            summary: RouteSummary {
                engine: MERCATOR_ENGINE_ID,
                protection: RouteProtectionClass::LinkProtected,
                connectivity: ConnectivityPosture {
                    repair: RouteRepairClass::Repairable,
                    partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
                },
                protocol_mix: protocol_mix_for_path(topology, &self.primary.path),
                hop_count_hint: Belief::certain(hop_count, topology.observed_at_tick),
                valid_for: candidate_window(topology.observed_at_tick)?,
            },
            estimate: Estimate::certain(
                RouteEstimate {
                    estimated_protection: RouteProtectionClass::LinkProtected,
                    estimated_connectivity: ConnectivityPosture {
                        repair: RouteRepairClass::Repairable,
                        partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
                    },
                    topology_epoch: topology.value.epoch,
                    degradation: RouteDegradation::None,
                },
                topology.observed_at_tick,
            ),
            backend_ref: BackendRouteRef {
                engine: MERCATOR_ENGINE_ID,
                backend_route_id,
            },
        })
    }

    #[must_use]
    pub fn avoided_overloaded_broker(&self, overload_threshold: u16) -> bool {
        self.primary.broker_pressure < overload_threshold
            && self
                .alternates
                .iter()
                .any(|alternate| alternate.broker_pressure >= overload_threshold)
    }
}

#[must_use]
pub fn plan_corridor(
    local_node_id: NodeId,
    topology: &Observation<Configuration>,
    objective: &RoutingObjective,
    config: &MercatorEngineConfig,
    evidence: &MercatorEvidenceGraph,
) -> MercatorPlanningOutcome {
    plan_corridor_with_context(
        local_node_id,
        topology,
        objective,
        config,
        evidence,
        MercatorPlanningContext::default(),
    )
}

#[must_use]
pub fn plan_corridor_with_context(
    local_node_id: NodeId,
    topology: &Observation<Configuration>,
    objective: &RoutingObjective,
    config: &MercatorEngineConfig,
    evidence: &MercatorEvidenceGraph,
    context: MercatorPlanningContext,
) -> MercatorPlanningOutcome {
    let DestinationId::Node(goal) = objective.destination else {
        return MercatorPlanningOutcome::NoCandidate;
    };
    if !topology.value.nodes.contains_key(&local_node_id)
        || !topology.value.nodes.contains_key(&goal)
    {
        return MercatorPlanningOutcome::Inadmissible;
    }
    let max_hops = planning_max_hops(config, context);
    let mut realizations = bounded_paths(local_node_id, goal, topology, evidence, max_hops);
    if realizations.is_empty() {
        return MercatorPlanningOutcome::NoCandidate;
    }
    realizations.sort_by_key(|realization| {
        std::cmp::Reverse(realization_ordering_key_with_config(realization, config))
    });
    let alternate_cap =
        usize::try_from(config.evidence.corridor_alternate_count_max).unwrap_or(usize::MAX);
    let primary = realizations.remove(0);
    let alternates = realizations.into_iter().take(alternate_cap).collect();
    MercatorPlanningOutcome::Selected(MercatorCorridor {
        objective: MercatorObjectiveKey::destination(objective.destination.clone()),
        primary,
        alternates,
        topology_epoch: topology.value.epoch,
    })
}

pub fn candidate_for(
    local_node_id: NodeId,
    topology: &Observation<Configuration>,
    objective: &RoutingObjective,
    config: &MercatorEngineConfig,
    evidence: &MercatorEvidenceGraph,
) -> Result<RouteCandidate, RouteSelectionError> {
    candidate_for_with_context(
        local_node_id,
        topology,
        objective,
        config,
        evidence,
        MercatorPlanningContext::default(),
    )
}

pub fn candidate_for_with_context(
    local_node_id: NodeId,
    topology: &Observation<Configuration>,
    objective: &RoutingObjective,
    config: &MercatorEngineConfig,
    evidence: &MercatorEvidenceGraph,
    context: MercatorPlanningContext,
) -> Result<RouteCandidate, RouteSelectionError> {
    match plan_corridor_with_context(
        local_node_id,
        topology,
        objective,
        config,
        evidence,
        context,
    ) {
        MercatorPlanningOutcome::Selected(corridor) => corridor.candidate(objective, topology),
        MercatorPlanningOutcome::NoCandidate => Err(RouteSelectionError::NoCandidate),
        MercatorPlanningOutcome::Inadmissible => Err(RouteSelectionError::Inadmissible(
            jacquard_core::RouteAdmissionRejection::BackendUnavailable,
        )),
    }
}

pub fn check_candidate(
    local_node_id: NodeId,
    topology: &Observation<Configuration>,
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    candidate: &RouteCandidate,
    config: &MercatorEngineConfig,
    evidence: &MercatorEvidenceGraph,
) -> Result<RouteAdmissionCheck, RouteSelectionError> {
    let expected = candidate_for_with_context(
        local_node_id,
        topology,
        objective,
        config,
        evidence,
        MercatorPlanningContext::default(),
    )?;
    if expected.backend_ref != candidate.backend_ref || expected.route_id != candidate.route_id {
        return Err(RouteSelectionError::Inadmissible(
            jacquard_core::RouteAdmissionRejection::BackendUnavailable,
        ));
    }
    Ok(admission_for(topology, objective, profile, expected).admission_check)
}

pub fn admit_candidate(
    local_node_id: NodeId,
    topology: &Observation<Configuration>,
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    candidate: &RouteCandidate,
    config: &MercatorEngineConfig,
    evidence: &MercatorEvidenceGraph,
) -> Result<RouteAdmission, RouteSelectionError> {
    let expected = candidate_for_with_context(
        local_node_id,
        topology,
        objective,
        config,
        evidence,
        MercatorPlanningContext::default(),
    )?;
    if expected.backend_ref != candidate.backend_ref || expected.route_id != candidate.route_id {
        return Err(RouteSelectionError::Inadmissible(
            jacquard_core::RouteAdmissionRejection::BackendUnavailable,
        ));
    }
    Ok(admission_for(topology, objective, profile, expected))
}

pub fn materialize_admitted(
    input: RouteMaterializationInput,
) -> Result<RouteInstallation, RouteRuntimeError> {
    if input.admission.backend_ref.engine != MERCATOR_ENGINE_ID {
        return Err(RouteRuntimeError::Invalidated);
    }
    let token = decode_backend_token(&input.admission.backend_ref.backend_route_id)
        .ok_or(RouteRuntimeError::Invalidated)?;
    if token.topology_epoch != input.handle.topology_epoch()
        || route_id_for_backend(&input.admission.backend_ref.backend_route_id)
            != *input.handle.route_id()
    {
        return Err(RouteRuntimeError::Invalidated);
    }
    let now = input.handle.materialized_at_tick();
    Ok(RouteInstallation {
        materialization_proof: RouteMaterializationProof {
            stamp: input.handle.stamp.clone(),
            witness: Fact {
                basis: FactBasis::Admitted,
                value: input.admission.witness,
                established_at_tick: now,
            },
        },
        last_lifecycle_event: RouteLifecycleEvent::Activated,
        health: RouteHealth {
            reachability_state: ReachabilityState::Reachable,
            stability_score: jacquard_core::HealthScore(u32::from(token.support_score)),
            congestion_penalty_points: PenaltyPoints(0),
            last_validated_at_tick: now,
        },
        progress: RouteProgressContract {
            productive_step_count_max: Limit::Bounded(1),
            total_step_count_max: Limit::Bounded(DEFAULT_WORK_STEP_BOUND),
            last_progress_at_tick: now,
            state: RouteProgressState::Pending,
        },
    })
}

pub(crate) fn active_route_from_backend(
    backend_route_id: BackendRouteId,
) -> Option<ActiveMercatorRoute> {
    let token = decode_backend_token(&backend_route_id)?;
    Some(ActiveMercatorRoute {
        destination: token.destination,
        topology_epoch: token.topology_epoch,
        primary_path: token.primary_path,
        alternate_next_hops: token.alternate_next_hops,
        alternate_paths: token.alternate_paths,
        backend_route_id,
        support_score: token.support_score,
        stale_started_at: None,
    })
}

pub fn selected_neighbor_from_backend_route_id(
    backend_route_id: &BackendRouteId,
) -> Option<NodeId> {
    decode_backend_token(backend_route_id).and_then(|token| token.primary_path.get(1).copied())
}

pub(crate) fn path_is_viable(
    path: &[NodeId],
    topology: &Observation<Configuration>,
    evidence: &MercatorEvidenceGraph,
) -> bool {
    !path.is_empty()
        && path.windows(2).all(|edge| {
            edge_score(edge[0], edge[1], topology, evidence).is_some_and(|score| score > 0)
        })
}

pub(crate) fn repair_realization_from_alternates(
    local_node_id: NodeId,
    active: &ActiveMercatorRoute,
    topology: &Observation<Configuration>,
    config: &MercatorEngineConfig,
    evidence: &MercatorEvidenceGraph,
) -> Option<MercatorRouteRealization> {
    let DestinationId::Node(goal) = active.destination else {
        return None;
    };
    surviving_alternate_path(active, topology, config, evidence).or_else(|| {
        searched_alternate_path(local_node_id, goal, active, topology, config, evidence)
    })
}

fn bounded_paths(
    start: NodeId,
    goal: NodeId,
    topology: &Observation<Configuration>,
    evidence: &MercatorEvidenceGraph,
    max_hops: u8,
) -> Vec<MercatorRouteRealization> {
    let mut paths = Vec::new();
    let mut queue = VecDeque::from([vec![start]]);
    while let Some(path) = queue.pop_front() {
        let Some(current) = path.last().copied() else {
            continue;
        };
        if current == goal {
            paths.push(realization_for_path(&path, topology, evidence));
            continue;
        }
        if hop_count_for_path(&path) >= max_hops {
            continue;
        }
        for neighbor in usable_neighbors(current, topology, evidence) {
            if path.contains(&neighbor) {
                continue;
            }
            let mut next = path.clone();
            next.push(neighbor);
            queue.push_back(next);
        }
    }
    paths
}

fn surviving_alternate_path(
    active: &ActiveMercatorRoute,
    topology: &Observation<Configuration>,
    config: &MercatorEngineConfig,
    evidence: &MercatorEvidenceGraph,
) -> Option<MercatorRouteRealization> {
    active
        .alternate_paths
        .iter()
        .filter(|path| path_is_viable(path, topology, evidence))
        .map(|path| realization_for_path(path, topology, evidence))
        .max_by_key(|realization| realization_ordering_key_with_config(realization, config))
}

fn searched_alternate_path(
    local_node_id: NodeId,
    goal: NodeId,
    active: &ActiveMercatorRoute,
    topology: &Observation<Configuration>,
    config: &MercatorEngineConfig,
    evidence: &MercatorEvidenceGraph,
) -> Option<MercatorRouteRealization> {
    let max_hops = repair_max_hops(config);
    active
        .alternate_next_hops
        .iter()
        .copied()
        .take(usize::try_from(config.bounds.repair_attempt_count_max).unwrap_or(usize::MAX))
        .filter_map(|next_hop| {
            repair_path_via_next_hop(
                local_node_id,
                next_hop,
                goal,
                topology,
                evidence,
                max_hops,
                config,
            )
        })
        .max_by_key(|realization| realization_ordering_key_with_config(realization, config))
}

fn repair_path_via_next_hop(
    local_node_id: NodeId,
    next_hop: NodeId,
    goal: NodeId,
    topology: &Observation<Configuration>,
    evidence: &MercatorEvidenceGraph,
    max_hops: u8,
    config: &MercatorEngineConfig,
) -> Option<MercatorRouteRealization> {
    edge_score(local_node_id, next_hop, topology, evidence)?;
    bounded_paths(next_hop, goal, topology, evidence, max_hops)
        .into_iter()
        .map(|suffix| {
            let mut path = Vec::with_capacity(suffix.path.len().saturating_add(1));
            path.push(local_node_id);
            path.extend(suffix.path);
            realization_for_path(&path, topology, evidence)
        })
        .max_by_key(|realization| realization_ordering_key_with_config(realization, config))
}

fn planning_max_hops(config: &MercatorEngineConfig, context: MercatorPlanningContext) -> u8 {
    let fairness_budget = if context.reserve_for_underserved_objective {
        config.bounds.repair_attempt_count_max
    } else {
        0
    };
    u8::try_from(
        config
            .evidence
            .corridor_alternate_count_max
            .saturating_add(fairness_budget)
            .saturating_add(2),
    )
    .unwrap_or(u8::MAX)
    .max(2)
}

fn repair_max_hops(config: &MercatorEngineConfig) -> u8 {
    u8::try_from(
        config
            .evidence
            .corridor_alternate_count_max
            .saturating_add(config.bounds.repair_attempt_count_max)
            .saturating_add(2),
    )
    .unwrap_or(u8::MAX)
    .max(2)
}

fn usable_neighbors(
    node: NodeId,
    topology: &Observation<Configuration>,
    evidence: &MercatorEvidenceGraph,
) -> Vec<NodeId> {
    let mut neighbors = topology
        .value
        .links
        .iter()
        .filter_map(|((from, to), link)| {
            (*from == node && link_is_usable(link)).then_some((*to, topology_link_score(link)))
        })
        .collect::<BTreeMap<_, _>>();
    for maintained in evidence.link_evidence() {
        if maintained.from == node && maintained.bidirectional_confidence > 0 {
            neighbors
                .entry(maintained.to)
                .and_modify(|score| *score = (*score).max(maintained.bidirectional_confidence))
                .or_insert(maintained.bidirectional_confidence);
        }
    }
    let mut ranked = neighbors.into_iter().collect::<Vec<_>>();
    ranked.sort_by_key(|(neighbor, score)| (std::cmp::Reverse(*score), *neighbor));
    ranked.into_iter().map(|(neighbor, _)| neighbor).collect()
}

fn realization_for_path(
    path: &[NodeId],
    topology: &Observation<Configuration>,
    evidence: &MercatorEvidenceGraph,
) -> MercatorRouteRealization {
    let support_score = path
        .windows(2)
        .filter_map(|edge| edge_score(edge[0], edge[1], topology, evidence))
        .min()
        .unwrap_or(0);
    MercatorRouteRealization {
        path: path.to_vec(),
        support_score,
        broker_pressure: broker_pressure_for_path(path, evidence),
        observed_at_tick: topology.observed_at_tick,
    }
}

fn edge_score(
    from: NodeId,
    to: NodeId,
    topology: &Observation<Configuration>,
    evidence: &MercatorEvidenceGraph,
) -> Option<u16> {
    let topology_score = topology
        .value
        .links
        .get(&(from, to))
        .filter(|link| link_is_usable(link))
        .map(topology_link_score);
    let maintained_score = evidence
        .link_evidence()
        .into_iter()
        .find(|candidate| candidate.from == from && candidate.to == to)
        .map(|candidate| candidate.bidirectional_confidence);
    topology_score.into_iter().chain(maintained_score).max()
}

fn broker_pressure_for_path(path: &[NodeId], evidence: &MercatorEvidenceGraph) -> u16 {
    path.iter()
        .skip(1)
        .take(path.len().saturating_sub(2))
        .filter_map(|broker| {
            evidence
                .broker_pressure_for(*broker)
                .map(|pressure| pressure.pressure_score)
        })
        .max()
        .unwrap_or(0)
}

fn realization_ordering_key_with_config(
    realization: &MercatorRouteRealization,
    config: &MercatorEngineConfig,
) -> CorridorOrderingKey {
    realization_ordering_key_with_threshold(
        realization,
        config.bounds.broker_overload_pressure_threshold,
    )
}

fn realization_ordering_key_with_threshold(
    realization: &MercatorRouteRealization,
    overload_threshold: u16,
) -> CorridorOrderingKey {
    CorridorOrderingKey {
        freshness_rank: support_state_rank(MercatorSupportState::Fresh),
        overload_rank: u8::from(realization.broker_pressure < overload_threshold),
        support_score: realization.support_score,
        broker_pressure_inverse: std::cmp::Reverse(realization.broker_pressure),
        continuity_rank: 0,
        hop_count_inverse: std::cmp::Reverse(hop_count_for_path(&realization.path)),
        tie_break: realization.path.last().copied().unwrap_or(NodeId([0; 32])),
    }
}

fn admission_for(
    topology: &Observation<Configuration>,
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    candidate: RouteCandidate,
) -> RouteAdmission {
    let admission_profile = admission_profile_for(topology);
    RouteAdmission {
        backend_ref: candidate.backend_ref,
        objective: objective.clone(),
        profile: profile.clone(),
        admission_check: RouteAdmissionCheck {
            decision: AdmissionDecision::Admissible,
            profile: admission_profile.clone(),
            productive_step_bound: Limit::Bounded(1),
            total_step_bound: Limit::Bounded(DEFAULT_WORK_STEP_BOUND),
            route_cost: RouteCost {
                message_count_max: Limit::Bounded(1),
                byte_count_max: Limit::Bounded(ByteCount(0)),
                hop_count: candidate.summary.hop_count_hint.value_or(1),
                repair_attempt_count_max: Limit::Bounded(0),
                hold_bytes_reserved: Limit::Bounded(ByteCount(0)),
                work_step_count_max: Limit::Bounded(DEFAULT_WORK_STEP_BOUND),
            },
        },
        summary: candidate.summary,
        witness: RouteWitness {
            protection: ObjectiveVsDelivered {
                objective: objective.target_protection,
                delivered: RouteProtectionClass::LinkProtected,
            },
            connectivity: ObjectiveVsDelivered {
                objective: profile.selected_connectivity,
                delivered: ConnectivityPosture {
                    repair: RouteRepairClass::Repairable,
                    partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
                },
            },
            admission_profile,
            topology_epoch: topology.value.epoch,
            degradation: RouteDegradation::None,
        },
    }
}

fn admission_profile_for(topology: &Observation<Configuration>) -> AdmissionAssumptions {
    AdmissionAssumptions {
        message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
        failure_model: jacquard_core::FailureModelClass::Benign,
        runtime_envelope: RuntimeEnvelopeClass::Canonical,
        node_density_class: match topology.value.environment.reachable_neighbor_count {
            0..=1 => NodeDensityClass::Sparse,
            2..=4 => NodeDensityClass::Moderate,
            _ => NodeDensityClass::Dense,
        },
        connectivity_regime: jacquard_core::ConnectivityRegime::Stable,
        adversary_regime: AdversaryRegime::Cooperative,
        claim_strength: ClaimStrength::ConservativeUnderProfile,
    }
}

fn candidate_window(start_tick: Tick) -> Result<TimeWindow, RouteSelectionError> {
    TimeWindow::new(
        start_tick,
        Tick(start_tick.0.saturating_add(DEFAULT_VALIDITY_TICKS)),
    )
    .map_err(|_| RouteSelectionError::PolicyConflict)
}

fn protocol_mix_for_path(
    topology: &Observation<Configuration>,
    path: &[NodeId],
) -> Vec<TransportKind> {
    let protocols = path
        .windows(2)
        .filter_map(|edge| topology.value.links.get(&(edge[0], edge[1])))
        .map(|link| link.endpoint.transport_kind.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if protocols.is_empty() {
        vec![TransportKind::WifiAware]
    } else {
        protocols
    }
}

fn link_is_usable(link: &Link) -> bool {
    matches!(
        link.state.state,
        LinkRuntimeState::Active | LinkRuntimeState::Degraded
    )
}

fn topology_link_score(link: &Link) -> u16 {
    link.state
        .delivery_confidence_permille
        .value()
        .unwrap_or(RatioPermille(500))
        .0
}

fn hop_count_for_path(path: &[NodeId]) -> u8 {
    u8::try_from(path.len().saturating_sub(1)).unwrap_or(u8::MAX)
}

fn route_id_for_backend(backend_route_id: &BackendRouteId) -> RouteId {
    RouteId::from(&Blake3Hashing.hash_tagged(MERCATOR_ROUTE_ID_DOMAIN, &backend_route_id.0))
}

fn encode_backend_token(
    token: &MercatorBackendToken,
) -> Result<BackendRouteId, RouteSelectionError> {
    let mut bytes = Vec::new();
    bytes.push(MERCATOR_TOKEN_VERSION);
    bytes.extend_from_slice(&token.topology_epoch.0.to_be_bytes());
    encode_destination(&token.destination, &mut bytes)?;
    bytes.push(
        u8::try_from(token.primary_path.len()).map_err(|_| RouteSelectionError::PolicyConflict)?,
    );
    for node in &token.primary_path {
        bytes.extend_from_slice(&node.0);
    }
    bytes.push(
        u8::try_from(token.alternate_next_hops.len())
            .map_err(|_| RouteSelectionError::PolicyConflict)?,
    );
    for node in &token.alternate_next_hops {
        bytes.extend_from_slice(&node.0);
    }
    bytes.push(
        u8::try_from(token.alternate_paths.len())
            .map_err(|_| RouteSelectionError::PolicyConflict)?,
    );
    for path in &token.alternate_paths {
        bytes.push(u8::try_from(path.len()).map_err(|_| RouteSelectionError::PolicyConflict)?);
        for node in path {
            bytes.extend_from_slice(&node.0);
        }
    }
    bytes.extend_from_slice(&token.support_score.to_be_bytes());
    Ok(BackendRouteId(bytes))
}

fn decode_backend_token(backend_route_id: &BackendRouteId) -> Option<MercatorBackendToken> {
    let bytes = &backend_route_id.0;
    let mut cursor = 0_usize;
    if *bytes.get(cursor)? != MERCATOR_TOKEN_VERSION {
        return None;
    }
    cursor = cursor.saturating_add(1);
    let topology_epoch = RouteEpoch(u64::from_be_bytes(read_array(bytes, &mut cursor)?));
    let destination = decode_destination(bytes, &mut cursor)?;
    let primary_len = usize::from(*bytes.get(cursor)?);
    cursor = cursor.saturating_add(1);
    let mut primary_path = Vec::with_capacity(primary_len);
    for _ in 0..primary_len {
        primary_path.push(NodeId(read_array(bytes, &mut cursor)?));
    }
    let alternate_len = usize::from(*bytes.get(cursor)?);
    cursor = cursor.saturating_add(1);
    let mut alternate_next_hops = Vec::with_capacity(alternate_len);
    for _ in 0..alternate_len {
        alternate_next_hops.push(NodeId(read_array(bytes, &mut cursor)?));
    }
    let alternate_path_len = usize::from(*bytes.get(cursor)?);
    cursor = cursor.saturating_add(1);
    let mut alternate_paths = Vec::with_capacity(alternate_path_len);
    for _ in 0..alternate_path_len {
        let path_len = usize::from(*bytes.get(cursor)?);
        cursor = cursor.saturating_add(1);
        let mut path = Vec::with_capacity(path_len);
        for _ in 0..path_len {
            path.push(NodeId(read_array(bytes, &mut cursor)?));
        }
        alternate_paths.push(path);
    }
    let support_score = u16::from_be_bytes(read_array(bytes, &mut cursor)?);
    Some(MercatorBackendToken {
        topology_epoch,
        destination,
        primary_path,
        alternate_next_hops,
        alternate_paths,
        support_score,
    })
}

fn encode_destination(
    destination: &DestinationId,
    out: &mut Vec<u8>,
) -> Result<(), RouteSelectionError> {
    match destination {
        DestinationId::Node(node) => {
            out.push(0);
            out.extend_from_slice(&node.0);
        }
        DestinationId::Service(service) => {
            out.push(1);
            let len =
                u16::try_from(service.0.len()).map_err(|_| RouteSelectionError::PolicyConflict)?;
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(&service.0);
        }
        DestinationId::Gateway(gateway) => {
            out.push(2);
            out.extend_from_slice(&gateway.0);
        }
    }
    Ok(())
}

fn decode_destination(bytes: &[u8], cursor: &mut usize) -> Option<DestinationId> {
    let kind = *bytes.get(*cursor)?;
    *cursor = (*cursor).saturating_add(1);
    match kind {
        0 => Some(DestinationId::Node(NodeId(read_array(bytes, cursor)?))),
        1 => {
            let len = usize::from(u16::from_be_bytes(read_array(bytes, cursor)?));
            let end = (*cursor).checked_add(len)?;
            let service = bytes.get(*cursor..end)?.to_vec();
            *cursor = end;
            Some(DestinationId::Service(jacquard_core::ServiceId(service)))
        }
        2 => Some(DestinationId::Gateway(jacquard_core::GatewayId(
            read_array(bytes, cursor)?,
        ))),
        _ => None,
    }
}

fn read_array<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Option<[u8; N]> {
    let end = cursor.checked_add(N)?;
    let mut out = [0u8; N];
    out.copy_from_slice(bytes.get(*cursor..end)?);
    *cursor = end;
    Some(out)
}
