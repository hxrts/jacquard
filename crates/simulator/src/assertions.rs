//! Scenario assertion builders and validation helpers for routing outcomes.

use std::{error::Error, fmt};

use jacquard_core::{DestinationId, NodeId, RoutingEngineId};

use crate::ReducedReplayView;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScenarioAssertions {
    expected_routes: Vec<(NodeId, DestinationId)>,
    expected_engines: Vec<(NodeId, DestinationId, RoutingEngineId)>,
    absent_routes: Vec<(NodeId, DestinationId)>,
    expected_recovery_within_rounds: Option<u32>,
    expected_distinct_engine_count: Option<usize>,
}

impl ScenarioAssertions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn expect_route_materialized(
        mut self,
        owner_node_id: NodeId,
        destination: DestinationId,
    ) -> Self {
        self.expected_routes.push((owner_node_id, destination));
        self
    }

    #[must_use]
    pub fn expect_engine_selected(
        mut self,
        owner_node_id: NodeId,
        destination: DestinationId,
        engine_id: &RoutingEngineId,
    ) -> Self {
        self.expected_engines
            .push((owner_node_id, destination, engine_id.clone()));
        self
    }

    #[must_use]
    pub fn expect_route_absent(
        mut self,
        owner_node_id: NodeId,
        destination: DestinationId,
    ) -> Self {
        self.absent_routes.push((owner_node_id, destination));
        self
    }

    #[must_use]
    pub fn expect_recovery_within_rounds(mut self, rounds: u32) -> Self {
        self.expected_recovery_within_rounds = Some(rounds);
        self
    }

    #[must_use]
    pub fn expect_distinct_engine_count(mut self, count: usize) -> Self {
        self.expected_distinct_engine_count = Some(count);
        self
    }

    pub fn evaluate(&self, replay: &ReducedReplayView) -> Result<(), AssertionFailure> {
        for (owner, destination) in &self.expected_routes {
            if !replay.route_seen(*owner, destination) {
                return Err(AssertionFailure::new(format!(
                    "expected route materialized for owner {:?} destination {:?}, but no active route was observed",
                    owner, destination
                )));
            }
        }
        for (owner, destination, engine_id) in &self.expected_engines {
            if !replay.route_seen_with_engine(*owner, destination, engine_id) {
                return Err(AssertionFailure::new(format!(
                    "expected engine {:?} for owner {:?} destination {:?}, but that engine was never observed on an active route",
                    engine_id, owner, destination
                )));
            }
        }
        for (owner, destination) in &self.absent_routes {
            if replay.route_seen(*owner, destination) {
                return Err(AssertionFailure::new(format!(
                    "expected no route for owner {:?} destination {:?}, but an active route was observed",
                    owner, destination
                )));
            }
        }
        if let Some(rounds) = self.expected_recovery_within_rounds {
            if !replay.recovered_within_rounds(rounds) {
                return Err(AssertionFailure::new(format!(
                    "expected a route to recover within {} rounds after loss, but no such recovery was observed",
                    rounds
                )));
            }
        }
        if let Some(count) = self.expected_distinct_engine_count {
            if replay.distinct_engine_ids.len() != count {
                return Err(AssertionFailure::new(format!(
                    "expected {} distinct engine ids, observed {} ({:?})",
                    count,
                    replay.distinct_engine_ids.len(),
                    replay.distinct_engine_ids
                )));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssertionFailure {
    detail: String,
}

impl AssertionFailure {
    #[must_use]
    pub fn new(detail: String) -> Self {
        Self { detail }
    }
}

impl fmt::Display for AssertionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.detail)
    }
}

impl Error for AssertionFailure {}
