use dioxus_html::{
    elements::{prelude::*, *},
    extensions::*,
};
use dioxus_vdom::Element;
use dioxus_vdom_macro::rsx;

#[allow(deprecated)]
use crate::hooks::use_router;

/// The default component to render when an external navigation fails.
#[allow(non_snake_case)]
pub fn FailureExternalNavigation() -> Element {
    #[allow(deprecated)]
    let router = use_router();

    rsx! {
        h1 { "External Navigation Failure!" }
        p {
            "The application tried to programmatically navigate to an external page. This "
            "operation has failed. Click the link below to complete the navigation manually."
        }
        a { onclick: move |_| { router.clear_error() }, "Click here to go back" }
    }
}
