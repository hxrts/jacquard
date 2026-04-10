//! Internal trait-bound bundles for the pathway engine module.
//!
//! Each alias groups the set of traits that one `PathwayEngine` generic
//! parameter must satisfy. Using the aliases keeps `impl` headers
//! readable without changing the public engine-neutral trait surface in
//! `jacquard-traits` or the pathway-owned extension seams in this crate.
//! Every alias has a blanket impl, so referring to an alias is
//! identical to inlining its full trait list.

use jacquard_core::Configuration;
use jacquard_traits::{
    CommitteeSelector, HashDigestBytes, Hashing, OrderEffects, RetentionStore,
    RouteEventLogEffects, StorageEffects, TimeEffects, TransportSenderEffects,
};

use crate::{PathwayNeighborhoodEstimateAccess, PathwayPeerEstimateAccess, PathwayTopologyModel};

pub(crate) trait PathwayTopologyBounds: PathwayTopologyModel
where
    Self::PeerEstimate: PathwayPeerEstimateAccess,
    Self::NeighborhoodEstimate: PathwayNeighborhoodEstimateAccess,
{
}

impl<T> PathwayTopologyBounds for T
where
    T: PathwayTopologyModel,
    T::PeerEstimate: PathwayPeerEstimateAccess,
    T::NeighborhoodEstimate: PathwayNeighborhoodEstimateAccess,
{
}

pub(crate) trait PathwayTransportBounds:
    TransportSenderEffects + Send + Sync + 'static
{
}

impl<T> PathwayTransportBounds for T where T: TransportSenderEffects + Send + Sync + 'static {}

pub(crate) trait PathwayRetentionBounds: RetentionStore {}

impl<T> PathwayRetentionBounds for T where T: RetentionStore {}

pub(crate) trait PathwayEffectsBounds:
    TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects
{
}

impl<T> PathwayEffectsBounds for T where
    T: TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects
{
}

pub(crate) trait PathwayHasherBounds: Hashing
where
    Self::Digest: HashDigestBytes,
{
}

impl<T> PathwayHasherBounds for T
where
    T: Hashing,
    T::Digest: HashDigestBytes,
{
}

pub(crate) trait PathwaySelectorBounds:
    CommitteeSelector<TopologyView = Configuration>
{
}

impl<T> PathwaySelectorBounds for T where T: CommitteeSelector<TopologyView = Configuration> {}
