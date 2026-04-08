//! Internal trait-bound bundles for the mesh engine module.
//!
//! Each alias groups the set of traits that one `MeshEngine` generic
//! parameter must satisfy. Using the aliases keeps `impl` headers
//! readable without changing the public trait surface in
//! `jacquard-traits`. Every alias has a blanket impl, so referring to
//! an alias is identical to inlining its full trait list.

use jacquard_core::Configuration;
use jacquard_traits::{
    CommitteeSelector, HashDigestBytes, Hashing, MeshNeighborhoodEstimateAccess,
    MeshPeerEstimateAccess, OrderEffects, RetentionStore, RouteEventLogEffects,
    StorageEffects, TimeEffects, TransportEffects,
};

pub(crate) trait MeshTopologyBounds: jacquard_traits::MeshTopologyModel
where
    Self::PeerEstimate: MeshPeerEstimateAccess,
    Self::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
{
}

impl<T> MeshTopologyBounds for T
where
    T: jacquard_traits::MeshTopologyModel,
    T::PeerEstimate: MeshPeerEstimateAccess,
    T::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
{
}

pub(crate) trait TransportEffectsBounds: TransportEffects + Send + Sync + 'static {}

impl<T> TransportEffectsBounds for T where T: TransportEffects + Send + Sync + 'static {}

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

pub(crate) trait MeshHasherBounds: Hashing
where
    Self::Digest: HashDigestBytes,
{
}

impl<T> MeshHasherBounds for T
where
    T: Hashing,
    T::Digest: HashDigestBytes,
{
}

pub(crate) trait MeshSelectorBounds:
    CommitteeSelector<TopologyView = Configuration>
{
}

impl<T> MeshSelectorBounds for T where T: CommitteeSelector<TopologyView = Configuration>
{}
