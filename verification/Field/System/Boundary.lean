import Field.Assumptions

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemBoundary

open FieldAssumptions

/-- The current field proof contract still lacks the comparison structure
needed for global routing-quality or optimality theorems. -/
theorem default_contract_does_not_claim_quality_comparison_ready :
    ¬ defaultContract.optional.qualityComparisonReady := by
  simp [defaultContract, defaultOptionalStrengtheningAssumptions]

end FieldSystemBoundary
