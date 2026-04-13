//! Generic multi-engine router middleware over the shared routing traits.
//!
//! Control flow: this module owns the cross-engine orchestration
//! loop. Activation goes `objective -> policy profile -> tick registered
//! engines -> gather candidates across engines -> select one ordered candidate
//! -> admit/materialize through the owning engine -> publish router-owned
//! canonical state`. Maintenance, re-selection, expiry, and anti-entropy reuse
//! the same engine registry and always mutate canonical state on the router
//! side, even when one selected engine performs the route-private work.
//! Candidate ordering, admission, and publication operate on shared summaries,
//! checks, and installation evidence; they do not require explicit hop-by-hop
//! path disclosure from every engine.
//!
//! Ownership:
//! - canonical route mutations and registry-level engine dispatch happen here
//! - registered engines return typed evidence and opaque private runtime state

use std::{cmp::Reverse, collections::BTreeMap};

use jacquard_core::{
    AdmissionDecision, Belief, CapabilityError, Configuration, FactSourceClass, MaterializedRoute,
    Observation, OrderStamp, OriginAuthenticationClass, PublicationId, RouteCandidate,
    RouteCommitment, RouteDegradation, RouteError, RouteHandle, RouteHealth, RouteId,
    RouteIdentityStamp, RouteLease, RouteMaintenanceResult, RouteMaintenanceTrigger,
    RouteMaterializationInput, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
    RouteRuntimeError, RouteSelectionError, RouteSemanticHandoff, RouterCanonicalMutation,
    RouterMaintenanceOutcome, RouterRoundOutcome, RoutingEngineCapabilities, RoutingEngineId,
    RoutingEvidenceClass, RoutingObjective, RoutingPolicyInputs, RoutingTickChange,
    RoutingTickContext, RoutingTickHint, SelectedRoutingParameters, Tick, TimeWindow,
    TransportKind, TransportObservation,
};
use jacquard_traits::{
    OrderEffects, PolicyEngine, RouteEventLogEffects, Router, RouterEngineRegistry,
    RouterManagedEngine, RoutingControlPlane, RoutingDataPlane, RoutingMiddleware, StorageEffects,
    TimeEffects,
};

use crate::runtime::{RouterCheckpointRecord, RouterRuntimeAdapter};

const DEFAULT_ROUTE_LEASE_TICKS: u64 = 32;

#[derive(Clone, Debug)]
pub struct FixedPolicyEngine {
    profile: SelectedRoutingParameters,
}

impl FixedPolicyEngine {
    #[must_use]
    pub fn new(profile: SelectedRoutingParameters) -> Self {
        Self { profile }
    }
}

impl PolicyEngine for FixedPolicyEngine {
    fn compute_profile(
        &self,
        _objective: &RoutingObjective,
        _inputs: &RoutingPolicyInputs,
    ) -> SelectedRoutingParameters {
        self.profile.clone()
    }
}

struct RegisteredEngine {
    capabilities: RoutingEngineCapabilities,
    engine: Box<dyn RouterManagedEngine>,
}

pub struct MultiEngineRouter<Policy, Effects> {
    local_node_id: jacquard_core::NodeId,
    registered_engines: BTreeMap<RoutingEngineId, RegisteredEngine>,
    policy_engine: Policy,
    effects: Effects,
    topology: Observation<Configuration>,
    policy_inputs: RoutingPolicyInputs,
    active_routes: BTreeMap<RouteId, MaterializedRoute>,
    published_commitments: BTreeMap<RouteId, Vec<RouteCommitment>>,
}

impl<Policy, Effects> MultiEngineRouter<Policy, Effects>
where
    Policy: PolicyEngine,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
{
    #[must_use]
    pub fn new(
        local_node_id: jacquard_core::NodeId,
        policy_engine: Policy,
        effects: Effects,
        topology: Observation<Configuration>,
        policy_inputs: RoutingPolicyInputs,
    ) -> Self {
        Self {
            local_node_id,
            registered_engines: BTreeMap::new(),
            policy_engine,
            effects,
            topology,
            policy_inputs,
            active_routes: BTreeMap::new(),
            published_commitments: BTreeMap::new(),
        }
    }

    pub fn register_engine(
        &mut self,
        extension: Box<dyn RouterManagedEngine>,
    ) -> Result<(), RouteError> {
        let engine_id = extension.engine_id();
        if extension.local_node_id_for_router() != self.local_node_id {
            return Err(CapabilityError::Rejected.into());
        }
        if self.registered_engines.contains_key(&engine_id) {
            return Err(CapabilityError::Rejected.into());
        }
        let capabilities = extension.capabilities();
        self.registered_engines.insert(
            engine_id,
            RegisteredEngine {
                capabilities,
                engine: extension,
            },
        );
        Ok(())
    }

    pub fn ingest_topology_observation(&mut self, topology: Observation<Configuration>) {
        self.topology = topology;
    }

    pub fn ingest_policy_inputs(&mut self, inputs: RoutingPolicyInputs) {
        self.policy_inputs = inputs;
    }

    pub fn ingest_transport_observation(
        &mut self,
        observation: &TransportObservation,
    ) -> Result<(), RouteError> {
        for entry in self.registered_engines.values_mut() {
            entry
                .engine
                .ingest_transport_observation_for_router(observation)?;
        }
        Ok(())
    }

    pub fn recover_checkpointed_routes(&mut self) -> Result<usize, RouteError> {
        let records =
            RouterRuntimeAdapter::new(self.local_node_id, &mut self.effects).load_routes()?;
        let mut recovered = 0usize;
        for (route_id, record) in records {
            let engine_id = record.route.identity.admission.summary.engine.clone();
            let restored = match self.registered_engines.get_mut(&engine_id) {
                Some(entry) => entry.engine.restore_route_runtime_for_router(&route_id)?,
                None => false,
            };
            if !restored {
                RouterRuntimeAdapter::new(self.local_node_id, &mut self.effects)
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
    pub fn effects(&self) -> &Effects {
        &self.effects
    }

    pub fn effects_mut(&mut self) -> &mut Effects {
        &mut self.effects
    }

    #[must_use]
    pub fn local_node_id(&self) -> jacquard_core::NodeId {
        self.local_node_id
    }

    #[must_use]
    pub fn registered_engine_ids(&self) -> Vec<RoutingEngineId> {
        self.registered_engines.keys().cloned().collect()
    }

    #[must_use]
    pub fn registered_engine_capabilities(
        &self,
        engine_id: &RoutingEngineId,
    ) -> Option<RoutingEngineCapabilities> {
        self.registered_engines
            .get(engine_id)
            .map(|entry| entry.capabilities.clone())
    }

    #[must_use]
    pub fn active_route(&self, route_id: &RouteId) -> Option<&MaterializedRoute> {
        self.active_routes.get(route_id)
    }

    #[must_use]
    pub fn active_route_count(&self) -> usize {
        self.active_routes.len()
    }

    #[must_use]
    pub fn active_routes_snapshot(&self) -> Vec<MaterializedRoute> {
        let mut routes = self.active_routes.values().cloned().collect::<Vec<_>>();
        routes.sort_by_key(|route| route.identity.stamp.route_id);
        routes
    }

    fn current_policy_inputs(&self) -> RoutingPolicyInputs {
        let mut inputs = self.policy_inputs.clone();
        inputs.routing_engine_count =
            u32::try_from(self.registered_engines.len()).unwrap_or(u32::MAX);
        inputs
    }

    fn runtime_adapter(&mut self) -> RouterRuntimeAdapter<'_, Effects> {
        RouterRuntimeAdapter::new(self.local_node_id, &mut self.effects)
    }

    // T8: get/get_mut variants intentionally separate due to borrow-checker
    // requirements.
    fn engine_for_id(
        &self,
        engine_id: &RoutingEngineId,
    ) -> Result<&(dyn RouterManagedEngine + '_), RouteError> {
        if let Some(entry) = self.registered_engines.get(engine_id) {
            Ok(entry.engine.as_ref())
        } else {
            Err(CapabilityError::Unsupported.into())
        }
    }

    fn engine_for_id_mut(
        &mut self,
        engine_id: &RoutingEngineId,
    ) -> Result<&mut (dyn RouterManagedEngine + '_), RouteError> {
        if let Some(entry) = self.registered_engines.get_mut(engine_id) {
            Ok(entry.engine.as_mut())
        } else {
            Err(CapabilityError::Unsupported.into())
        }
    }

    fn route_engine_id(&self, route_id: &RouteId) -> Result<RoutingEngineId, RouteError> {
        self.active_routes
            .get(route_id)
            .map(|route| route.identity.admission.summary.engine.clone())
            .ok_or(RouteSelectionError::NoCandidate.into())
    }

    fn route_commitments_for(
        &self,
        route: &MaterializedRoute,
    ) -> Result<Vec<RouteCommitment>, RouteError> {
        let engine_id = route.identity.admission.summary.engine.clone();
        Ok(self.engine_for_id(&engine_id)?.route_commitments(route))
    }

    fn advance_all_engines(&mut self) -> Result<(RoutingTickChange, RoutingTickHint), RouteError> {
        let tick = RoutingTickContext::new(self.topology.clone());
        let mut aggregate = RoutingTickChange::NoChange;
        let mut hint = RoutingTickHint::HostDefault;
        for entry in self.registered_engines.values_mut() {
            let outcome = entry.engine.engine_tick(&tick)?;
            if outcome.change == RoutingTickChange::PrivateStateUpdated {
                aggregate = RoutingTickChange::PrivateStateUpdated;
            }
            hint = hint.more_urgent(outcome.next_tick_hint);
        }
        Ok((aggregate, hint))
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
        let route_id = route.identity.stamp.route_id;
        self.runtime_adapter()
            .persist_route(&RouterCheckpointRecord {
                route: route.clone(),
                commitments: commitments.clone(),
            })?;
        self.active_routes.insert(route_id, route);
        self.published_commitments.insert(route_id, commitments);
        Ok(())
    }

    // long-block-exception: activation is one fail-closed canonical route
    // publication path from candidate selection through checkpointing.
    fn activate_with_profile(
        &mut self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
    ) -> Result<MaterializedRoute, RouteError> {
        self.advance_all_engines()?;
        let candidate = self
            .ordered_candidates(objective, profile)
            .into_iter()
            .next()
            .ok_or(RouteSelectionError::NoCandidate)?;
        let route_id = candidate.route_id;
        let engine_id = candidate.backend_ref.engine.clone();
        let admission = self.engine_for_id(&engine_id)?.admit_route(
            objective,
            profile,
            candidate,
            &self.topology,
        )?;
        if admission.admission_check.decision != AdmissionDecision::Admissible {
            return Err(RouteSelectionError::Inadmissible(
                match admission.admission_check.decision {
                    AdmissionDecision::Rejected(reason) => reason,
                    AdmissionDecision::Admissible => unreachable!(),
                },
            )
            .into());
        }

        let input = self.materialization_input(route_id, &admission)?;
        let route_id = *input.handle.route_id();
        let installation = self
            .engine_for_id_mut(&engine_id)?
            .materialize_route(input.clone())?;
        let route = MaterializedRoute::from_installation(input, installation);
        let commitments = self.route_commitments_for(&route)?;
        let record = RouterCheckpointRecord {
            route: route.clone(),
            commitments: commitments.clone(),
        };
        if let Err(error) = self
            .runtime_adapter()
            .persist_route(&record)
            .and_then(|()| {
                self.runtime_adapter().record_route_event(
                    jacquard_core::RouteEvent::RouteMaterialized {
                        handle: jacquard_core::RouteHandle {
                            stamp: route.identity.stamp.clone(),
                        },
                        proof: route.identity.proof.clone(),
                    },
                )
            })
        {
            self.engine_for_id_mut(&engine_id)?.teardown(&route_id);
            return Err(error);
        }
        self.active_routes.insert(route_id, route.clone());
        self.published_commitments.insert(route_id, commitments);
        Ok(route)
    }

    fn materialization_input(
        &mut self,
        route_id: RouteId,
        admission: &jacquard_core::RouteAdmission,
    ) -> Result<RouteMaterializationInput, RouteError> {
        let publication_id = publication_id(self.effects.next_order_stamp());
        let now = self.effects.now_tick();
        let lease = RouteLease {
            owner_node_id: self.local_node_id,
            lease_epoch: self.topology.value.epoch,
            valid_for: TimeWindow::new(now, Tick(now.0.saturating_add(DEFAULT_ROUTE_LEASE_TICKS)))
                .map_err(|_| RouteRuntimeError::Invalidated)?,
        };
        Ok(RouteMaterializationInput {
            handle: RouteHandle {
                stamp: RouteIdentityStamp {
                    route_id,
                    topology_epoch: self.topology.value.epoch,
                    materialized_at_tick: now,
                    publication_id,
                },
            },
            admission: admission.clone(),
            lease,
        })
    }

    fn ordered_candidates(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
    ) -> Vec<RouteCandidate> {
        let mut candidates = self
            .registered_engines
            .values()
            .flat_map(|entry| {
                entry
                    .engine
                    .candidate_routes(objective, profile, &self.topology)
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| candidate_ordering_key(candidate, profile));
        candidates
    }

    fn expire_stale_leases(&mut self) -> Result<Option<RouteId>, RouteError> {
        let now = self.effects.now_tick();
        let expired = self
            .active_routes
            .iter()
            .filter_map(|(route_id, route)| {
                (!route.identity.lease.is_valid_at(now)).then_some(*route_id)
            })
            .collect::<Vec<_>>();
        let first = expired.first().copied();
        for route_id in expired {
            let engine_id = self.route_engine_id(&route_id)?;
            self.engine_for_id_mut(&engine_id)?.teardown(&route_id);
            self.remove_published_route(&route_id)?;
        }
        Ok(first)
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
        if handoff.route_id != route.identity.stamp.route_id {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        route.identity.lease.owner_node_id = handoff.to_node_id;
        route.identity.lease.lease_epoch = handoff.handoff_epoch;
        let commitments = self.route_commitments_for(&route)?;
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
            jacquard_core::RouteMaintenanceOutcome::ReplacementRequired { trigger } => {
                self.handle_replacement_required(route_id, *trigger)?
            }
            jacquard_core::RouteMaintenanceOutcome::HandedOff(handoff) => {
                self.handle_handoff_maintenance(route_id, next_runtime, handoff, &result)?
            }
            jacquard_core::RouteMaintenanceOutcome::Failed(
                jacquard_core::RouteMaintenanceFailure::LeaseExpired,
            ) => self.handle_expired_route(route_id, &result)?,
            _ if result.event == jacquard_core::RouteLifecycleEvent::Expired => {
                self.handle_expired_route(route_id, &result)?
            }
            _ => self.handle_continued_route(route_id, next_runtime, &result)?,
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
            route: Box::new(route),
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
        let commitments = self.route_commitments_for(&route)?;
        self.persist_route_with_event(route_id, route.clone(), commitments, result)?;
        Ok(RouterCanonicalMutation::LeaseTransferred {
            route_id: *route_id,
            handoff: handoff.clone(),
            lease: route.identity.lease,
        })
    }

    fn handle_expired_route(
        &mut self,
        route_id: &RouteId,
        result: &RouteMaintenanceResult,
    ) -> Result<RouterCanonicalMutation, RouteError> {
        let engine_id = self.route_engine_id(route_id)?;
        self.engine_for_id_mut(&engine_id)?.teardown(route_id);
        self.runtime_adapter().remove_route(route_id)?;
        self.runtime_adapter().record_route_event(
            jacquard_core::RouteEvent::RouteMaintenanceCompleted {
                route_id: *route_id,
                result: result.clone(),
            },
        )?;
        self.active_routes.remove(route_id);
        self.published_commitments.remove(route_id);
        Ok(RouterCanonicalMutation::RouteExpired {
            route_id: *route_id,
        })
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
        let commitments = self.route_commitments_for(&route)?;
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
                route: route.clone(),
                commitments: commitments.clone(),
            })?;
        self.runtime_adapter().record_route_event(
            jacquard_core::RouteEvent::RouteMaintenanceCompleted {
                route_id: *route_id,
                result: result.clone(),
            },
        )?;
        self.active_routes.insert(*route_id, route);
        self.published_commitments.insert(*route_id, commitments);
        Ok(())
    }
}

impl<Policy, Effects> RouterEngineRegistry for MultiEngineRouter<Policy, Effects>
where
    Policy: PolicyEngine,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
{
    fn register_engine(
        &mut self,
        extension: Box<dyn RouterManagedEngine>,
    ) -> Result<(), RouteError> {
        Self::register_engine(self, extension)
    }

    fn registered_engine_ids(&self) -> Vec<RoutingEngineId> {
        Self::registered_engine_ids(self)
    }

    fn registered_engine_capabilities(
        &self,
        engine_id: &RoutingEngineId,
    ) -> Option<RoutingEngineCapabilities> {
        Self::registered_engine_capabilities(self, engine_id)
    }
}

impl<Policy, Effects> RoutingMiddleware for MultiEngineRouter<Policy, Effects>
where
    Policy: PolicyEngine,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
{
    fn ingest_topology_observation(&mut self, topology: Observation<Configuration>) {
        Self::ingest_topology_observation(self, topology);
    }

    fn ingest_policy_inputs(&mut self, inputs: RoutingPolicyInputs) {
        Self::ingest_policy_inputs(self, inputs);
    }

    fn ingest_transport_observation(
        &mut self,
        observation: &TransportObservation,
    ) -> Result<(), RouteError> {
        Self::ingest_transport_observation(self, observation)
    }

    fn recover_checkpointed_routes(&mut self) -> Result<usize, RouteError> {
        Self::recover_checkpointed_routes(self)
    }
}

impl<Policy, Effects> Router for MultiEngineRouter<Policy, Effects>
where
    Policy: PolicyEngine,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
{
    fn activate_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<MaterializedRoute, RouteError> {
        let profile = self
            .policy_engine
            .compute_profile(&objective, &self.current_policy_inputs());
        self.activate_with_profile(&objective, &profile)
    }

    fn route_commitments(&self, route_id: &RouteId) -> Result<Vec<RouteCommitment>, RouteError> {
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
        let engine_id = self.route_engine_id(route_id)?;
        self.engine_for_id_mut(&engine_id)?.teardown(route_id);
        self.remove_published_route(route_id)?;
        let profile = self
            .policy_engine
            .compute_profile(&objective, &self.current_policy_inputs());
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

impl<Policy, Effects> RoutingControlPlane for MultiEngineRouter<Policy, Effects>
where
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
        let (identity, mut next_runtime, engine_id) = {
            let route = self
                .active_routes
                .get(route_id)
                .ok_or(RouteSelectionError::NoCandidate)?;
            (
                route.identity.clone(),
                route.runtime.clone(),
                route.identity.admission.summary.engine.clone(),
            )
        };
        let result = self.engine_for_id_mut(&engine_id)?.maintain_route(
            &identity,
            &mut next_runtime,
            trigger,
        )?;
        self.apply_maintenance_result(route_id, next_runtime, result)
    }

    fn advance_round(&mut self) -> Result<RouterRoundOutcome, RouteError> {
        let (aggregate, tick_hint) = self.advance_all_engines()?;
        let expired_route_id = if aggregate == RoutingTickChange::PrivateStateUpdated {
            self.expire_stale_leases()?
        } else {
            None
        };
        let canonical_mutation = expired_route_id
            .map(|route_id| RouterCanonicalMutation::RouteExpired { route_id })
            .unwrap_or(RouterCanonicalMutation::None);
        Ok(RouterRoundOutcome {
            topology_epoch: self.topology.value.epoch,
            engine_change: aggregate,
            next_round_hint: tick_hint,
            canonical_mutation,
        })
    }
}

impl<Policy, Effects> RoutingDataPlane for MultiEngineRouter<Policy, Effects>
where
    Policy: PolicyEngine,
    Effects: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects,
{
    fn forward_payload(&mut self, route_id: &RouteId, payload: &[u8]) -> Result<(), RouteError> {
        let engine_id = self.route_engine_id(route_id)?;
        self.engine_for_id_mut(&engine_id)?
            .forward_payload_for_router(route_id, payload)
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
            value: route.runtime.health.clone(),
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::AdmissionWitnessed,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: self.effects.now_tick(),
        })
    }
}

fn publication_id(order: OrderStamp) -> PublicationId {
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&order.0.to_le_bytes());
    PublicationId(bytes)
}

type CandidateOrderingKey = (
    Reverse<bool>,
    Reverse<bool>,
    Reverse<bool>,
    Reverse<RouteProtectionClass>,
    Reverse<RouteRepairClass>,
    Reverse<RoutePartitionClass>,
    RouteDegradation,
    Belief<u8>,
    Vec<TransportKind>,
    RoutingEngineId,
    Vec<u8>,
);

fn candidate_ordering_key(
    candidate: &RouteCandidate,
    profile: &SelectedRoutingParameters,
) -> CandidateOrderingKey {
    (
        Reverse(candidate.summary.protection == profile.selected_protection),
        Reverse(candidate.summary.connectivity.repair == profile.selected_connectivity.repair),
        Reverse(
            candidate.summary.connectivity.partition == profile.selected_connectivity.partition,
        ),
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
