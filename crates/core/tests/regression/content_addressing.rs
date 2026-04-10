use jacquard_core::{Blake3Digest, ContentEncodingError, ContentId, RouteCommitmentId, RouteId};
use jacquard_traits::{
    Blake3Hashing, ContentAddressable, HashDigestBytes, Hashing, TemplateAddressable,
};

#[derive(Clone, Debug)]
struct CanonicalRouteArtifact {
    route_bytes: Vec<u8>,
    hop_bytes: Vec<u8>,
}

impl ContentAddressable for CanonicalRouteArtifact {
    type Digest = Blake3Digest;

    fn canonical_bytes(&self) -> Result<Vec<u8>, ContentEncodingError> {
        let mut bytes = self.route_bytes.clone();
        bytes.extend_from_slice(&self.hop_bytes);
        Ok(bytes)
    }
}

impl TemplateAddressable for CanonicalRouteArtifact {
    type Digest = Blake3Digest;

    fn template_bytes(&self) -> Result<Vec<u8>, ContentEncodingError> {
        Ok(vec![
            u8::try_from(self.route_bytes.len()).expect("route length fits"),
            u8::try_from(self.hop_bytes.len()).expect("hop length fits"),
        ])
    }
}

#[test]
fn truncated_route_identities_are_stable_prefixes_of_the_same_digest() {
    let digest = Blake3Hashing.hash_tagged(b"route", b"stable-content");
    let route_id = RouteId::from(&digest);
    let commitment_id = RouteCommitmentId::from(&digest);

    assert_eq!(route_id.0, commitment_id.0);
    assert_eq!(&route_id.0[..], &digest.as_bytes()[..16]);
}

#[test]
fn content_id_is_stable_for_equivalent_canonical_artifacts() {
    let artifact_a = CanonicalRouteArtifact {
        route_bytes: vec![1, 2, 3],
        hop_bytes: vec![9, 8, 7],
    };
    let artifact_b = CanonicalRouteArtifact {
        route_bytes: vec![1, 2, 3],
        hop_bytes: vec![9, 8, 7],
    };

    let id_a = artifact_a.content_id(&Blake3Hashing).expect("content id");
    let id_b = artifact_b.content_id(&Blake3Hashing).expect("content id");

    assert_eq!(id_a, id_b);
    assert_eq!(id_a.digest, Blake3Hashing.hash_bytes(&[1, 2, 3, 9, 8, 7]));
}

#[test]
fn template_identity_stays_stable_when_instance_bytes_change_without_shape_change() {
    let template_a = CanonicalRouteArtifact {
        route_bytes: vec![1, 2, 3],
        hop_bytes: vec![9, 8, 7],
    };
    let template_b = CanonicalRouteArtifact {
        route_bytes: vec![4, 5, 6],
        hop_bytes: vec![0, 1, 2],
    };

    let id_a: ContentId<Blake3Digest> =
        template_a.template_id(&Blake3Hashing).expect("template id");
    let id_b: ContentId<Blake3Digest> =
        template_b.template_id(&Blake3Hashing).expect("template id");

    assert_eq!(id_a, id_b);
}
