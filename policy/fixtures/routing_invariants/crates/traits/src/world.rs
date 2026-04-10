pub trait NodeWorldExtension {
    fn poll(&mut self) -> Result<(), RouteError>;
}
