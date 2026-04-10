import Field.Model.Instance

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldModelRefinement

open FieldModelAPI
open FieldModelInstance

/-- The composed round still publishes a corridor projection whose support is
subordinate to the round-updated posterior support. -/
theorem round_projection_support_conservative
    (evidence : EvidenceInput)
    (state : LocalState) :
    (FieldModelAPI.roundStep evidence state).projection.support ≤
      (FieldModelAPI.roundStep evidence state).posterior.support :=
  (FieldModelAPI.multi_layer_projection_subordinate evidence state).2.2

/-- The composed round keeps the mean-field strength aligned to the updated
posterior support. -/
theorem round_mean_field_tracks_posterior_support
    (evidence : EvidenceInput)
    (state : LocalState) :
    (FieldModelAPI.roundStep evidence state).meanField.fieldStrength =
      (FieldModelAPI.roundStep evidence state).posterior.support :=
  (FieldModelAPI.multi_layer_projection_subordinate evidence state).1

/-- The stronger explicit-path claim still has to pass through the round-updated
posterior knowledge state; the projection cannot outrun that local knowledge. -/
theorem explicit_projection_requires_explicit_round_knowledge
    (evidence : EvidenceInput)
    (state : LocalState)
    (hShape :
      (FieldModelAPI.roundStep evidence state).projection.shape =
        CorridorShape.explicitPath) :
    (FieldModelAPI.roundStep evidence state).posterior.knowledge =
      ReachabilityKnowledge.explicitPath :=
  FieldModelAPI.explicit_projection_requires_explicit_knowledge evidence state hShape

/-- Once the reduced model sees explicit-path evidence twice in a row, the
shared projection stays explicit-path. -/
theorem repeated_explicit_path_rounds_stabilize
    (state : LocalState) :
    (FieldModelAPI.roundStep explicitPathEvidence
        (FieldModelAPI.roundStep explicitPathEvidence state)).projection.shape =
      CorridorShape.explicitPath := by
  simpa [FieldModelAPI.roundStep] using
    repeated_explicit_path_rounds_preserve_projection 1 state

end FieldModelRefinement
