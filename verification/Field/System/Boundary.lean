import Field.Assumptions

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemBoundary

open FieldAssumptions

/-- The default contract does not justify strong global optimality claims. -/
theorem default_contract_does_not_claim_global_optimality_ready :
    ¬ defaultContract.optional.globalOptimalityReady :=
  FieldAssumptions.default_contract_does_not_claim_global_optimality_ready

/-- Even the stronger reduced-quality contract remains explicitly non-optimality. -/
theorem reduced_quality_contract_still_does_not_claim_global_optimality_ready :
    ¬ reducedQualityContract.optional.globalOptimalityReady :=
  FieldAssumptions.reduced_quality_contract_still_does_not_claim_global_optimality_ready

end FieldSystemBoundary
