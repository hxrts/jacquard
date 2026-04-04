#![forbid(unsafe_code)]

use contour_core::{
    AdaptiveRoutingProfile, Blake3Digest, ContentEncodingError, ContentId, InstalledRoute,
    OrderStamp, RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteError, RouteHealth,
    RouteId, RouteMaintenanceDisposition, RouteMaintenanceTrigger, RouteTransition,
    RoutingFamilyCapabilities, RoutingFamilyId, RoutingObjective, RoutingObservations, Tick,
    TopologySnapshot,
};

pub use contour_core;

pub trait TimeEffects {
    fn now_tick(&self) -> Tick;
}

pub trait OrderEffects {
    fn next_order_stamp(&mut self) -> OrderStamp;
}

pub trait Hashing {
    type Digest: Clone + Eq;

    fn hash_bytes(&self, input: &[u8]) -> Self::Digest;
    fn hash_tagged(&self, domain: &[u8], input: &[u8]) -> Self::Digest;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Blake3Hashing;

impl Hashing for Blake3Hashing {
    type Digest = Blake3Digest;

    fn hash_bytes(&self, input: &[u8]) -> Self::Digest {
        Blake3Digest(*blake3::hash(input).as_bytes())
    }

    fn hash_tagged(&self, domain: &[u8], input: &[u8]) -> Self::Digest {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&(domain.len() as u32).to_le_bytes());
        hasher.update(domain);
        hasher.update(input);
        Blake3Digest(*hasher.finalize().as_bytes())
    }
}

pub trait ContentAddressable {
    type Digest: Clone + Eq;

    fn canonical_bytes(&self) -> Result<Vec<u8>, ContentEncodingError>;

    fn content_id<H: Hashing<Digest = Self::Digest>>(
        &self,
        hasher: &H,
    ) -> Result<ContentId<Self::Digest>, ContentEncodingError> {
        let canonical = self.canonical_bytes()?;
        Ok(ContentId {
            digest: hasher.hash_bytes(&canonical),
        })
    }
}

pub trait TemplateAddressable {
    type Digest: Clone + Eq;

    fn template_bytes(&self) -> Result<Vec<u8>, ContentEncodingError>;

    fn template_id<H: Hashing<Digest = Self::Digest>>(
        &self,
        hasher: &H,
    ) -> Result<ContentId<Self::Digest>, ContentEncodingError> {
        let canonical = self.template_bytes()?;
        Ok(ContentId {
            digest: hasher.hash_bytes(&canonical),
        })
    }
}

pub trait AdaptiveRoutingController {
    fn compute_profile(
        &self,
        objective: &RoutingObjective,
        observations: &RoutingObservations,
    ) -> AdaptiveRoutingProfile;
}

pub trait RouteFamilyExtension {
    fn family_id(&self) -> RoutingFamilyId;

    fn capabilities(&self) -> RoutingFamilyCapabilities;

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &TopologySnapshot,
    ) -> Vec<RouteCandidate>;

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: &RouteCandidate,
    ) -> Result<RouteAdmissionCheck, RouteError>;

    fn admit_route(
        &mut self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
    ) -> Result<RouteAdmission, RouteError>;

    fn install_route(&mut self, admission: RouteAdmission) -> Result<InstalledRoute, RouteError>;

    fn maintain_route(
        &mut self,
        route: &mut InstalledRoute,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceDisposition, RouteError>;

    fn teardown(&mut self, route_id: &RouteId);
}

pub trait TopLevelRouter {
    fn register_family(
        &mut self,
        extension: Box<dyn RouteFamilyExtension>,
    ) -> Result<(), RouteError>;

    fn establish_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<InstalledRoute, RouteError>;

    fn reselect_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<InstalledRoute, RouteError>;
}

pub trait RoutingControlPlane {
    fn establish_route(
        &mut self,
        objective: RoutingObjective,
    ) -> Result<InstalledRoute, RouteError>;

    fn maintain_route(
        &mut self,
        route_id: &RouteId,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteTransition, RouteError>;

    fn anti_entropy_tick(&mut self) -> Result<(), RouteError>;
}

pub trait RoutingDataPlane {
    fn forward_payload(&mut self, route_id: &RouteId, payload: &[u8]) -> Result<(), RouteError>;

    fn observe_route_health(&mut self, route_id: &RouteId) -> Result<RouteHealth, RouteError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StaticContent(&'static [u8]);

    impl ContentAddressable for StaticContent {
        type Digest = Blake3Digest;

        fn canonical_bytes(&self) -> Result<Vec<u8>, ContentEncodingError> {
            Ok(self.0.to_vec())
        }
    }

    #[test]
    fn blake3_hashing_is_deterministic() {
        let hashing = Blake3Hashing;
        let digest_a = hashing.hash_tagged(b"route", b"payload");
        let digest_b = hashing.hash_tagged(b"route", b"payload");
        let digest_c = hashing.hash_tagged(b"other", b"payload");

        assert_eq!(digest_a, digest_b);
        assert_ne!(digest_a, digest_c);
    }

    #[test]
    fn content_addressing_uses_canonical_bytes() {
        let hashing = Blake3Hashing;
        let item = StaticContent(b"contour");
        let content_id = item.content_id(&hashing).expect("content id");

        assert_eq!(content_id.digest, hashing.hash_bytes(b"contour"));
    }
}
