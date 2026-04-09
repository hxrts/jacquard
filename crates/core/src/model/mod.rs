//! World and configuration primitives for the routing decision pipeline.
//!
//! This module groups the shared data types that describe the routing world,
//! the inputs and outputs of the policy and estimation stages, and the
//! observation layer that connects raw world facts to engine-consumable values.
//!
//! Submodules:
//! - `action` — policy outputs, selected routing parameters, and operating
//!   modes.
//! - `estimation` — engine-neutral route estimates between observation and
//!   admission.
//! - `observations` — relay budgets, information summaries, and observation
//!   aliases.
//! - `policy` — routing objectives, protection and connectivity classes, policy
//!   inputs.
//! - `world` — the instantiated world model: `Node`, `Link`, `Environment`,
//!   `Configuration`, and their profile/state splits.

mod action;
mod estimation;
mod observations;
mod policy;
mod world;

pub use action::*;
pub use estimation::*;
pub use observations::*;
pub use policy::*;
pub use world::*;
