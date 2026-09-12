// A #[child] prefix must be static. A dynamic parameter there would be bound on this
// enum's variant while rendering is delegated to the child Routable, whose components
// receive props only from their own segments. The derive rejects it at the prefix and
// points at #[nest], which keeps the parameter on this enum where layouts and sibling
// routes receive it as an ordinary prop.

use dioxus::prelude::*;

#[derive(Routable, Clone, PartialEq, Debug)]
enum ChildRoute {
    #[route("/view")]
    View {},
}

#[derive(Routable, Clone, PartialEq, Debug)]
enum Route {
    #[child("/file/:file_id")]
    File { file_id: String, child: ChildRoute },
}

#[component]
fn View() -> Element {
    unimplemented!()
}

fn main() {}
