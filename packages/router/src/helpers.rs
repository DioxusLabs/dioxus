use std::{
    any::{type_name, TypeId},
    collections::BTreeMap,
    sync::Arc,
};

use dioxus::prelude::*;
use log::error;
use urlencoding::encode;

use crate::{
    contexts::RouterContext,
    navigation::{NamedNavigationSegment, Query},
};

/// A private hook to subscribe to the router.
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
pub(crate) fn use_router_subscription(cx: &ScopeState) -> &mut Option<RouterContext> {
    let id = cx.use_hook(|| Arc::new(cx.scope_id()));

    cx.use_hook(|| {
        let router = cx.consume_context::<RouterContext>()?;

        let _ = router
            .tx
            .unbounded_send(crate::service::RouterMessage::Subscribe(id.clone()));

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
/// - [`None`] if no target for the `name` was found, or a required parameter was not provided.
///
/// # Panic
/// - In debug builds, when the release build would return [`None`].
#[must_use]
pub(crate) fn construct_named_path(
    (id, name): &(TypeId, &'static str),
    parameters: &[(&'static str, String)],
    query: &Option<Query>,
    targets: &BTreeMap<TypeId, Vec<NamedNavigationSegment>>,
) -> Option<String> {
    // find path layout
    let segments = match targets.get(id) {
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
                        error!(r#"no value for parameter "{key}", no constructed route"#);
                        #[cfg(debug_assertions)]
                        panic!(r#"no value for parameter "{key}""#);
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
        None => {}
        Some(Query::QueryString(qs)) => {
            if qs.is_empty() {
                // do nothing
            } else if qs.starts_with('?') {
                path = format!("{path}{qs}")
            } else {
                path = format!("{path}?{qs}")
            }
        }
        Some(Query::QueryVec(vals)) => {
            if let Ok(q) = serde_urlencoded::to_string(vals) {
                path = format!("{path}?{q}")
            }
        }
    }

    Some(path)
}

/// Create a tuple for [`NamedTarget`]s.
///
/// [`NamedTarget`]: crate::navigation::NavigationTarget::NamedTarget
#[must_use]
pub(crate) fn named_tuple<T: 'static>(_: T) -> (TypeId, &'static str) {
    (TypeId::of::<T>(), type_name::<T>())
}

#[cfg(test)]
mod tests {
    use crate::names::RootIndex;

    use super::*;

    struct Fixed;
    struct Invalid;
    struct Parameter;

    #[test]
    fn named_path_fixed() {
        assert_eq!(
            Some(String::from("/test/nest/")),
            construct_named_path(&named_tuple(Fixed), &[], &None, &test_targets())
        );
    }

    #[test]
    fn named_path_parameters() {
        assert_eq!(
            Some(String::from("/test/value/")),
            construct_named_path(
                &named_tuple(Parameter),
                &vec![("para", String::from("value"))],
                &None,
                &test_targets()
            )
        );
    }

    #[test]
    fn named_path_root() {
        assert_eq!(
            Some(String::from("/")),
            construct_named_path(&named_tuple(RootIndex), &[], &None, &test_targets())
        );
    }

    #[test]
    fn named_path_query_with_marker() {
        assert_eq!(
            Some(String::from("/test/nest/?query=works")),
            construct_named_path(
                &named_tuple(Fixed),
                &[],
                &Some(Query::QueryString(String::from("?query=works"))),
                &test_targets()
            )
        )
    }

    #[test]
    fn named_path_query_without_marker() {
        assert_eq!(
            Some(String::from("/test/nest/?query=works")),
            construct_named_path(
                &named_tuple(Fixed),
                &[],
                &Some(Query::QueryString(String::from("query=works"))),
                &test_targets()
            )
        )
    }

    #[test]
    fn named_path_query_as_vec() {
        assert_eq!(
            Some(String::from("/test/nest/?query=works")),
            construct_named_path(
                &named_tuple(Fixed),
                &[],
                &Some(Query::QueryVec(vec![(
                    String::from("query"),
                    String::from("works")
                )])),
                &test_targets()
            )
        )
    }

    #[cfg(debug_assertions)]
    #[test]
    // TODO: find a better way to test this panic message; see docs for std::any::type_name
    #[should_panic = r#"no route for name "dioxus_router::helpers::tests::Invalid""#]
    fn named_path_not_found_panic_in_debug() {
        let _ = construct_named_path(&named_tuple(Invalid), &[], &None, &test_targets());
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn named_path_not_found_none_in_release() {
        assert_eq!(
            None,
            construct_named_path(&named_tuple(Invalid), &[], &None, &test_targets())
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = r#"no value for parameter "para""#]
    fn named_path_missing_parameter_panic_in_debug() {
        let _ = construct_named_path(&named_tuple(Parameter), &[], &None, &test_targets());
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn named_path_missing_parameter_none_in_release() {
        assert_eq!(
            None,
            construct_named_path(&named_tuple(Parameter), &[], &None, &test_targets())
        );
    }

    fn test_targets() -> BTreeMap<TypeId, Vec<NamedNavigationSegment>> {
        let mut targets = BTreeMap::new();

        targets.insert(
            TypeId::of::<Fixed>(),
            vec![
                NamedNavigationSegment::Fixed(String::from("test")),
                NamedNavigationSegment::Fixed(String::from("nest")),
            ],
        );
        targets.insert(
            TypeId::of::<Parameter>(),
            vec![
                NamedNavigationSegment::Fixed(String::from("test")),
                NamedNavigationSegment::Parameter("para"),
            ],
        );
        targets.insert(TypeId::of::<RootIndex>(), vec![]);

        targets
    }
}
