//! Helpers that provide common functionality.

use std::{collections::BTreeMap, sync::Arc};

use dioxus_core::ScopeState;
use log::error;

use crate::{contexts::RouterContext, navigation::NamedNavigationSegment};

/// A private "hook" to subscribe to the router.
///
/// Used to reduce redundancy within other components/hooks. Safe to call multiple times for a
/// single component, but not recommended.
///
/// # Return values
/// - [`None`], when the current component isn't a descendant of a
///   [Router](crate::components::Router).
/// - Otherwise [`Some`].
pub(crate) fn sub_to_router<'a>(cx: &'a ScopeState) -> &'a mut Option<RouterContext> {
    let id = cx.use_hook(|_| Arc::new(cx.scope_id()));

    cx.use_hook(|_| {
        let router = cx.consume_context::<RouterContext>()?;

        router
            .tx
            .unbounded_send(crate::service::RouterMessage::Subscribe(id.clone()))
            .unwrap();

        Some(router)
    })
}

pub(crate) fn construct_named_path(
    name: &'static str,
    vars: &Vec<(&'static str, String)>,
    targets: &BTreeMap<&'static str, Vec<NamedNavigationSegment>>,
) -> Option<String> {
    let segments = match targets.get(name) {
        Some(x) => x,
        None => {
            error!(r#"no route for name "{name}""#);
            return None;
        }
    };

    let mut path = String::from("/");

    for seg in segments {
        match seg {
            NamedNavigationSegment::Fixed(f) => path = format!("{path}{f}/"),
            NamedNavigationSegment::Variable(key) => {
                let (_, value) = match vars.iter().find(|(k, _)| k == key) {
                    Some(v) => v,
                    None => {
                        error!(r#"no value for varible "{key}""#);
                        return None;
                    }
                };
                path = format!("{path}{value}/");
            }
        }
    }

    Some(path)
}
