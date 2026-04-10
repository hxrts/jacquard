pub trait ExamplePlanner {
    fn check_candidate(
        &self,
        candidate: &RouteCandidate,
    ) -> Result<RouteAdmissionCheck, RouteError>;

    fn admit_route(
        &self,
        candidate: RouteCandidate,
    ) -> Result<RouteAdmission, RouteError>;
}

pub struct Tick(u64);
pub struct RouteEpoch(u64);

pub fn bad_epoch_wrap(tick: Tick) -> RouteEpoch {
    RouteEpoch(tick.0)
}
