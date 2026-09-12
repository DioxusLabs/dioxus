// A variant inside a #[nest] must declare every dynamic parameter of the
// nest as a field. Omitting the field is rejected by the Routable derive
// at the offending variant, not later as an unbound identifier in the
// generated Display and render arms.

use dioxus::prelude::*;

#[derive(Routable, Clone, PartialEq, Debug)]
enum Route {
    #[nest("/user/:id")]
        #[route("/")]
        Profile { id: String },
        // Missing `id: String`; the derive rejects this variant.
        #[route("/settings")]
        Settings {},
    #[end_nest]
    #[route("/about")]
    About {},
}

#[component]
fn Profile(id: String) -> Element {
    unimplemented!()
}

#[component]
fn Settings() -> Element {
    unimplemented!()
}

#[component]
fn About() -> Element {
    unimplemented!()
}

fn main() {}
