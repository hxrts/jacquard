//! Host bridge for the reference client.
//!
//! The bridge is the only surface in this crate that may:
//! - own a transport driver
//! - stamp raw ingress with Jacquard logical time
//! - flush queued outbound transport commands to the driver
//! - advance the router through synchronous rounds
//!
//! Tests and examples should bind an owner with [`HostBridge::bind`] and then
//! drive the router through that owner rather than mutating transport drivers
//! or calling `advance_round` on the router directly.

use std::collections::VecDeque;

use jacquard_adapter::{
    dispatch_mailbox, DispatchOverflow, DispatchReceiver, DispatchSender,
};
use jacquard_core::{
    Configuration, LinkEndpoint, Observation, RouteError, RouterRoundOutcome, Tick,
    TransportError, TransportIngressEvent, TransportObservation,
};
use jacquard_mem_link_profile::InMemoryTransport;
use jacquard_traits::{
    effect_handler, RoutingControlPlane, TransportDriver, TransportSenderEffects,
};

use crate::PathwayRouter;

/// Default queue capacities used by the reference client bridge.
pub(crate) const DEFAULT_BRIDGE_QUEUE_CONFIG: BridgeQueueConfig =
    BridgeQueueConfig::new(64, 64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeQueueConfig {
    pub inbound_capacity: usize,
    pub outbound_capacity_per_engine: usize,
}

impl BridgeQueueConfig {
    #[must_use]
    pub const fn new(
        inbound_capacity: usize,
        outbound_capacity_per_engine: usize,
    ) -> Self {
        Self {
            inbound_capacity,
            outbound_capacity_per_engine,
        }
    }
}

impl Default for BridgeQueueConfig {
    fn default() -> Self {
        DEFAULT_BRIDGE_QUEUE_CONFIG
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OutboundTransportCommand {
    endpoint: LinkEndpoint,
    payload: Vec<u8>,
}

// Decouples engine send calls from driver I/O: engines enqueue during a round
// and the bridge flushes to the driver in one pass after advance_round returns.
#[derive(Clone)]
pub(crate) struct QueuedTransportSender {
    queue: DispatchSender<OutboundTransportCommand>,
}

impl QueuedTransportSender {
    #[must_use]
    fn new(queue: DispatchSender<OutboundTransportCommand>) -> Self {
        Self { queue }
    }
}

#[effect_handler]
impl TransportSenderEffects for QueuedTransportSender {
    fn send_transport(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.queue
            .send(OutboundTransportCommand {
                endpoint: endpoint.clone(),
                payload: payload.to_vec(),
            })
            .map(|_| ())
            .map_err(|DispatchOverflow| TransportError::Unavailable)
    }
}

pub(crate) struct BridgeTransport {
    driver: InMemoryTransport,
    outbound_sender: DispatchSender<OutboundTransportCommand>,
    outbound_receiver: DispatchReceiver<OutboundTransportCommand>,
}

impl BridgeTransport {
    pub(crate) fn with_queue_config(
        driver: InMemoryTransport,
        queue_config: BridgeQueueConfig,
    ) -> Self {
        let (outbound_sender, outbound_receiver) =
            dispatch_mailbox(queue_config.outbound_capacity_per_engine);
        Self {
            driver,
            outbound_sender,
            outbound_receiver,
        }
    }

    pub(crate) fn sender(&self) -> QueuedTransportSender {
        QueuedTransportSender::new(self.outbound_sender.clone())
    }

    fn drain_raw_ingress(
        &mut self,
    ) -> Result<Vec<TransportIngressEvent>, TransportError> {
        self.driver.drain_transport_ingress()
    }

    // Drains the outbound queue built up during a round to the real driver.
    fn flush_outbound(&mut self) -> Result<usize, TransportError> {
        let commands = self.outbound_receiver.drain();
        for command in &commands {
            self.driver
                .send_transport(&command.endpoint, &command.payload)?;
        }
        Ok(commands.len())
    }

    fn pending_outbound(&self) -> usize {
        self.outbound_receiver.pending_len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeRoundReport {
    pub router_outcome: RouterRoundOutcome,
    pub ingested_transport_observations: Vec<TransportObservation>,
    pub flushed_transport_commands: usize,
    pub dropped_transport_observations: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeWaitState {
    pub next_round_hint: jacquard_core::RoutingTickHint,
    pub pending_transport_observations: usize,
    pub pending_transport_commands: usize,
    pub dropped_transport_observations: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeRoundProgress {
    Advanced(Box<BridgeRoundReport>),
    Waiting(BridgeWaitState),
}

pub struct HostBridge<Router> {
    topology: Observation<Configuration>,
    router: Router,
    transport: BridgeTransport,
    pending_transport_observations: VecDeque<TransportObservation>,
    inbound_capacity: usize,
    next_tick: Tick,
    dropped_transport_observations: usize,
}

pub struct BoundHostBridge<'a, Router> {
    bridge: &'a mut HostBridge<Router>,
}

impl<Router> HostBridge<Router> {
    #[must_use]
    pub fn new(
        topology: Observation<Configuration>,
        router: Router,
        transport: InMemoryTransport,
    ) -> Self {
        Self::with_queue_config(
            topology,
            router,
            transport,
            BridgeQueueConfig::default(),
        )
    }

    #[must_use]
    pub fn with_queue_config(
        topology: Observation<Configuration>,
        router: Router,
        transport: InMemoryTransport,
        queue_config: BridgeQueueConfig,
    ) -> Self {
        let next_tick = Tick(topology.observed_at_tick.0.saturating_add(1));
        Self {
            topology,
            router,
            transport: BridgeTransport::with_queue_config(transport, queue_config),
            pending_transport_observations: VecDeque::new(),
            inbound_capacity: queue_config.inbound_capacity,
            next_tick,
            dropped_transport_observations: 0,
        }
    }

    #[must_use]
    pub(crate) fn from_transport(
        topology: Observation<Configuration>,
        router: Router,
        transport: BridgeTransport,
        queue_config: BridgeQueueConfig,
    ) -> Self {
        let next_tick = Tick(topology.observed_at_tick.0.saturating_add(1));
        Self {
            topology,
            router,
            transport,
            pending_transport_observations: VecDeque::new(),
            inbound_capacity: queue_config.inbound_capacity,
            next_tick,
            dropped_transport_observations: 0,
        }
    }

    #[must_use]
    pub fn topology(&self) -> &Observation<Configuration> {
        &self.topology
    }

    pub fn bind(&mut self) -> BoundHostBridge<'_, Router> {
        BoundHostBridge { bridge: self }
    }
}

impl HostBridge<PathwayRouter> {
    fn sync_router_time(&mut self, tick: Tick) {
        self.router.effects_mut().now = tick;
    }

    fn advance_tick(&mut self) -> Tick {
        let tick = self.next_tick;
        self.next_tick = Tick(self.next_tick.0.saturating_add(1));
        self.sync_router_time(tick);
        tick
    }

    fn stage_transport_ingress(&mut self) -> Result<(), RouteError> {
        // Advance the logical clock once per ingress drain so every event in
        // this batch shares the same Jacquard tick — the host owns time here,
        // not the transport driver.
        let observed_at_tick = self.advance_tick();
        let raw_events = self.transport.drain_raw_ingress()?;
        for event in raw_events {
            // Enforce the inbound capacity bound; excess events are counted but
            // not queued so the caller can observe backpressure.
            if self.pending_transport_observations.len() >= self.inbound_capacity {
                self.dropped_transport_observations =
                    self.dropped_transport_observations.saturating_add(1);
                continue;
            }
            self.pending_transport_observations
                .push_back(event.observe_at(observed_at_tick));
        }
        Ok(())
    }
}

impl BoundHostBridge<'_, PathwayRouter> {
    #[must_use]
    pub fn topology(&self) -> &Observation<Configuration> {
        self.bridge.topology()
    }

    #[must_use]
    pub fn router(&self) -> &PathwayRouter {
        &self.bridge.router
    }

    pub fn router_mut(&mut self) -> &mut PathwayRouter {
        &mut self.bridge.router
    }

    pub fn replace_shared_topology(&mut self, topology: Observation<Configuration>) {
        self.bridge
            .router
            .ingest_topology_observation(topology.clone());
        self.bridge.topology = topology;
    }

    pub fn advance_round(&mut self) -> Result<BridgeRoundProgress, RouteError> {
        // Round sequence: drain+stamp ingress → ingest observations → advance
        // router → flush outbound queue to driver → report progress.
        self.bridge.stage_transport_ingress()?;
        let ingested = self
            .bridge
            .pending_transport_observations
            .drain(..)
            .collect::<Vec<_>>();
        for observation in &ingested {
            self.bridge
                .router
                .ingest_transport_observation(observation)?;
        }

        let router_outcome = self.bridge.router.advance_round()?;
        let flushed_transport_commands = self.bridge.transport.flush_outbound()?;
        let dropped_transport_observations =
            std::mem::take(&mut self.bridge.dropped_transport_observations);

        // Return Waiting when nothing moved this round so callers can back off.
        if ingested.is_empty()
            && flushed_transport_commands == 0
            && dropped_transport_observations == 0
            && router_outcome.engine_change
                == jacquard_core::RoutingTickChange::NoChange
        {
            return Ok(BridgeRoundProgress::Waiting(BridgeWaitState {
                next_round_hint: router_outcome.next_round_hint,
                pending_transport_observations: self
                    .bridge
                    .pending_transport_observations
                    .len(),
                pending_transport_commands: self.bridge.transport.pending_outbound(),
                dropped_transport_observations,
            }));
        }

        Ok(BridgeRoundProgress::Advanced(Box::new(BridgeRoundReport {
            router_outcome,
            ingested_transport_observations: ingested,
            flushed_transport_commands,
            dropped_transport_observations,
        })))
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        ByteCount, EndpointLocator, NodeId, RouteEpoch, RoutingTickChange,
        RoutingTickHint, TransportKind,
    };
    use jacquard_mem_link_profile::InMemoryRuntimeEffects;
    use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};

    use super::*;

    fn endpoint(byte: u8) -> LinkEndpoint {
        LinkEndpoint::new(
            TransportKind::WifiAware,
            EndpointLocator::Opaque(vec![byte]),
            ByteCount(128),
        )
    }

    fn sample_topology(_local_node_id: NodeId) -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(1),
                nodes: std::collections::BTreeMap::new(),
                links: std::collections::BTreeMap::new(),
                environment: jacquard_core::Environment {
                    reachable_neighbor_count: 0,
                    churn_permille: jacquard_core::RatioPermille(0),
                    contention_permille: jacquard_core::RatioPermille(0),
                },
            },
            source_class: jacquard_core::FactSourceClass::Local,
            evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
            origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        }
    }

    fn sample_router(local_node_id: NodeId) -> PathwayRouter {
        MultiEngineRouter::new(
            local_node_id,
            FixedPolicyEngine::new(crate::clients::default_profile()),
            InMemoryRuntimeEffects { now: Tick(1), ..Default::default() },
            sample_topology(local_node_id),
            crate::clients::policy_inputs_for_empty(local_node_id),
        )
    }

    #[test]
    fn owner_binding_is_required_for_round_progression() {
        let local_node_id = NodeId([7; 32]);
        let transport =
            InMemoryTransport::attach(local_node_id, [endpoint(7)], Default::default());
        let mut bridge = HostBridge::new(
            sample_topology(local_node_id),
            sample_router(local_node_id),
            transport,
        );

        let mut owner = bridge.bind();
        let progress = owner.advance_round().expect("advance bridge round");

        assert_eq!(
            progress,
            BridgeRoundProgress::Waiting(BridgeWaitState {
                next_round_hint: RoutingTickHint::HostDefault,
                pending_transport_observations: 0,
                pending_transport_commands: 0,
                dropped_transport_observations: 0,
            })
        );
    }

    #[test]
    fn bounded_ingress_reports_dropped_observations() {
        let local_node_id = NodeId([9; 32]);
        let network = jacquard_mem_link_profile::SharedInMemoryNetwork::default();
        let transport =
            InMemoryTransport::attach(local_node_id, [endpoint(9)], network.clone());
        let mut remote =
            InMemoryTransport::attach(NodeId([8; 32]), [endpoint(8)], network);

        let mut bridge = HostBridge::with_queue_config(
            sample_topology(local_node_id),
            sample_router(local_node_id),
            transport,
            BridgeQueueConfig::new(1, 64),
        );
        remote
            .send_transport(&endpoint(9), b"first")
            .expect("send first ingress frame");
        remote
            .send_transport(&endpoint(9), b"second")
            .expect("send second ingress frame");

        let mut owner = bridge.bind();
        let progress = owner.advance_round().expect("advance bridge round");
        let BridgeRoundProgress::Advanced(report) = progress else {
            panic!("expected advanced bridge round");
        };
        assert_eq!(report.router_outcome.topology_epoch, RouteEpoch(1));
        assert_eq!(
            report.router_outcome.engine_change,
            RoutingTickChange::NoChange
        );
        assert_eq!(report.ingested_transport_observations.len(), 1);
        assert_eq!(report.dropped_transport_observations, 1);
    }

    #[test]
    fn outbound_queue_reports_backpressure_fail_closed() {
        let local_node_id = NodeId([5; 32]);
        let transport = BridgeTransport::with_queue_config(
            InMemoryTransport::attach(local_node_id, [endpoint(5)], Default::default()),
            BridgeQueueConfig::new(1, 1),
        );
        let mut sender = transport.sender();

        sender
            .send_transport(&endpoint(5), b"first")
            .expect("queue first outbound frame");
        let error = sender
            .send_transport(&endpoint(5), b"second")
            .expect_err("outbound queue should fail closed when full");

        assert_eq!(error, TransportError::Unavailable);
        assert_eq!(transport.pending_outbound(), 1);
    }
}
