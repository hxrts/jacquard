//! Mesh-only router implementation over the shared routing traits.
//!
//! Control flow intuition: activation goes `objective -> profile ->
//! candidate set -> admissible candidate -> router-owned handle/lease -> engine
//! materialization -> canonical publication`. Maintenance and anti-entropy stay
//! router-owned at the semantic level even when the mesh engine performs the
//! route-private work underneath.
//!
//! Ownership:
//! - canonical route mutations happen here
//! - mesh only returns typed evidence and route-private runtime state

use std::{cmp::Reverse, collections::BTreeMap};

use jacquard_core::{
    AdaptiveRoutingProfile, AdmissionDecision, Belief, CapabilityError, Configuration,
    FactSourceClass, MaterializedRoute, Observation, OrderStamp,
    OriginAuthenticationClass, PublicationId, RouteCandidate, RouteCommitment,
    RouteDegradation, RouteError, RouteHandle, RouteHealth, RouteId, RouteLease,
    RouteMaintenanceResult, RouteMaintenanceTrigger, RouteMaterializationInput,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteRuntimeError,
    RouteSelectionError, RouteSemanticHandoff, RouterCanonicalMutation,
    RouterMaintenanceOutcome, RouterTickOutcome, RoutingEngineCapabilities,
    RoutingEngineId, RoutingEvidenceClass, RoutingObjective, RoutingPolicyInputs,
    RoutingTickChange, RoutingTickContext, Tick, TimeWindow, TransportProtocol,
};
use jacquard_mesh::MeshEngine;
use jacquard_traits::{
    CommitteeSelector, HashDigestBytes, Hashing, MeshNeighborhoodEstimateAccess,
    MeshPeerEstimateAccess, MeshTopologyModel, MeshTransport, OrderEffects,
    PolicyEngine, RetentionStore, RouteEventLogEffects, Router, RoutingControlPlane,
    RoutingDataPlane, RoutingEngine, StorageEffects, TimeEffects,
};

use crate::runtime::{RouterCheckpointRecord, RouterRuntimeAdapter};

const DEFAULT_ROUTE_LEASE_TICKS: u64 = 32;

/// Minimal mesh-only policy engine used by the first router implementation.
#[derive(Clone, Debug)]
pub struct FixedPolicyEngine {
    profile: AdaptiveRoutingProfile,
}

impl FixedPolicyEngine {
    #[must_use]
    pub fn new(profile: AdaptiveRoutingProfile) -> Self {
        Self { profile }
    }
}

impl PolicyEngine for FixedPolicyEngine {
    fn compute_profile(
        &self,
        _objective: &RoutingObjective,
        _inputs: &RoutingPolicyInputs,
    ) -> AdaptiveRoutingProfile {
        self.profile.clone()
    }
}

/// Local bridge for the still-mesh-specific data-plane seam.
pub trait MeshRouterEngineBridge: RoutingEngine {
    fn local_node_id_for_router(&self) -> jacquard_core::NodeId;

    fn forward_payload_for_router(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError>;

    fn restore_route_runtime_for_router(
        &mut self,
        route_id: &RouteId,
    ) -> Result<bool, RouteError>;
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector> MeshRouterEngineBridge
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: MeshTopologyModel,
    Topology::PeerEstimate: MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
    Transport: MeshTransport + Send + Sync + 'static,
    Retention: RetentionStore,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
    Hasher: Hashing,
    Hasher::Digest: HashDigestBytes,
    Selector: CommitteeSelector<TopologyView = Configuration>,
{
    fn local_node_id_for_router(&self) -> jacquard_core::NodeId {
        self.local_node_id()
    }

    fn forward_payload_for_router(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        self.forward_payload(route_id, payload)
    }

    fn restore_route_runtime_for_router(
        &mut self,
        route_id: &RouteId,
    ) -> Result<bool, RouteError> {
        Ok(self.restore_checkpointed_route(route_id)?.is_some())
    }
}

/// Router-owned canonical mesh route table plus one mesh engine.
pub struct MeshOnlyRouter<Engine, Policy, Effects> {
    engine:                  Engine,
    registered_engine_id:    RoutingEngineId,
    registered_capabilities: RoutingEngineCapabilities,
    policy_engine:           Policy,
    effects:                 Effects,
    topology:                Observation<Configuration>,
    policy_inputs:           RoutingPolicyInputs,
    active_routes:           BTreeMap<RouteId, MaterializedRoute>,
    published_commitments:   BTreeMap<RouteId, Vec<RouteCommitment>>,
}

impl<Engine, Policy, Effects> MeshOnlyRouter<Engine, Policy, Effects>
where
    Engine: MeshRouterEngineBridge,
    Policy: PolicyEngine,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
{
    #[must_use]
    pub fn new(
        engine: Engine,
        policy_engine: Policy,
        effects: Effects,
        topology: Observation<Configuration>,
        policy_inputs: RoutingPolicyInputs,
    ) -> Self {
        let registered_engine_id = engine.engine_id();
        let registered_capabilities = engine.capabilities();
        Self {
            engine,
            registered_engine_id,
            registered_capabilities,
            policy_engine,
            effects,
            topology,
            policy_inputs,
            active_routes: BTreeMap::new(),
            published_commitments: BTreeMap::new(),
        }
    }

    pub fn replace_topology(&mut self, topology: Observation<Configuration>) {
        self.topology = topology;
    }

    pub fn replace_policy_inputs(&mut self, inputs: RoutingPolicyInputs) {
        self.policy_inputs = inputs;
    }

    pub fn recover_checkpointed_routes(&mut self) -> Result<usize, RouteError> {
        let local_node_id = self.engine.local_node_id_for_router();
        let records = RouterRuntimeAdapter::new(local_node_id, &mut self.effects)
            .load_routes()?;
        let mut recovered = 0usize;
        for (route_id, record) in records {
            if !self.engine.restore_route_runtime_for_router(&route_id)? {
                RouterRuntimeAdapter::new(local_node_id, &mut self.effects)
                    .remove_route(&route_id)?;
                continue;
            }
            self.active_routes.insert(route_id, record.route);
            self.published_commitments
                .insert(route_id, record.commitments);
            recovered = recovered.saturating_add(1);
        }
        Ok(recovered)
    }

    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    #[must_use]
    pub fn effects(&self) -> &Effects {
        &self.effects
    }

    pub fn effects_mut(&mut self) -> &mut Effects {
        &mut self.effects
    }

    #[must_use]
    pub fn registered_engine_id(&self) -> RoutingEngineId {
        self.registered_engine_id.clone()
    }

    #[must_use]
    pub fn registered_capabilities(&self) -> &RoutingEngineCapabilities {
        &self.registered_capabilities
    }

    #[must_use]
    pub fn active_route(&self, route_id: &RouteId) -> Option<&MaterializedRoute> {
        self.active_routes.get(route_id)
    }

    #[must_use]
    pub fn active_route_count(&self) -> usize {
        self.active_routes.len()
    }

    fn runtime_adapter(&mut self) -> RouterRuntimeAdapter<'_, Effects> {
        let local_node_id = self.engine.local_node_id_for_router();
        RouterRuntimeAdapter::new(local_node_id, &mut self.effects)
    }

    fn remove_published_route(&mut self, route_id: &RouteId) -> Result<(), RouteError> {
        self.runtime_adapter().remove_route(route_id)?;
        self.active_routes.remove(route_id);
        self.published_commitments.remove(route_id);
        Ok(())
    }

    fn publish_route_state(
        &mut self,
        route: MaterializedRoute,
        commitments: Vec<RouteCommitment>,
    ) -> Result<(), RouteError> {
        let route_id = route.identity.handle.route_id;
        self.runtime_adapter()
            .persist_route(&RouterCheckpointRecord {
                route:       route.clone(),
                commitments: commitments.clone(),
            })?;
        self.active_routes.insert(route_id, route);
        self.published_commitments.insert(route_id, commitments);
        Ok(())
    }

    fn activate_with_profile(
        &mut self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
    ) -> Result<MaterializedRoute, RouteError> {
        let _ = self
            .engine
            .engine_tick(&RoutingTickContext::new(self.topology.clone()))?;
        let candidate = self
            .ordered_candidates(objective, profile)
            .into_iter()
            .next()
            .ok_or(RouteSelectionError::NoCandidate)?;
        let admission =
            self.engine
                .admit_route(objective, profile, candidate, &self.topology)?;
        if admission.admission_check.decision != AdmissionDecision::Admissible {
            return Err(RouteSelectionError::Inadmissible(
                match admission.admission_check.decision {
                    | AdmissionDecision::Rejected(reason) => reason,
                    | AdmissionDecision::Admissible => unreachable!(),
                },
            )
            .into());
        }

        let input = self.materialization_input(&admission)?;
        let installation = self.engine.materialize_route(input.clone())?;
        let route = MaterializedRoute::from_installation(input, installation);
        let route_id = route.identity.handle.route_id;
        let commitments = self.engine.route_commitments(&route);
        let record = RouterCheckpointRecord {
            route:       route.clone(),
            commitments: commitments.clone(),
        };
        if let Err(error) =
            self.runtime_adapter()
                .persist_route(&record)
                .and_then(|()| {
                    self.runtime_adapter().record_route_event(
                        jacquard_core::RouteEvent::RouteMaterialized {
                            handle: route.identity.handle.clone(),
                            proof:  route.identity.materialization_proof.clone(),
                        },
                    )
                })
        {
            self.engine.teardown(&route_id);
            return Err(error);
        }
        self.active_routes.insert(route_id, route.clone());
        self.published_commitments.insert(route_id, commitments);
        Ok(route)
    }

    fn materialization_input(
        &mut self,
        admission: &jacquard_core::RouteAdmission,
    ) -> Result<RouteMaterializationInput, RouteError> {
        let publication_id = publication_id(self.effects.next_order_stamp());
        let now = self.effects.now_tick();
        let lease = RouteLease {
            owner_node_id: self.engine.local_node_id_for_router(),
            lease_epoch:   self.topology.value.epoch,
            valid_for:     TimeWindow::new(
                now,
                Tick(now.0.saturating_add(DEFAULT_ROUTE_LEASE_TICKS)),
            )
            .map_err(|_| RouteRuntimeError::Invalidated)?,
        };
        Ok(RouteMaterializationInput {
            handle: RouteHandle {
                route_id: admission.route_id,
                topology_epoch: self.topology.value.epoch,
                materialized_at_tick: now,
                publication_id,
            },
            admission: admission.clone(),
            lease,
        })
    }

    fn ordered_candidates(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
    ) -> Vec<RouteCandidate> {
        let mut candidates =
            self.engine
                .candidate_routes(objective, profile, &self.topology);
        candidates.sort_by_key(candidate_ordering_key);
        candidates
    }

    fn expire_stale_leases(&mut self) {
        let now = self.effects.now_tick();
        let expired = self
            .active_routes
            .iter()
            .filter_map(|(route_id, route)| {
                (!route.identity.lease.is_valid_at(now)).then_some(*route_id)
            })
            .collect::<Vec<_>>();
        for route_id in expired {
            self.engine.teardown(&route_id);
            let _ = self.remove_published_route(&route_id);
        }
    }

    fn transfer_route_lease_inner(
        &mut self,
        route_id: &RouteId,
        handoff: &RouteSemanticHandoff,
    ) -> Result<MaterializedRoute, RouteError> {
        let mut route = self
            .active_routes
            .get(route_id)
            .cloned()
            .ok_or(RouteSelectionError::NoCandidate)?;
        if handoff.route_id != route.identity.handle.route_id {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        route.identity.lease.owner_node_id = handoff.to_node_id;
        route.identity.lease.lease_epoch = handoff.handoff_epoch;
        let commitments = self.engine.route_commitments(&route);
        self.publish_route_state(route.clone(), commitments)?;
        Ok(route)
    }

    fn apply_maintenance_result(
        &mut self,
        route_id: &RouteId,
        next_runtime: jacquard_core::RouteRuntimeState,
        result: RouteMaintenanceResult,
    ) -> Result<RouterMaintenanceOutcome, RouteError> {
        let canonical_mutation = match &result.outcome {
            | jacquard_core::RouteMaintenanceOutcome::ReplacementRequired {
                trigger,
            } => self.handle_replacement_required(route_id, *trigger)?,
            | jacquard_core::RouteMaintenanceOutcome::HandedOff(handoff) => self
                .handle_handoff_maintenance(route_id, next_runtime, handoff, &result)?,
            | jacquard_core::RouteMaintenanceOutcome::Failed(
                jacquard_core::RouteMaintenanceFailure::LeaseExpired,
            ) => self.handle_expired_route(route_id, &result)?,
            | _ if result.event == jacquard_core::RouteLifecycleEvent::Expired => {
                self.handle_expired_route(route_id, &result)?
            },
            | _ => self.handle_continued_route(route_id, next_runtime, &result)?,
        };

        Ok(RouterMaintenanceOutcome {
            engine_result: result,
            canonical_mutation,
        })
    }

    fn handle_replacement_required(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouterCanonicalMutation, RouteError> {
        let route = <Self as Router>::reselect_route(self, route_id, trigger)?;
        Ok(RouterCanonicalMutation::RouteReplaced {
            previous_route_id: *route_id,
            route:             Box::new(route),
        })
    }

    fn handle_handoff_maintenance(
        &mut self,
        route_id: &RouteId,
        next_runtime: jacquard_core::RouteRuntimeState,
        handoff: &RouteSemanticHandoff,
        result: &RouteMaintenanceResult,
    ) -> Result<RouterCanonicalMutation, RouteError> {
        let mut route = self
            .active_routes
            .get(route_id)
            .cloned()
            .ok_or(RouteSelectionError::NoCandidate)?;
        route.runtime = next_runtime;
        route.identity.lease.owner_node_id = handoff.to_node_id;
        route.identity.lease.lease_epoch = handoff.handoff_epoch;
        let commitments = self.engine.route_commitments(&route);
        self.persist_route_with_event(route_id, route.clone(), commitments, result)?;
        Ok(RouterCanonicalMutation::LeaseTransferred {
            route_id: *route_id,
            handoff:  handoff.clone(),
            lease:    route.identity.lease,
        })
    }

    fn handle_expired_route(
        &mut self,
        route_id: &RouteId,
        result: &RouteMaintenanceResult,
    ) -> Result<RouterCanonicalMutation, RouteError> {
        self.engine.teardown(route_id);
        self.runtime_adapter().remove_route(route_id)?;
        self.runtime_adapter().record_route_event(
            jacquard_core::RouteEvent::RouteMaintenanceCompleted {
                route_id: *route_id,
                result:   result.clone(),
            },
        )?;
        self.active_routes.remove(route_id);
        self.published_commitments.remove(route_id);
        Ok(RouterCanonicalMutation::RouteExpired { route_id: *route_id })
    }

    fn handle_continued_route(
        &mut self,
        route_id: &RouteId,
        next_runtime: jacquard_core::RouteRuntimeState,
        result: &RouteMaintenanceResult,
    ) -> Result<RouterCanonicalMutation, RouteError> {
        let mut route = self
            .active_routes
            .get(route_id)
            .cloned()
            .ok_or(RouteSelectionError::NoCandidate)?;
        route.runtime = next_runtime;
        let commitments = self.engine.route_commitments(&route);
        self.persist_route_with_event(route_id, route, commitments, result)?;
        Ok(RouterCanonicalMutation::None)
    }

    fn persist_route_with_event(
        &mut self,
        route_id: &RouteId,
        route: MaterializedRoute,
        commitments: Vec<RouteCommitment>,
        result: &RouteMaintenanceResult,
    ) -> Result<(), RouteError> {
        self.runtime_adapter()
            .persist_route(&RouterCheckpointRecord {
                route:       route.clone(),
                commitments: commitments.clone(),
            })?;
        self.runtime_adapter().record_route_event(
            jacquard_core::RouteEvent::RouteMaintenanceCompleted {
                route_id: *route_id,
                result:   result.clone(),
            },
        )?;
        self.active_routes.insert(*route_id, route);
        self.published_commitments.insert(*route_id, commitments);
        Ok(())
    }
}

impl<Engine, Policy, Effects> Router for MeshOnlyRouter<Engine, Policy, Effects>
where
    Engine: MeshRouterEngineBridge,
    Policy: PolicyEngine,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
{
    fn register_engine(
        &mut self,
        extension: Box<dyn RoutingEngine>,
    ) -> Result<(), RouteError> {
        if extension.engine_id() != self.registered_engine_id {
            return Err(CapabilityError::Unsupported.into());
        }
        Err(CapabilityError::Rejected.into())
    }

    fn activate_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<MaterializedRoute, RouteError> {
        let profile = self
            .policy_engine
            .compute_profile(&objective, &self.policy_inputs);
        self.activate_with_profile(&objective, &profile)
    }

    fn route_commitments(
        &self,
        route_id: &RouteId,
    ) -> Result<Vec<RouteCommitment>, RouteError> {
        let commitments = self
            .published_commitments
            .get(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        Ok(commitments.clone())
    }

    fn reselect_route(
        &mut self,
        route_id: &RouteId,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<MaterializedRoute, RouteError> {
        let objective = self
            .active_routes
            .get(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?
            .identity
            .admission
            .objective
            .clone();
        self.engine.teardown(route_id);
        self.remove_published_route(route_id)?;
        let profile = self
            .policy_engine
            .compute_profile(&objective, &self.policy_inputs);
        self.activate_with_profile(&objective, &profile)
    }

    fn transfer_route_lease(
        &mut self,
        route_id: &RouteId,
        handoff: RouteSemanticHandoff,
    ) -> Result<MaterializedRoute, RouteError> {
        self.transfer_route_lease_inner(route_id, &handoff)
    }
}

impl<Engine, Policy, Effects> RoutingControlPlane
    for MeshOnlyRouter<Engine, Policy, Effects>
where
    Engine: MeshRouterEngineBridge,
    Policy: PolicyEngine,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
{
    fn activate_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<MaterializedRoute, RouteError> {
        <Self as Router>::activate_route(self, objective)
    }

    fn maintain_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouterMaintenanceOutcome, RouteError> {
        let (identity, mut next_runtime) = {
            let route = self
                .active_routes
                .get(route_id)
                .ok_or(RouteSelectionError::NoCandidate)?;
            (route.identity.clone(), route.runtime.clone())
        };
        let result =
            self.engine
                .maintain_route(&identity, &mut next_runtime, trigger)?;
        self.apply_maintenance_result(route_id, next_runtime, result)
    }

    fn anti_entropy_tick(&mut self) -> Result<RouterTickOutcome, RouteError> {
        let outcome = self
            .engine
            .engine_tick(&RoutingTickContext::new(self.topology.clone()))?;
        let mut canonical_mutation = RouterCanonicalMutation::None;
        if outcome.change == RoutingTickChange::PrivateStateUpdated {
            let expired_route_id =
                self.active_routes.iter().find_map(|(route_id, route)| {
                    (!route.identity.lease.is_valid_at(self.effects.now_tick()))
                        .then_some(*route_id)
                });
            self.expire_stale_leases();
            if let Some(route_id) = expired_route_id {
                canonical_mutation = RouterCanonicalMutation::RouteExpired { route_id };
            }
        }
        Ok(RouterTickOutcome {
            topology_epoch: outcome.topology_epoch,
            engine_change: outcome.change,
            canonical_mutation,
        })
    }
}

impl<Engine, Policy, Effects> RoutingDataPlane
    for MeshOnlyRouter<Engine, Policy, Effects>
where
    Engine: MeshRouterEngineBridge,
    Policy: PolicyEngine,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
{
    fn forward_payload(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        self.engine.forward_payload_for_router(route_id, payload)
    }

    fn observe_route_health(
        &self,
        route_id: &RouteId,
    ) -> Result<Observation<RouteHealth>, RouteError> {
        let route = self
            .active_routes
            .get(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        Ok(Observation {
            value:                 route.runtime.health.clone(),
            source_class:          FactSourceClass::Local,
            evidence_class:        RoutingEvidenceClass::AdmissionWitnessed,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick:      self.effects.now_tick(),
        })
    }
}

fn publication_id(order: OrderStamp) -> PublicationId {
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&order.0.to_le_bytes());
    PublicationId(bytes)
}

type CandidateOrderingKey = (
    Reverse<RouteProtectionClass>,
    Reverse<RouteRepairClass>,
    Reverse<RoutePartitionClass>,
    RouteDegradation,
    Belief<u8>,
    Vec<TransportProtocol>,
    RoutingEngineId,
    Vec<u8>,
);

fn candidate_ordering_key(candidate: &RouteCandidate) -> CandidateOrderingKey {
    (
        Reverse(candidate.summary.protection),
        Reverse(candidate.summary.connectivity.repair),
        Reverse(candidate.summary.connectivity.partition),
        candidate.estimate.value.degradation,
        candidate.summary.hop_count_hint,
        candidate.summary.protocol_mix.clone(),
        candidate.backend_ref.engine.clone(),
        candidate.backend_ref.backend_route_id.0.clone(),
    )
}
