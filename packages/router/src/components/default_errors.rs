use crate::{components::Link, hooks::use_route};
use dioxus::prelude::*;
use dioxus_router_core::prelude::{named, FailureExternalNavigation as FENName, RootIndex};

#[allow(non_snake_case)]
pub fn FailureExternalNavigation(cx: Scope) -> Element {
    let state = use_route(&cx).expect(
        "`FailureExternalNavigation` can only be mounted by the router itself, \
            since it is not exposed",
    );
    let href = state.parameter::<FENName>().expect(
        "`FailureExternalNavigation` cannot be mounted without receiving its parameter, \
            since it is not exposed",
    );

    render! {
        h1 { "External Navigation Failure!" }
        p {
            "The application tried to programmatically navigate to an external page. This "
            "operation has failed. Click the link below to complete the navigation manually."
        }
        a {
            href: "{href}",
            rel: "noopener noreferrer",
            "Click here to fix the failure."
        }
    }
}

#[allow(non_snake_case)]
pub fn FailureNamedNavigation(cx: Scope) -> Element {
    render! {
        h1 { "Named Navigation Failure!" }
        p {
            "The application has tried to navigate to an unknown name. This is a bug. Please "
            "inform the developer, so they can fix it."
            b { "Thank you!" }
        }
        p {
            "We are sorry for the inconvenience. The link below may help to fix the problem, but "
            "there is no guarantee."
        }
        Link {
            target: named::<RootIndex>(),
            "Click here to try to fix the failure."
        }
    }
}

#[allow(non_snake_case)]
pub fn FailureRedirectionLimit(cx: Scope) -> Element {
    render! {
        h1 { "Redirection Limit Failure!" }
        p {
            "The application seems to have entered into an endless redirection loop. This is a "
            "bug. Please inform the developer, so they can fix it."
            b { "Thank you!" }
        }
        p {
            "We are sorry for the inconvenience. The link below may help to fix the problem, but "
            "there is no guarantee."
        }
        Link {
            target: named::<RootIndex>(),
            "Click here to try to fix the failure."
        }
    }
}
