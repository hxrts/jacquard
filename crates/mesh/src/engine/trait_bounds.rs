//! Internal trait-bound bundles for the mesh engine module.
//!
//! Each alias groups the set of traits that one `MeshEngine` generic
//! parameter must satisfy. Using the aliases keeps `impl` headers
//! readable without changing the public trait surface in
//! `jacquard-traits`. Every alias has a blanket impl, so referring to
//! an alias is identical to inlining its full trait list.

use jacquard_core::{Blake3Digest, Configuration};
use jacquard_traits::{
    CommitteeSelector, Hashing, MeshTransport, OrderEffects, RetentionStore, RouteEventLogEffects,
    StorageEffects, TimeEffects, TransportEffects,
};

pub(crate) trait MeshTopologyBounds:
    jacquard_traits::MeshTopologyModel<
    PeerEstimate = crate::topology::MeshPeerEstimate,
    NeighborhoodEstimate = crate::topology::MeshNeighborhoodEstimate,
>
{
}

impl<T> MeshTopologyBounds for T where
    T: jacquard_traits::MeshTopologyModel<
        PeerEstimate = crate::topology::MeshPeerEstimate,
        NeighborhoodEstimate = crate::topology::MeshNeighborhoodEstimate,
    >
{
}

pub(crate) trait MeshTransportBounds:
    MeshTransport + TransportEffects + Send + Sync + 'static
{
}

impl<T> MeshTransportBounds for T where T: MeshTransport + TransportEffects + Send + Sync + 'static {}

pub(crate) trait MeshRetentionBounds: RetentionStore {}

impl<T> MeshRetentionBounds for T where T: RetentionStore {}

pub(crate) trait MeshEffectsBounds:
    TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects
{
}

impl<T> MeshEffectsBounds for T where
    T: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects
{
}

pub(crate) trait MeshHasherBounds: Hashing<Digest = Blake3Digest> {}

impl<T> MeshHasherBounds for T where T: Hashing<Digest = Blake3Digest> {}

pub(crate) trait MeshSelectorBounds:
    CommitteeSelector<TopologyView = Configuration>
{
}

impl<T> MeshSelectorBounds for T where T: CommitteeSelector<TopologyView = Configuration> {}
