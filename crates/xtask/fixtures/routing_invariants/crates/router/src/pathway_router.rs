pub struct FixtureRouter {
    active_routes: std::collections::BTreeMap<u8, u8>,
}

impl FixtureRouter {
    pub fn bad(&mut self) {
        let route_id = 1;
        let _ = self.active_routes.get_mut(&route_id);
    }
}
