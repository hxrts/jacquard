use super::*;

impl<Transport, Effects> RouterManagedEngine for FieldEngine<Transport, Effects>
where
    Transport: jacquard_traits::TransportSenderEffects,
{
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id
    }

    fn ingest_transport_observation_for_router(
        &mut self,
        observation: &TransportObservation,
    ) -> Result<(), RouteError> {
        let TransportObservation::PayloadReceived {
            from_node_id,
            payload,
            observed_at_tick,
            ..
        } = observation
        else {
            return Ok(());
        };
        if payload.len() != FIELD_SUMMARY_ENCODING_BYTES {
            return Ok(());
        }
        let payload: [u8; FIELD_SUMMARY_ENCODING_BYTES] = payload
            .as_slice()
            .try_into()
            .map_err(|_| RouteRuntimeError::Invalidated)?;
        let _ignored_non_field_payload = self
            .ingest_forward_summary(*from_node_id, payload, *observed_at_tick)
            .is_err();
        Ok(())
    }

    fn forward_payload_for_router(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        let active = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        let mut candidates =
            Vec::with_capacity(active.continuation_neighbors.len().saturating_add(1));
        candidates.push(active.selected_neighbor);
        candidates.extend(
            active
                .continuation_neighbors
                .iter()
                .copied()
                .filter(|neighbor| *neighbor != active.selected_neighbor),
        );
        for neighbor in candidates {
            let Some(endpoint) = self.state.neighbor_endpoints.get(&neighbor) else {
                continue;
            };
            self.transport.send_transport(endpoint, payload)?;
            if neighbor != active.selected_neighbor {
                active.selected_neighbor = neighbor;
                // allow-ignored-result: forwarding stays productive even if the observational protocol reconfiguration marker cannot be retained.
                let _ = self.reconfigure_route_protocol_session(
                    route_id,
                    neighbor,
                    FieldProtocolReconfigurationCause::ContinuationShift,
                    self.state.last_tick_processed,
                );
            }
            return Ok(());
        }
        Err(RouteSelectionError::NoCandidate.into())
    }

    fn restore_route_runtime_for_router(&mut self, route_id: &RouteId) -> Result<bool, RouteError> {
        if !self.active_routes.contains_key(route_id) {
            return Ok(false);
        }
        let needs_restore = self
            .active_routes
            .get(route_id)
            .is_some_and(|active| active.coordination_capability.is_none());
        if needs_restore {
            if let Some(checkpoint) = self.take_route_checkpoint(route_id) {
                self.restore_route_protocol_session(route_id, checkpoint)?;
            } else {
                self.note_route_without_checkpoint(route_id)?;
                let topology_epoch = self
                    .active_routes
                    .get(route_id)
                    .expect("route presence checked")
                    .topology_epoch;
                self.install_route_protocol_session(
                    route_id,
                    topology_epoch,
                    self.state.last_tick_processed,
                )?;
                self.note_fresh_route_runtime_install(route_id)?;
            }
        }
        Ok(true)
    }

    fn analysis_snapshot_for_router(
        &self,
        active_routes: &[jacquard_core::MaterializedRoute],
    ) -> Option<Box<dyn std::any::Any>> {
        Some(Box::new(self.exported_replay_bundle(active_routes)))
    }
}

pub(super) fn destination_objective_class(
    destination: &DestinationId,
) -> crate::engine::FieldReducedObjectiveClass {
    match destination {
        DestinationId::Node(_) => crate::engine::FieldReducedObjectiveClass::Node,
        DestinationId::Gateway(_) => crate::engine::FieldReducedObjectiveClass::Gateway,
        DestinationId::Service(_) => crate::engine::FieldReducedObjectiveClass::Service,
    }
}

pub(super) fn owner_tag_for_neighbor(neighbor: NodeId) -> u64 {
    u64::from_le_bytes(
        neighbor.0[..8]
            .try_into()
            .expect("node id prefix is 8 bytes"),
    )
}

pub(super) fn bound_task_for_route(route_id: &RouteId) -> u64 {
    u64::from_le_bytes(
        route_id.0[..8]
            .try_into()
            .expect("route id prefix is 8 bytes"),
    )
}

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    pub fn suspend_route_runtime_for_recovery(
        &mut self,
        route_id: &RouteId,
    ) -> Result<bool, RouteError> {
        let Some(capability) = self
            .active_routes
            .get(route_id)
            .and_then(|active| active.coordination_capability.clone())
        else {
            return Ok(self.active_routes.contains_key(route_id));
        };
        let checkpoint = match self.protocol_runtime.checkpoint_session(&capability) {
            Ok(checkpoint) => checkpoint,
            Err(_) => {
                self.note_route_recovery_failed(
                    route_id,
                    FieldRouteRecoveryTrigger::SuspendForRuntimeLoss,
                )?;
                return Err(RouteRuntimeError::Invalidated.into());
            }
        };
        let _closed = match self.protocol_runtime.close_session(&capability) {
            Ok(closed) => closed,
            Err(_) => {
                self.note_route_recovery_failed(
                    route_id,
                    FieldRouteRecoveryTrigger::SuspendForRuntimeLoss,
                )?;
                return Err(RouteRuntimeError::Invalidated.into());
            }
        };
        let active = self
            .active_routes
            .get_mut(route_id)
            .expect("route remains present during recovery suspend");
        active.coordination_capability = None;
        active.recovery.note_checkpoint_stored(checkpoint);
        Ok(true)
    }

    pub(super) fn install_route_protocol_session(
        &mut self,
        route_id: &RouteId,
        topology_epoch: jacquard_core::RouteEpoch,
        _now_tick: Tick,
    ) -> Result<(), RouteError> {
        let Some(active) = self.active_routes.get(route_id) else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        let destination = DestinationId::from(&active.destination);
        let session_key = FieldProtocolSessionKey {
            protocol: FieldProtocolKind::ExplicitCoordination,
            route_id: Some(*route_id),
            topology_epoch,
            destination: Some(SummaryDestinationKey::from(&destination)),
        };
        let capability = self
            .protocol_runtime
            .open_session(
                &session_key,
                owner_tag_for_neighbor(active.selected_neighbor),
                Some(bound_task_for_route(route_id)),
            )
            .map_err(|_| RouteRuntimeError::Invalidated)?;
        let active = self
            .active_routes
            .get_mut(route_id)
            .expect("route remains present during session install");
        active.coordination_capability = Some(capability);
        Ok(())
    }

    pub(super) fn restore_route_protocol_session(
        &mut self,
        route_id: &RouteId,
        checkpoint: FieldProtocolCheckpoint,
    ) -> Result<(), RouteError> {
        self.validate_route_checkpoint(route_id, &checkpoint)?;
        let capability = match self.protocol_runtime.restore_session(checkpoint) {
            Ok(capability) => capability,
            Err(_) => {
                self.note_route_recovery_failed(
                    route_id,
                    FieldRouteRecoveryTrigger::RestoreRuntime,
                )?;
                return Err(RouteRuntimeError::Invalidated.into());
            }
        };
        let active = self
            .active_routes
            .get_mut(route_id)
            .expect("route remains present during checkpoint restore");
        active.coordination_capability = Some(capability);
        active.recovery.note_checkpoint_restored();
        Ok(())
    }

    pub(super) fn reconfigure_route_protocol_session(
        &mut self,
        route_id: &RouteId,
        new_neighbor: NodeId,
        cause: FieldProtocolReconfigurationCause,
        now_tick: Tick,
    ) -> Result<(), RouteError> {
        let capability = self
            .active_routes
            .get(route_id)
            .and_then(|active| active.coordination_capability.clone())
            .ok_or(RouteRuntimeError::Invalidated)?;
        let updated = self
            .protocol_runtime
            .transfer_owner_with_cause(
                &capability,
                owner_tag_for_neighbor(new_neighbor),
                Some(bound_task_for_route(route_id)),
                cause,
                now_tick,
            )
            .map_err(|_| RouteRuntimeError::Invalidated)?;
        let active = self
            .active_routes
            .get_mut(route_id)
            .expect("route remains present during session reconfiguration");
        active.coordination_capability = Some(updated);
        if cause == FieldProtocolReconfigurationCause::ContinuationShift {
            active.recovery.note_continuation_retained();
        }
        Ok(())
    }

    pub(super) fn close_route_protocol_session(
        &mut self,
        route_id: &RouteId,
    ) -> Result<(), RouteError> {
        let Some(capability) = self
            .active_routes
            .get(route_id)
            .and_then(|active| active.coordination_capability.clone())
        else {
            return Ok(());
        };
        let _closed = self
            .protocol_runtime
            .close_session(&capability)
            .map_err(|_| RouteRuntimeError::Invalidated)?;
        Ok(())
    }

    pub(super) fn take_route_checkpoint(
        &mut self,
        route_id: &RouteId,
    ) -> Option<FieldProtocolCheckpoint> {
        self.active_routes
            .get_mut(route_id)
            .and_then(|active| active.recovery.checkpoint.take())
    }

    pub(super) fn note_route_without_checkpoint(
        &mut self,
        route_id: &RouteId,
    ) -> Result<(), RouteError> {
        let active = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        active.recovery.note_no_checkpoint_available();
        Ok(())
    }

    pub(super) fn note_fresh_route_runtime_install(
        &mut self,
        route_id: &RouteId,
    ) -> Result<(), RouteError> {
        let active = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        active.recovery.note_fresh_session_installed();
        Ok(())
    }

    pub(super) fn note_route_recovery_failed(
        &mut self,
        route_id: &RouteId,
        trigger: FieldRouteRecoveryTrigger,
    ) -> Result<(), RouteError> {
        let active = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        active.recovery.note_recovery_failed(trigger);
        Ok(())
    }

    pub(super) fn validate_route_checkpoint(
        &mut self,
        route_id: &RouteId,
        checkpoint: &FieldProtocolCheckpoint,
    ) -> Result<(), RouteError> {
        let Some(active) = self.active_routes.get_mut(route_id) else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        let destination = DestinationId::from(&active.destination);
        let expected_session = FieldProtocolSessionKey {
            protocol: FieldProtocolKind::ExplicitCoordination,
            route_id: Some(*route_id),
            topology_epoch: active.topology_epoch,
            destination: Some(SummaryDestinationKey::from(&destination)),
        };
        let expected_owner = owner_tag_for_neighbor(active.selected_neighbor);
        if checkpoint.session != expected_session || checkpoint.owner_tag != expected_owner {
            active
                .recovery
                .note_recovery_failed(FieldRouteRecoveryTrigger::RestoreRuntime);
            return Err(RouteRuntimeError::Invalidated.into());
        }
        Ok(())
    }
}
