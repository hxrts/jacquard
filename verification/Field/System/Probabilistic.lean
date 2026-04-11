import Field.Model.Instance
import Field.Model.Refinement
import Field.Router.Probabilistic
import Field.System.EndToEnd

/-
The Problem. The field system layer already composes async transport,
publication, and local posterior semantics, but it still needs one explicit
probabilistic system layer that explains how envelopes, routes, and published
candidates induce probabilistic observations and posterior decisions.

Solution Structure.
1. Translate async envelopes and lifecycle routes into reduced evidence.
2. Define the induced probabilistic observations and posterior beliefs.
3. Prove representative observation-strength and repeated-observation lemmas
   over that system-facing probabilistic surface.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemProbabilistic

open FieldAsyncAPI
open FieldInformationBayesian
open FieldModelAPI
open FieldModelInstance
open FieldModelRefinement
open FieldNetworkAPI
open FieldRouterProbabilistic
open FieldRouterPublication
open FieldSystemEndToEnd

/-! ## Envelope Evidence And Posterior Surface -/

def reachabilitySignalOfShape : CorridorShape → ReachabilitySignal
  | .opaque => .unknown
  | .corridorEnvelope => .corridorOnly
  | .explicitPath => .explicitPath

def feedbackOfShape : CorridorShape → EvidenceFeedback
  | .opaque => .none
  | .corridorEnvelope => .weakReverse
  | .explicitPath => .strongReverse

def evidenceOfProjection
    (projection : CorridorEnvelopeProjection) : EvidenceInput :=
  { refresh := .explicitRefresh
    reachability := reachabilitySignalOfShape projection.shape
    supportSignal := projection.support
    entropySignal := 0
    controllerPressure := 0
    feedback := feedbackOfShape projection.shape }

def evidenceOfAsyncEnvelope
    (envelope : AsyncEnvelope) : EvidenceInput :=
  if envelope.dropped then
    { refresh := .unchanged
      reachability := .preserve
      supportSignal := envelope.projection.support
      entropySignal := 0
      controllerPressure := 0
      feedback := .none }
  else
    { refresh := .explicitRefresh
      reachability := reachabilitySignalOfShape envelope.projection.shape
      supportSignal := envelope.projection.support
      entropySignal := 0
      controllerPressure := if envelope.delay = 0 then 0 else 600
      feedback := feedbackOfShape envelope.projection.shape }

def evidenceOfLifecycleRoute
    (route : FieldRouterLifecycle.LifecycleRoute) : EvidenceInput :=
  match route.status with
  | .withdrawn =>
      { refresh := .unchanged
        reachability := .preserve
        supportSignal := route.candidate.support
        entropySignal := 0
        controllerPressure := 0
        feedback := .none }
  | .expired =>
      { refresh := .unchanged
        reachability := .preserve
        supportSignal := route.candidate.support
        entropySignal := 0
        controllerPressure := 0
        feedback := .none }
  | _ =>
      { refresh := .explicitRefresh
        reachability := reachabilitySignalOfShape route.candidate.shape
        supportSignal := route.candidate.support
        entropySignal := 0
        controllerPressure := 0
        feedback := feedbackOfShape route.candidate.shape }

def probabilisticObservationOfAsyncEnvelope
    (state : AsyncState)
    (envelope : AsyncEnvelope) : ProbabilisticRouteObservation :=
  let senderState := state.network.localStates envelope.sender envelope.destination
  observationOfEvidence (evidenceOfAsyncEnvelope envelope) senderState

noncomputable def probabilisticPosteriorOfAsyncEnvelope
    (state : AsyncState)
    (envelope : AsyncEnvelope) : ProbabilisticRouteBelief :=
  let senderState := state.network.localStates envelope.sender envelope.destination
  FieldModelAPI.bayesianPosterior (evidenceOfAsyncEnvelope envelope) senderState

noncomputable def probabilisticPosteriorOfPublishedCandidate
    (state : AsyncState)
    (candidate : PublishedCandidate) : ProbabilisticRouteBelief :=
  let senderState := state.network.localStates candidate.publisher candidate.destination
  FieldModelAPI.bayesianPosterior
    (evidenceOfProjection
      { shape := candidate.shape
        support := candidate.support
        hopLower := candidate.hopLower
        hopUpper := candidate.hopUpper })
    senderState

def repeatedObservationPair
    (left right : AsyncEnvelope) : Prop :=
  left = right

def correlatedObservationPair
    (left right : AsyncEnvelope) : Prop :=
  left.sender = right.sender ∧
    left.destination = right.destination ∧
    left ≠ right

inductive ReducedProbabilisticAsyncEnvelope
  | delayed (envelope : AsyncEnvelope)
  | lossy (envelope : AsyncEnvelope)
  | repeated (left right : AsyncEnvelope)
  | correlated (left right : AsyncEnvelope)

noncomputable def posteriorConfidenceDecisionOfAsyncEnvelope
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (state : AsyncState)
    (envelope : AsyncEnvelope) : PosteriorRoutingDecision :=
  posteriorConfidenceDecision thresholds hAdm
    (probabilisticPosteriorOfAsyncEnvelope state envelope)

noncomputable def posteriorMinRegretDecisionOfAsyncEnvelope
    (state : AsyncState)
    (envelope : AsyncEnvelope) : PosteriorRoutingDecision :=
  posteriorMinRegretDecision (probabilisticPosteriorOfAsyncEnvelope state envelope)

def probabilisticObservationStrength
    (observation : ProbabilisticRouteObservation) : Nat :=
  match observation.delivery with
  | .missing => 0
  | .delayed => 1
  | .delivered => 2

theorem dropped_envelope_maps_to_missing_observation
    (state : AsyncState)
    (envelope : AsyncEnvelope)
    (hDropped : envelope.dropped = true) :
    (probabilisticObservationOfAsyncEnvelope state envelope).delivery =
      ObservationDeliveryClass.missing := by
  unfold probabilisticObservationOfAsyncEnvelope
  simp [evidenceOfAsyncEnvelope, hDropped, observationOfEvidence]

theorem delayed_envelope_maps_to_delayed_observation
    (state : AsyncState)
    (envelope : AsyncEnvelope)
    (hDropped : envelope.dropped = false)
    (hDelayed : envelope.delay ≠ 0) :
    (probabilisticObservationOfAsyncEnvelope state envelope).delivery =
      ObservationDeliveryClass.delayed := by
  unfold probabilisticObservationOfAsyncEnvelope
  simp [evidenceOfAsyncEnvelope, hDropped, hDelayed, observationOfEvidence]

theorem ready_envelope_maps_to_delivered_observation
    (state : AsyncState)
    (envelope : AsyncEnvelope)
    (hReady : readyForDelivery envelope = true) :
    (probabilisticObservationOfAsyncEnvelope state envelope).delivery =
      ObservationDeliveryClass.delivered := by
  have hDelay : envelope.delay = 0 := by
    cases envelope with
    | mk sender receiver destination projection delay retryCount dropped =>
        cases delay <;> cases dropped <;> simp [readyForDelivery] at hReady
        rfl
  have hDropped : envelope.dropped = false := by
    cases envelope with
    | mk sender receiver destination projection delay retryCount dropped =>
        cases delay <;> cases dropped <;> simp [readyForDelivery] at hReady
        rfl
  unfold probabilisticObservationOfAsyncEnvelope
  simp [evidenceOfAsyncEnvelope, hDropped, hDelay, observationOfEvidence]

theorem duplicate_envelopes_induce_equal_probabilistic_observations
    (state : AsyncState)
    (left right : AsyncEnvelope)
    (hRepeat : repeatedObservationPair left right) :
    probabilisticObservationOfAsyncEnvelope state left =
      probabilisticObservationOfAsyncEnvelope state right := by
  cases hRepeat
  rfl

theorem stable_evidence_preserves_posterior_supported_choice
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (state : AsyncState)
    (left right : AsyncEnvelope)
    (hRepeat : repeatedObservationPair left right) :
    posteriorConfidenceDecisionOfAsyncEnvelope thresholds hAdm state left =
      posteriorConfidenceDecisionOfAsyncEnvelope thresholds hAdm state right := by
  cases hRepeat
  rfl

theorem withdrawn_or_expired_route_maps_to_missing_observation
    (state : LocalState)
    (route : FieldRouterLifecycle.LifecycleRoute)
    (hStatus :
      route.status = .withdrawn ∨ route.status = .expired) :
    (observationOfEvidence (evidenceOfLifecycleRoute route) state).delivery =
      ObservationDeliveryClass.missing := by
  rcases hStatus with hWithdrawn | hExpired
  · simp [evidenceOfLifecycleRoute, hWithdrawn, observationOfEvidence]
  · simp [evidenceOfLifecycleRoute, hExpired, observationOfEvidence]

theorem observation_dropout_degradation_bounded
    (state : AsyncState)
    (envelope : AsyncEnvelope)
    (hDropped : envelope.dropped = true)
    (hReady :
      readyForDelivery { envelope with dropped := false, delay := 0 } = true) :
    probabilisticObservationStrength
        (probabilisticObservationOfAsyncEnvelope state envelope) + 2 ≥
      probabilisticObservationStrength
        (probabilisticObservationOfAsyncEnvelope state { envelope with dropped := false, delay := 0 }) := by
  have hMissing :
      (probabilisticObservationOfAsyncEnvelope state envelope).delivery =
        ObservationDeliveryClass.missing :=
    dropped_envelope_maps_to_missing_observation state envelope hDropped
  have hDelivered :
      (probabilisticObservationOfAsyncEnvelope state { envelope with dropped := false, delay := 0 }).delivery =
        ObservationDeliveryClass.delivered :=
    ready_envelope_maps_to_delivered_observation state { envelope with dropped := false, delay := 0 } hReady
  simp [probabilisticObservationStrength, hMissing, hDelivered]

theorem sparse_evidence_has_no_false_confidence_signal
    (state : AsyncState)
    (envelope : AsyncEnvelope)
    (hKnowledge :
      (state.network.localStates envelope.sender envelope.destination).posterior.knowledge =
        ReachabilityKnowledge.unknown)
    (hDropped : envelope.dropped = true)
    (hShape : envelope.projection.shape = CorridorShape.opaque)
    (hSupport : envelope.projection.support = 0) :
    let observation := probabilisticObservationOfAsyncEnvelope state envelope
    observation.observedKnowledge = FieldHypothesis.unknown ∧
      observation.delivery = ObservationDeliveryClass.missing := by
  have hMissing :
      (probabilisticObservationOfAsyncEnvelope state envelope).delivery =
        ObservationDeliveryClass.missing :=
    dropped_envelope_maps_to_missing_observation state envelope hDropped
  unfold probabilisticObservationOfAsyncEnvelope
  simp [evidenceOfAsyncEnvelope, hDropped, observationOfEvidence, hKnowledge]

theorem produced_explicit_candidate_requires_positive_explicit_bayesian_mass
    (state : AsyncState)
    (hAssumptions : state.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.inFlight = [])
    (hHarmony : FieldNetworkAPI.NetworkLocallyHarmonious state.network)
    (candidate : PublishedCandidate)
    (hProduced : ProducedInstalledCandidate state candidate)
    (hShape : candidate.shape = CorridorShape.explicitPath) :
    0 <
      (probabilisticPosteriorOfPublishedCandidate state candidate).pmf
        (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath) := by
  let senderState := state.network.localStates candidate.publisher candidate.destination
  unfold probabilisticPosteriorOfPublishedCandidate
  exact
    bayesianPosterior_positive_of_positive_prior_and_likelihood
      (priorBeliefOfPosteriorState
        (state.network.localStates candidate.publisher candidate.destination).posterior)
      (observationOfEvidence
        (evidenceOfProjection
          { shape := candidate.shape
            support := candidate.support
            hopLower := candidate.hopLower
            hopUpper := candidate.hopUpper })
        (state.network.localStates candidate.publisher candidate.destination))
      (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath)
      (priorBeliefOfPosteriorState_positive
        (state.network.localStates candidate.publisher candidate.destination).posterior
        (defaultHypothesisOfKnowledge ReachabilityKnowledge.explicitPath))
      (by
        have _hKnowledge :
            (state.network.localStates candidate.publisher candidate.destination).posterior.knowledge =
              ReachabilityKnowledge.explicitPath :=
          produced_candidate_requires_explicit_sender_knowledge
            state hAssumptions hEmpty hHarmony candidate hProduced hShape
        unfold observationLikelihood existenceLikelihood knowledgeLikelihood
          deliveryLikelihood witnessLikelihood evidenceOfProjection
          reachabilitySignalOfShape feedbackOfShape
        simp [hShape, defaultHypothesisOfKnowledge, observationOfEvidence])

theorem probabilistic_explicit_candidate_support_boundary
    (state : AsyncState)
    (candidate : PublishedCandidate)
    (hAssumptions : state.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.inFlight = []) :
    state.assumptions = reliableImmediateAssumptions ∧ state.inFlight = [] := by
  exact ⟨hAssumptions, hEmpty⟩

end FieldSystemProbabilistic
