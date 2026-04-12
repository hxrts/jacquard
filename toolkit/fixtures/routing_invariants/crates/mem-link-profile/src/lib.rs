use jacquard_core::MaterializedRoute;

pub struct FixtureTransport {
    route: Option<MaterializedRoute>,
}

impl FixtureTransport {
    pub fn bad(&self) -> bool {
        self.route.is_some()
    }
}
