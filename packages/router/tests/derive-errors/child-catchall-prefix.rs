// A catch-all in a #[child] prefix is rejected for the same reason as a named dynamic
// parameter, plus one of its own: the catch-all drains the entire remaining path, so a
// child carrying any path segment can never match, while Display still emits the joined
// URL and produces a route that does not parse back.

use dioxus::prelude::*;

#[derive(Routable, Clone, PartialEq, Debug)]
enum ChildRoute {
    #[route("/view")]
    View {},
}

#[derive(Routable, Clone, PartialEq, Debug)]
enum Route {
    #[child("/:..rest")]
    Wild {
        rest: Vec<String>,
        child: ChildRoute,
    },
}

#[component]
fn View() -> Element {
    unimplemented!()
}

fn main() {}
