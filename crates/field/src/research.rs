//! Coded-diffusion research-path boundary.
//!
//! This module is the feature-neutral namespace for the experimental coded
//! reconstruction path. It owns only fragment movement, rank, custody,
//! duplicate/innovative arrivals, diffusion pressure, and reconstruction
//! quorum vocabulary. It must remain independent of the legacy planner stack.

use jacquard_core::NodeId;
use serde::{Deserialize, Serialize};

/// Stable message identifier for one coded reconstruction objective.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct DiffusionMessageId(pub [u8; 16]);

/// Stable fragment identifier within one coded reconstruction objective.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct DiffusionFragmentId(pub [u8; 16]);

/// Bounded coding-width description for one message.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CodingWindow {
    /// Independent rank required for reconstruction.
    pub required_rank: u16,
    /// Number of encoded fragments available to diffuse.
    pub encoded_fragments: u16,
}

impl CodingWindow {
    /// Construct a valid coding window.
    pub fn try_new(required_rank: u16, encoded_fragments: u16) -> Option<Self> {
        if required_rank == 0 || encoded_fragments < required_rank {
            return None;
        }

        Some(Self {
            required_rank,
            encoded_fragments,
        })
    }
}

/// Classification of one received fragment relative to receiver state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FragmentArrivalClass {
    /// The fragment increased independent receiver rank.
    Innovative,
    /// The fragment repeated information already represented at the receiver.
    Duplicate,
}

/// Observer-visible custody for one fragment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FragmentCustody {
    /// Message that owns the fragment.
    pub message_id: DiffusionMessageId,
    /// Fragment being retained or forwarded.
    pub fragment_id: DiffusionFragmentId,
    /// Node currently observed with custody.
    pub custodian: NodeId,
    /// Whether the current custodian is expected to retain the fragment.
    pub retained: bool,
}

/// Receiver-local reconstruction progress.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReceiverRankState {
    /// Message being reconstructed.
    pub message_id: DiffusionMessageId,
    /// Receiver whose rank is measured.
    pub receiver: NodeId,
    /// Current independent rank.
    pub independent_rank: u16,
    /// Count of arrivals that increased rank.
    pub innovative_arrivals: u16,
    /// Count of arrivals that did not increase rank.
    pub duplicate_arrivals: u16,
}

impl ReceiverRankState {
    /// Classify an arrival and return the updated receiver state.
    #[must_use]
    pub fn with_arrival(mut self, arrival: FragmentArrivalClass) -> Self {
        match arrival {
            FragmentArrivalClass::Innovative => {
                self.independent_rank = self.independent_rank.saturating_add(1);
                self.innovative_arrivals = self.innovative_arrivals.saturating_add(1);
            }
            FragmentArrivalClass::Duplicate => {
                self.duplicate_arrivals = self.duplicate_arrivals.saturating_add(1);
            }
        }
        self
    }
}

/// Aggregate reconstruction status over the observed receiver population.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReconstructionQuorum {
    /// Message being reconstructed.
    pub message_id: DiffusionMessageId,
    /// Rank required for reconstruction.
    pub required_rank: u16,
    /// Number of receivers represented by this aggregate.
    pub observed_receivers: u16,
    /// Number of represented receivers at or above the required rank.
    pub complete_receivers: u16,
    /// Minimum observed independent rank across represented receivers.
    pub min_independent_rank: u16,
}

impl ReconstructionQuorum {
    /// Whether every represented receiver has reached reconstruction rank.
    #[must_use]
    pub fn is_complete(self) -> bool {
        self.observed_receivers > 0
            && self.complete_receivers == self.observed_receivers
            && self.min_independent_rank >= self.required_rank
    }
}

/// Deterministic pressure signal for local coded diffusion control.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionPressure {
    /// Need to keep fragments in bounded custody, in permille.
    pub custody_pressure_permille: u16,
    /// Need to move innovative fragments, in permille.
    pub innovation_pressure_permille: u16,
    /// Need to suppress duplicate movement, in permille.
    pub duplicate_pressure_permille: u16,
}

impl DiffusionPressure {
    /// Clamp pressure components to the normalized deterministic range.
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            custody_pressure_permille: self.custody_pressure_permille.min(1000),
            innovation_pressure_permille: self.innovation_pressure_permille.min(1000),
            duplicate_pressure_permille: self.duplicate_pressure_permille.min(1000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id16(fill: u8) -> [u8; 16] {
        [fill; 16]
    }

    fn node_id(fill: u8) -> NodeId {
        NodeId([fill; 32])
    }

    #[test]
    fn coding_window_requires_reconstructable_width() {
        assert_eq!(CodingWindow::try_new(0, 4), None);
        assert_eq!(CodingWindow::try_new(5, 4), None);
        assert_eq!(
            CodingWindow::try_new(3, 5),
            Some(CodingWindow {
                required_rank: 3,
                encoded_fragments: 5,
            })
        );
    }

    #[test]
    fn receiver_rank_counts_innovative_and_duplicate_arrivals() {
        let receiver = node_id(7);
        let state = ReceiverRankState {
            message_id: DiffusionMessageId(id16(1)),
            receiver,
            independent_rank: 0,
            innovative_arrivals: 0,
            duplicate_arrivals: 0,
        }
        .with_arrival(FragmentArrivalClass::Innovative)
        .with_arrival(FragmentArrivalClass::Duplicate);

        assert_eq!(state.independent_rank, 1);
        assert_eq!(state.innovative_arrivals, 1);
        assert_eq!(state.duplicate_arrivals, 1);
    }

    #[test]
    fn quorum_requires_all_observed_receivers_to_complete() {
        let incomplete = ReconstructionQuorum {
            message_id: DiffusionMessageId(id16(1)),
            required_rank: 3,
            observed_receivers: 2,
            complete_receivers: 1,
            min_independent_rank: 2,
        };
        let complete = ReconstructionQuorum {
            complete_receivers: 2,
            min_independent_rank: 3,
            ..incomplete
        };

        assert!(!incomplete.is_complete());
        assert!(complete.is_complete());
    }

    #[test]
    fn diffusion_pressure_clamps_to_permille_range() {
        assert_eq!(
            DiffusionPressure {
                custody_pressure_permille: 1001,
                innovation_pressure_permille: 1200,
                duplicate_pressure_permille: 999,
            }
            .clamped(),
            DiffusionPressure {
                custody_pressure_permille: 1000,
                innovation_pressure_permille: 1000,
                duplicate_pressure_permille: 999,
            }
        );
    }
}
