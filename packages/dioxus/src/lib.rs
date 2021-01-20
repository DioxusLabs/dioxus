pub mod prelude {
    pub use dioxus_core::prelude::*;
}

use dioxus_core::prelude::FC;

// Re-export core completely
pub use dioxus_core as core;

struct App {}
impl App {
    fn at(&mut self, pat: &str) -> Route {
        todo!()
    }

    // start the app in a new thread
    // pass updates to it via state manager?
    fn start(self) {}
}

fn new() -> App {
    todo!()
}

struct Router {}

struct Route {}

struct RouteInfo {}

impl RouteInfo {
    fn query<T>(&self) -> T {
        todo!()
    }
}

impl Route {
    fn serve(&self, app: FC<RouteInfo>) {}
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn app_example() {
        let mut app = crate::new();

        app.at("/").serve(|ctx| {
            let parsed: u32 = ctx.props.query();
            html! { <div>"hello " </div> }
        });

        app.start();
    }
}
