use crate::{hooks::use_generic_router, routable::Routable};
use dioxus::prelude::*;

/// The default component to render when an external navigation fails.
#[allow(non_snake_case)]
pub fn FailureExternalNavigation<R: Routable + Clone>(cx: Scope) -> Element {
    let router = use_generic_router::<R>(cx);

    render! {
        h1 { "External Navigation Failure!" }
        p {
            "The application tried to programmatically navigate to an external page. This "
            "operation has failed. Click the link below to complete the navigation manually."
        }
        a {
            onclick: move |_| {
                router.clear_error()
            },
            "Click here to go back"
        }
    }
}
