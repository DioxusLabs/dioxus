//! Helpers that provide common functionality.

use std::{collections::BTreeMap, sync::Arc};

use dioxus_core::ScopeState;
use log::error;
use urlencoding::encode;

use crate::{
    contexts::RouterContext,
    navigation::{NamedNavigationSegment, Query},
};

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
    vars: &[(&'static str, String)],
    query: &Query,
    targets: &BTreeMap<&'static str, Vec<NamedNavigationSegment>>,
) -> Option<String> {
    // find path layout
    let segments = match targets.get(name) {
        Some(x) => x,
        None => {
            error!(r#"no route for name "{name}""#);
            return None;
        }
    };

    // assemble path
    let mut path = String::from("/");
    for seg in segments {
        match seg {
            NamedNavigationSegment::Fixed(f) => path = format!("{path}{f}/"),
            NamedNavigationSegment::Variable(key) => {
                let value = match vars.iter().find(|(k, _)| k == key) {
                    Some((_, v)) => encode(v).into_owned(),
                    None => {
                        error!(r#"no value for varible "{key}""#);
                        return None;
                    }
                };
                path = format!("{path}{value}/");
            }
        }
    }

    // add query
    match query {
        Query::QNone | Query::QString(None) => {}
        Query::QString(Some(qs)) => {
            if qs.starts_with("?") {
                path = format!("{path}{qs}")
            } else {
                path = format!("{path}?{qs}")
            }
        }
        Query::QVec(vals) => {
            if let Ok(q) = serde_urlencoded::to_string(vals) {
                path = format!("{path}?{q}")
            }
        }
    }

    Some(path)
}
