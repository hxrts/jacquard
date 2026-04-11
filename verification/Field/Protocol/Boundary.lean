import Field.Protocol.API
import Field.Protocol.Instance

/-! # Protocol.Boundary — boundary-facing protocol import surface -/

/- 
This module exists so theorem-boundary code can depend on the protocol API plus
the current reduced instance without importing `Field.Protocol.Instance`
directly. It is intentionally thin.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldProtocolBoundary

open FieldProtocolAPI

/-- Re-export the fail-closed export boundary through the protocol-facing
boundary surface. -/
theorem failed_closed_exports_nothing
    (snapshot : MachineSnapshot)
    (hFailed : snapshot.disposition = HostDisposition.failedClosed) :
    exportOutputs snapshot = [] :=
  FieldProtocolAPI.failed_closed_exports_nothing snapshot hFailed

end FieldProtocolBoundary
