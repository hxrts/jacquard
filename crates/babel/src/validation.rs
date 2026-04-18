//! Engine-owned Babel model helpers for model-lane validation.

use std::collections::BTreeMap;

use jacquard_core::{
    BackendRouteId, Configuration, MaterializedRoute, NodeId, Observation, RouteDegradation,
    RouteMaintenanceResult, RouteRuntimeState, RoutingTickChange, Tick, TransportKind,
};

use crate::{
    private_state::{reduce_round_state, BabelRoundInput, BabelRoundState},
    public_state::{
        ActiveBabelRoute, BabelBestNextHop, BabelPlannerSnapshot, FeasibilityEntry, RouteEntry,
    },
    runtime::{reduce_maintenance, restored_active_route, BabelMaintenanceInput},
    BabelEngine, DecayWindow,
};
use jacquard_traits::{
    RoutingEngineMaintenanceModel, RoutingEngineRestoreModel, RoutingEngineRoundModel,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelPlannerChoiceView {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub metric: u16,
    pub degradation: RouteDegradation,
    pub transport_kind: TransportKind,
    pub updated_at_tick: Tick,
    pub topology_epoch: jacquard_core::RouteEpoch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelPlannerSnapshotView {
    pub local_node_id: NodeId,
    pub stale_after_ticks: u64,
    pub choices: Vec<BabelPlannerChoiceView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelRoundRouteEntryView {
    pub destination: NodeId,
    pub via_neighbor: NodeId,
    pub router_id: NodeId,
    pub seqno: u16,
    pub metric: u16,
    pub observed_at_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelFeasibilityEntryView {
    pub destination: NodeId,
    pub seqno: u16,
    pub metric: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelRoundStateView {
    pub route_entries: Vec<BabelRoundRouteEntryView>,
    pub feasibility_entries: Vec<BabelFeasibilityEntryView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelRoundInputView {
    pub topology: Observation<Configuration>,
    pub now: Tick,
    pub local_node_id: NodeId,
    pub decay_window: DecayWindow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelRoundOutputView {
    pub change: RoutingTickChange,
    pub planner_snapshot: BabelPlannerSnapshotView,
    pub selected_destination_count: usize,
    pub best_next_hop_count: usize,
    pub feasibility_destination_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelRestoredRouteView {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub backend_route_id: BackendRouteId,
    pub installed_at_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelMaintenanceBestNextHopView {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub metric: u16,
    pub tq: jacquard_core::RatioPermille,
    pub degradation: RouteDegradation,
    pub transport_kind: TransportKind,
    pub updated_at_tick: Tick,
    pub topology_epoch: jacquard_core::RouteEpoch,
    pub backend_route_id: BackendRouteId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelMaintenanceStateView {
    pub runtime: RouteRuntimeState,
    pub active_route: BabelRestoredRouteView,
    pub best_next_hop: Option<BabelMaintenanceBestNextHopView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelMaintenanceInputView {
    pub now_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelMaintenanceOutputView {
    pub next_runtime: RouteRuntimeState,
    pub result: RouteMaintenanceResult,
}

#[must_use]
pub(crate) fn reduce_round_view(
    state: &BabelRoundStateView,
    input: &BabelRoundInputView,
) -> BabelRoundOutputView {
    let transition = reduce_round_state(
        private_round_state(state),
        &BabelRoundInput {
            topology: input.topology.clone(),
            now: input.now,
            local_node_id: input.local_node_id,
            decay_window: input.decay_window,
        },
    );
    BabelRoundOutputView {
        change: transition.change,
        planner_snapshot: view_snapshot(&transition.planner_snapshot),
        selected_destination_count: transition.next_state.selected_routes.len(),
        best_next_hop_count: transition.next_state.best_next_hops.len(),
        feasibility_destination_count: transition.next_state.feasibility_distances.len(),
    }
}

#[must_use]
pub(crate) fn restore_route_view(route: &MaterializedRoute) -> Option<BabelRestoredRouteView> {
    let restored = restored_active_route(route)?;
    Some(BabelRestoredRouteView {
        destination: restored.destination,
        next_hop: restored.next_hop,
        backend_route_id: restored.backend_route_id,
        installed_at_tick: restored.installed_at_tick,
    })
}

impl<Transport, Effects> RoutingEngineRoundModel for BabelEngine<Transport, Effects> {
    type RoundState = BabelRoundStateView;
    type RoundInput = BabelRoundInputView;
    type RoundOutput = BabelRoundOutputView;

    fn reduce_round_state(state: &Self::RoundState, input: &Self::RoundInput) -> Self::RoundOutput {
        reduce_round_view(state, input)
    }
}

impl<Transport, Effects> RoutingEngineMaintenanceModel for BabelEngine<Transport, Effects> {
    type MaintenanceState = BabelMaintenanceStateView;
    type MaintenanceInput = BabelMaintenanceInputView;
    type MaintenanceOutput = BabelMaintenanceOutputView;

    fn reduce_maintenance_state(
        state: &Self::MaintenanceState,
        input: &Self::MaintenanceInput,
    ) -> Self::MaintenanceOutput {
        let transition = reduce_maintenance(BabelMaintenanceInput {
            runtime: state.runtime.clone(),
            active_route: ActiveBabelRoute {
                destination: state.active_route.destination,
                next_hop: state.active_route.next_hop,
                backend_route_id: state.active_route.backend_route_id.clone(),
                installed_at_tick: state.active_route.installed_at_tick,
            },
            best_next_hop: state.best_next_hop.as_ref().map(|best| BabelBestNextHop {
                destination: best.destination,
                next_hop: best.next_hop,
                metric: best.metric,
                tq: best.tq,
                degradation: best.degradation,
                transport_kind: best.transport_kind.clone(),
                updated_at_tick: best.updated_at_tick,
                topology_epoch: best.topology_epoch,
                backend_route_id: best.backend_route_id.clone(),
            }),
            now_tick: input.now_tick,
        });
        BabelMaintenanceOutputView {
            next_runtime: transition.next_runtime,
            result: transition.result,
        }
    }
}

impl<Transport, Effects> RoutingEngineRestoreModel for BabelEngine<Transport, Effects> {
    type RestoredRoute = BabelRestoredRouteView;

    fn restore_route_runtime(route: &MaterializedRoute) -> Option<Self::RestoredRoute> {
        restore_route_view(route)
    }
}

fn private_round_state(state: &BabelRoundStateView) -> BabelRoundState {
    let mut route_table: BTreeMap<NodeId, BTreeMap<NodeId, RouteEntry>> = BTreeMap::new();
    for entry in &state.route_entries {
        route_table.entry(entry.destination).or_default().insert(
            entry.via_neighbor,
            RouteEntry {
                router_id: entry.router_id,
                seqno: entry.seqno,
                metric: entry.metric,
                observed_at_tick: entry.observed_at_tick,
            },
        );
    }
    let feasibility_distances = state
        .feasibility_entries
        .iter()
        .map(|entry| {
            (
                entry.destination,
                FeasibilityEntry {
                    seqno: entry.seqno,
                    metric: entry.metric,
                },
            )
        })
        .collect();
    BabelRoundState {
        route_table,
        selected_routes: BTreeMap::new(),
        best_next_hops: BTreeMap::new(),
        feasibility_distances,
    }
}

fn view_snapshot(snapshot: &BabelPlannerSnapshot) -> BabelPlannerSnapshotView {
    BabelPlannerSnapshotView {
        local_node_id: snapshot.local_node_id,
        stale_after_ticks: snapshot.stale_after_ticks,
        choices: snapshot
            .best_next_hops
            .values()
            .map(|best| BabelPlannerChoiceView {
                destination: best.destination,
                next_hop: best.next_hop,
                metric: best.metric,
                degradation: best.degradation,
                transport_kind: best.transport_kind.clone(),
                updated_at_tick: best.updated_at_tick,
                topology_epoch: best.topology_epoch,
            })
            .collect(),
    }
}
