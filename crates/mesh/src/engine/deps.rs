//! Internal bound bundles for the mesh engine module.
//!
//! These aliases keep impl headers readable without changing the public
//! trait surface in `jacquard-traits`.

use jacquard_core::{Blake3Digest, Configuration};
use jacquard_traits::{
    CommitteeSelector, Hashing, MeshTransport, OrderEffects, RetentionStore, RouteEventLogEffects,
    StorageEffects, TimeEffects, TransportEffects,
};

pub(crate) trait MeshTopologyDeps:
    jacquard_traits::MeshTopologyModel<
    PeerEstimate = crate::topology::MeshPeerEstimate,
    NeighborhoodEstimate = crate::topology::MeshNeighborhoodEstimate,
>
{
}

impl<T> MeshTopologyDeps for T where
    T: jacquard_traits::MeshTopologyModel<
        PeerEstimate = crate::topology::MeshPeerEstimate,
        NeighborhoodEstimate = crate::topology::MeshNeighborhoodEstimate,
    >
{
}

pub(crate) trait MeshTransportDeps:
    MeshTransport + TransportEffects + Send + Sync + 'static
{
}

impl<T> MeshTransportDeps for T where T: MeshTransport + TransportEffects + Send + Sync + 'static {}

pub(crate) trait MeshRetentionDeps: RetentionStore {}

impl<T> MeshRetentionDeps for T where T: RetentionStore {}

pub(crate) trait MeshEffectsDeps:
    TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects
{
}

impl<T> MeshEffectsDeps for T where
    T: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects
{
}

pub(crate) trait MeshHasherDeps: Hashing<Digest = Blake3Digest> {}

impl<T> MeshHasherDeps for T where T: Hashing<Digest = Blake3Digest> {}

pub(crate) trait MeshSelectorDeps: CommitteeSelector<TopologyView = Configuration> {}

impl<T> MeshSelectorDeps for T where T: CommitteeSelector<TopologyView = Configuration> {}
