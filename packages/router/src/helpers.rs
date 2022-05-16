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
/// single component, but not recommended. Multiple subscriptions will be discarded.
///
/// # Return values
/// - [`None`], when the current component isn't a descendant of a [`Router`].
/// - Otherwise [`Some`].
///
/// [`Router`]: crate::components::router
#[must_use]
#[allow(clippy::mut_from_ref)]
pub(crate) fn sub_to_router(cx: &ScopeState) -> &mut Option<RouterContext> {
    let id = cx.use_hook(|_| Arc::new(cx.scope_id()));

    cx.use_hook(|_| {
        let router = cx.consume_context::<RouterContext>()?;

        let _ = router
            .tx
            .unbounded_send(crate::service::RouterMessage::Subscribe(id.clone()))
            .ok();

        Some(router)
    })
}

/// Constructs a path for named navigation.
///
/// # Parameters
/// - `name`: the name to navigate to
/// - `parameters`: a list of parameters that can be inserted into the path
/// - `query`: the query to append to the path
/// - `targets`: the list of possible targets for the named navigation
///
/// # Return values:
/// - [`Some`] if the navigation was successful.
/// - [`None`] if no target for the `name` was found, or a required parameter was not provided. Only
///   in release builds.
/// - [`panic!`] in debug builds, when the release build would return [`None`].
#[must_use]
pub(crate) fn construct_named_path(
    name: &'static str,
    parameters: &[(&'static str, String)],
    query: &Query,
    targets: &BTreeMap<&'static str, Vec<NamedNavigationSegment>>,
) -> Option<String> {
    // find path layout
    let segments = match targets.get(name) {
        Some(x) => x,
        None => {
            error!(r#"no route for name "{name}""#);
            #[cfg(debug_assertions)]
            panic!(r#"no route for name "{name}""#);
            #[cfg(not(debug_assertions))]
            return None;
        }
    };

    // assemble path
    let mut path = String::from("/");
    for seg in segments {
        match seg {
            NamedNavigationSegment::Fixed(f) => path = format!("{path}{f}/"),
            NamedNavigationSegment::Parameter(key) => {
                let value = match parameters.iter().find(|(k, _)| k == key) {
                    Some((_, v)) => encode(v).into_owned(),
                    None => {
                        error!(r#"no value for variable "{key}""#);
                        #[cfg(debug_assertions)]
                        panic!(r#"no value for variable "{key}""#);
                        #[cfg(not(debug_assertions))]
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
            if qs.starts_with('?') {
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
