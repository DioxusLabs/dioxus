use std::{any::TypeId, collections::BTreeMap};

use dioxus::prelude::*;
use log::error;

use crate::{
    helpers::{construct_named_path, use_router_subscription},
    navigation::{
        NamedNavigationSegment,
        NavigationTarget::{self, *},
    },
    service::RouterMessage,
};

/// The properties for a [`Link`].
#[derive(Debug, Props)]
pub struct LinkProps<'a> {
    /// A class to apply to the generated HTML anchor when the `target` route is active.
    ///
    /// This overrides the `active_class` property of a [`Router`].
    ///
    /// [`Router`]: crate::components::Router
    pub active_class: Option<&'a str>,
    /// The children to render within the generated HTML anchor.
    pub children: Element<'a>,
    /// The `class` attribute of the generated HTML anchor.
    ///
    /// When the `target` route is active, `active_class` is appended at the end.
    pub class: Option<&'a str>,
    /// Require the _exact_ target route to be active, for the link to be active. See
    /// [`RouterState::is_active`](crate::state::RouterState::is_active).
    #[props(default)]
    pub exact: bool,
    /// The `id` attribute of the generated HTML anchor.
    pub id: Option<&'a str>,
    /// When [`true`], the `target` will be opened in a new tab.
    #[props(default)]
    pub new_tab: bool,
    /// The `rel` attribute of the generated HTML anchor.
    ///
    /// Defaults to `"noreferrer noopener"` for [`ExternalTarget`] targets.
    pub rel: Option<&'a str>,
    /// The navigation target. Corresponds to the `href` of an HTML anchor.
    #[props(into)]
    pub target: NavigationTarget,
}

/// A link to navigate to another route.
///
/// Only works as a descendent of a [`Router`] component, otherwise it is inactive.
///
/// # Function
/// Unlike a regular HTML anchor, a [`Link`] allows the router to handle the navigation and doesn't
/// cause the browser to load a new page.
///
/// However, in the background a [`Link`] still generates an anchor, which you can use for styling
/// as normal.
///
/// # External targets
/// When the [`Link`]s target is [`ExternalTarget`], the target is used as the `href` directly. This
/// means that a [`Link`] can always navigate to [`ExternalTarget`].
///
/// __TODO:__ explain when this isn't the case
///
/// # Panic
/// - When not nested within a [`Router`], but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// # struct SomeName;
/// rsx! {
///     // a link to a specific path
///     Link {
///         target: InternalTarget(String::from("/some/path")),
///         "Go to path"
///     }
///     // a link to a route with a name
///     Link {
///         target: (SomeName, vec![], None),
///         "Go to named target"
///     }
///     // a link to an external page
///     Link {
///         target: ExternalTarget(String::from("https://dioxuslabs.com/")),
///         "Go to external page"
///     }
/// };
/// ```
///
/// [`Router`]: crate::components::Router
#[allow(non_snake_case)]
pub fn Link<'a>(cx: Scope<'a, LinkProps<'a>>) -> Element {
    let LinkProps {
        active_class,
        children,
        class,
        exact,
        id,
        new_tab,
        rel,
        target,
    } = cx.props;

    // hook up to router
    let router = match use_router_subscription(&cx) {
        Some(x) => x,
        None => {
            error!("`Link` can only be used as a descendent of a `Router`, will be inactive");
            #[cfg(debug_assertions)]
            panic!("`Link` can only be used as a descendent of a `Router`");
            #[cfg(not(debug_assertions))]
            return None;
        }
    };
    let state = router.state.read().expect("router lock poison");
    let tx = router.tx.clone();

    // generate href
    let href = generate_href(target, &state.prefix, &router.named_routes);

    // check if route is active
    let active_class = active_class
        .map(|ac| ac.to_string())
        .and_then(|ac| state.is_active(target, *exact).then(|| format!(" {ac}")))
        .unwrap_or_default();

    // prepare id, class and target for the `a` tag
    let id = id.unwrap_or_default();
    let class = format!("{class}{active_class}", class = class.unwrap_or_default());
    let tag_target = match new_tab {
        true => "_blank",
        false => "",
    };

    // prepare prevented defaults
    let is_router_navigation = !(target.is_external_target() || *new_tab);
    let prevent_default = match is_router_navigation {
        true => "onclick",
        false => "",
    };

    // get rel attribute or apply default if external
    let rel = rel.unwrap_or_else(|| match target.is_external_target() {
        true => "noopener noreferrer",
        false => "",
    });

    cx.render(rsx! {
        a {
            href: "{href}",
            prevent_default: "{prevent_default}",
            onclick: move |_| {
                if is_router_navigation {
                    let _ = tx.unbounded_send(RouterMessage::Push(target.clone()));
                }
            },
            class: "{class}",
            id: "{id}",
            rel: "{rel}",
            target: "{tag_target}",
            children
        }
    })
}

/// Generate a `href` for the `target`.
fn generate_href(
    target: &NavigationTarget,
    prefix: &str,
    targets: &BTreeMap<TypeId, Vec<NamedNavigationSegment>>,
) -> String {
    let href = match target {
        InternalTarget(path) => path.to_string(),
        NamedTarget(name, parameters, query) => {
            // construct_named_path already reports failure in debug
            construct_named_path(name, parameters, query, targets)
                .unwrap_or_else(|| String::from("/"))
        }
        ExternalTarget(href) => return href.to_string(),
    };

    format!("{prefix}{href}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::named_tuple;

    struct Invalid;
    struct Test;

    #[test]
    fn href_path() {
        let path = "/test";
        let prefix = "/pre";
        let target = NavigationTarget::InternalTarget(String::from(path));
        let targets = BTreeMap::new();

        assert_eq!(path, generate_href(&target, "", &targets));
        assert_eq!(
            format!("{prefix}{path}"),
            generate_href(&target, prefix, &targets)
        );
    }

    #[test]
    fn href_name() {
        let prefix = "/pre";
        let target = NavigationTarget::NamedTarget(named_tuple(Test), vec![], None);
        let targets = {
            let mut t = BTreeMap::new();
            t.insert(
                TypeId::of::<Test>(),
                vec![
                    NamedNavigationSegment::Fixed(String::from("test")),
                    NamedNavigationSegment::Fixed(String::from("nest")),
                ],
            );
            t
        };

        assert_eq!(format!("/test/nest/"), generate_href(&target, "", &targets));
        assert_eq!(
            format!("{prefix}/test/nest/"),
            generate_href(&target, prefix, &targets)
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic] // message is checked by `construct_named_path`
    fn href_name_panic_in_debug() {
        generate_href(
            &NavigationTarget::NamedTarget(named_tuple(Invalid), vec![], None),
            "",
            &BTreeMap::new(),
        );
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn href_name_path_in_release() {
        assert_eq!(
            format!("/prefix/"),
            generate_href(
                &NavigationTarget::NamedTarget(named_tuple(Invalid), vec![], None),
                "/prefix",
                &BTreeMap::new(),
            )
        )
    }

    #[test]
    fn href_external() {
        let href = "test";
        let prefix = "/pre";
        let target = NavigationTarget::ExternalTarget(String::from(href));
        let targets = BTreeMap::new();

        assert_eq!(href, generate_href(&target, "", &targets));
        assert_eq!(href, generate_href(&target, prefix, &targets));
    }
}
