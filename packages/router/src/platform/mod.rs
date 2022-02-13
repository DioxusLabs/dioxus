pub trait RouterProvider {
    fn get_current_route(&self) -> String;
    fn listen(&self, callback: Box<dyn Fn()>);
}
