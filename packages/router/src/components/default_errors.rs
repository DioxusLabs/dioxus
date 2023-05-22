use crate::{components::Link, hooks::use_route, navigation::NavigationTarget, routable::Routable};
use dioxus::prelude::*;

#[allow(non_snake_case)]
pub fn FailureExternalNavigation<R: Routable + Clone>(cx: Scope) -> Element {
    let href = use_route::<R>(cx).expect(
        "`FailureExternalNavigation` can only be mounted by the router itself, \
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
pub fn FailureNamedNavigation<R: Routable + Clone>(cx: Scope) -> Element {
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
        Link::<R> {
            target: NavigationTarget::External("https://google.com".into()),
            "Click here to try to fix the failure."
        }
    }
}

#[allow(non_snake_case)]
pub fn FailureRedirectionLimit<R: Routable + Clone>(cx: Scope) -> Element {
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
        Link::<R> {
            target: NavigationTarget::External("https://google.com".into()),
            "Click here to try to fix the failure."
        }
    }
}
