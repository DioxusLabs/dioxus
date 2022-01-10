pub trait RouterProvider {
    fn get_current_route(&self) -> String;
    fn subscribe_to_route_changes(&self, callback: Box<dyn Fn(String)>);
}

enum RouteChange {
    LinkTo(String),
    Back(String),
}
