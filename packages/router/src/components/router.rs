use crate::{cfg::RouterCfg, RouterContext, RouterService};
use dioxus::prelude::*;

/// The props for the [`Router`](fn.Router.html) component.
#[derive(Props)]
pub struct RouterProps<'a> {
    /// The routes and elements that should be rendered when the path matches.
    ///
    /// If elements are not contained within Routes, the will be rendered
    /// regardless of the path.
    pub children: Element<'a>,

    /// The URL to point at
    ///
    /// This will be used to trim any latent segments from the URL when your app is
    /// not deployed to the root of the domain.
    pub base_url: Option<&'a str>,

    /// Hook into the router when the route is changed.
    ///
    /// This lets you easily implement redirects
    #[props(default)]
    pub onchange: EventHandler<'a, RouterContext>,

    /// Set the active class of all Link components contained in this router.
    ///
    /// This is useful if you don't want to repeat the same `active_class` prop value in every Link.
    /// By default set to `"active"`.
    pub active_class: Option<&'a str>,

    /// Set the initial url.
    #[props(!optional, into)]
    pub initial_url: Option<String>,
}

/// A component that conditionally renders children based on the current location of the app.
///
/// Uses BrowserRouter in the browser and HashRouter everywhere else.
///
/// Will fallback to HashRouter is BrowserRouter is not available, or through configuration.
#[allow(non_snake_case)]
pub fn Router<'a>(cx: Scope<'a, RouterProps<'a>>) -> Element {
    let svc = cx.use_hook(|| {
        cx.provide_context(RouterService::new(
            cx,
            RouterCfg {
                base_url: cx.props.base_url.map(|s| s.to_string()),
                active_class: cx.props.active_class.map(|s| s.to_string()),
                initial_url: cx.props.initial_url.clone(),
            },
        ))
    });

    // next time we run the rout_found will be filled
    if svc.route_found.get().is_none() {
        cx.props.onchange.call(svc.clone());
    }

    cx.render(rsx!(&cx.props.children))
}
