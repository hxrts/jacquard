//! Discoverable defaults for the reference client composition surface.

use crate::BridgeQueueConfig;

/// Default queue capacities used by the reference client bridge.
pub const DEFAULT_BRIDGE_QUEUE_CONFIG: BridgeQueueConfig =
    BridgeQueueConfig::new(64, 64);
