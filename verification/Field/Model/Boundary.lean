import Field.Model.Instance
import Field.Protocol.Instance

/-
The Problem. The proof story needs one narrow theorem linking private protocol
outputs to deterministic observer inputs without pretending to prove the whole
field controller. We want a small boundary module that says exactly what the
protocol may contribute: bounded observational evidence, never canonical route
truth.

Solution Structure.
1. Define a compact export-to-evidence adapter.
2. Keep that adapter corridor-only and bounded.
3. Prove that exported protocol outputs cannot manufacture explicit-path or
   unreachable route truth.
-/

/-! # FieldBoundary

Narrow Lean boundary between field protocol outputs and local observer inputs.

This module intentionally proves only the input-contract story. It does not try
to prove controller optimality or router-level correctness.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldBoundary

open FieldModelAPI
open FieldProtocolAPI

/-- Translate one observational protocol output into bounded local evidence. -/
def protocolOutputToEvidence (output : ProtocolOutput) : EvidenceInput :=
  { refresh := .explicitRefresh
    reachability :=
      if output.batch.summaryCount = 0 then
        .unknown
      else
        .corridorOnly
    supportSignal := min (output.batch.summaryCount * 125) 1000
    entropySignal :=
      if output.batch.summaryCount = 0 then
        800
      else
        250
    controllerPressure := 0
    feedback := .none }

/-- Protocol exports can only become observational field evidence. -/
theorem protocol_output_never_claims_canonical_route_truth
    (output : ProtocolOutput) :
    let evidence := protocolOutputToEvidence output
    evidence.reachability ≠ ReachabilitySignal.explicitPath ∧
      evidence.reachability ≠ ReachabilitySignal.unreachable := by
  -- The adapter emits only `unknown` or `corridorOnly`, never canonical claims.
  by_cases hCount : output.batch.summaryCount = 0
  · simp [protocolOutputToEvidence, hCount]
  · simp [protocolOutputToEvidence, hCount]

/-- Any output exported by the reduced protocol instance stays on the
observational side of the field boundary. -/
theorem exported_protocol_outputs_stay_observational
    (snapshot : MachineSnapshot)
    (output : ProtocolOutput)
    (hOutput : output ∈ FieldProtocolAPI.exportOutputs snapshot) :
    (protocolOutputToEvidence output).reachability = ReachabilitySignal.unknown ∨
      (protocolOutputToEvidence output).reachability =
        ReachabilitySignal.corridorOnly := by
  -- The reduced protocol instance exports only observational batches, and the
  -- adapter preserves that by emitting only unknown or corridor-only signals.
  by_cases hCount : output.batch.summaryCount = 0
  · simp [protocolOutputToEvidence, hCount]
  · simp [protocolOutputToEvidence, hCount]

end FieldBoundary
