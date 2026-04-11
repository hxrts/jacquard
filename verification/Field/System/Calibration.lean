import Field.Information.Calibration
import Field.System.Probabilistic

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemCalibration

open FieldInformationCalibration
open FieldInformationProbabilistic
open FieldRouterProbabilistic
open FieldSystemProbabilistic
open FieldAsyncAPI
open FieldRouterPublication

theorem posterior_explicit_decision_on_produced_candidate_requires_positive_latent_mass
    (state : AsyncState)
    (thresholds : PosteriorConfidenceThresholds)
    (hAdm : PosteriorConfidenceThresholdsAdmissible thresholds)
    (hPositive : 0 < thresholds.corridorMin)
    (candidate : PublishedCandidate)
    (hDecision :
      posteriorConfidenceDecision thresholds hAdm
        (probabilisticPosteriorOfPublishedCandidate state candidate) = .explicitPath) :
    0 <
      probabilisticExplicitPathMass
        (probabilisticPosteriorOfPublishedCandidate state candidate) := by
  have hThreshold :
      (thresholds.explicitPathMin : ℝ) ≤
        probabilisticExplicitPathMass
          (probabilisticPosteriorOfPublishedCandidate state candidate) :=
    posteriorConfidenceDecision_explicitPath_implies_threshold
      thresholds hAdm (probabilisticPosteriorOfPublishedCandidate state candidate) hDecision
  have hThresholdPos : (0 : ℝ) < (thresholds.explicitPathMin : ℝ) := by
    exact_mod_cast (lt_of_lt_of_le hPositive hAdm.2)
  linarith

end FieldSystemCalibration
