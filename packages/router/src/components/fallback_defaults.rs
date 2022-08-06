use dioxus::prelude::*;

use crate::{
    components::{GoBackButton, Link},
    helpers::use_router_subscription,
    navigation::NavigationTarget,
};

#[allow(non_snake_case)]
pub fn FallbackExternalNavigation(cx: Scope) -> Element {
    // can only be rendered by Router inside itself
    let router = use_router_subscription(&cx).as_mut()?;
    let state = router.state.read().expect("router lock poison");

    // get url
    let url = state.parameters.get("url").cloned().unwrap_or_default();

    cx.render(rsx! {
        h1 { "Oops, you weren't meant to go here!" }
        p { Link {
            target: NavigationTarget::ExternalTarget(url),
            "Click here to get back on track!"
        } }
        p {
            "The application you are using tried to send you to an external website, but it "
            "couldn't. Click the link above to open the external website, or the button below to "
            "go back to the previous page."
        }
        GoBackButton { "Click here to go back" }
    })
}

#[allow(non_snake_case)]
pub fn FallbackNamedNavigation(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "A named navigation error has occurred!" }
        p {
            "If you see this message, the application you are using has a bug. Please report it to "
            "the developer so they can fix it."
            strong { "Thank you!" }
        }
    })
}
