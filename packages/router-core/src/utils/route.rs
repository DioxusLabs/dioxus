use either::Either;
use urlencoding::decode;

use crate::{
    navigation::NavigationTarget,
    routes::{ParameterRoute, Route, RouteContent, Segment},
    RouterState,
};

pub fn route_segment<T: Clone>(
    segment: &Segment<T>,
    values: &[&str],
    state: RouterState<T>,
) -> Either<RouterState<T>, NavigationTarget> {
    route_segment_internal(segment, values, state, None, false)
}

fn route_segment_internal<T: Clone>(
    segment: &Segment<T>,
    values: &[&str],
    state: RouterState<T>,
    mut fallback: Option<RouteContent<T>>,
    mut clear_fallback: bool,
) -> Either<RouterState<T>, NavigationTarget> {
    // fallback
    if let Some(fb) = &segment.fallback {
        fallback = Some(fb.clone());
    }
    if let Some(clear) = &segment.clear_fallback {
        clear_fallback = *clear;
    }

    // index route
    if values.is_empty() {
        if let Some(c) = &segment.index {
            return merge(state, c.clone());
        }
        return Either::Left(state);
    }

    // fixed route
    if let Some(r) = segment.fixed.get(values[0]) {
        return merge_route(values, r, state, fallback, clear_fallback);
    }

    // matching routes
    for (m, r) in &segment.matching {
        if m.matches(values[0]) {
            return merge_parameter_route(values, r, state, fallback, clear_fallback);
        }
    }

    // catchall
    if let Some(c) = &segment.catch_all {
        return merge_parameter_route(values, c.as_ref(), state, fallback, clear_fallback);
    }

    merge_fallback(state, fallback, clear_fallback)
}

fn merge<T: Clone>(
    mut state: RouterState<T>,
    content: RouteContent<T>,
) -> Either<RouterState<T>, NavigationTarget> {
    match content {
        RouteContent::Content(c) => state.content.push(c),
        RouteContent::Redirect(t) => return Either::Right(t),
        RouteContent::MultiContent { main, named } => {
            if let Some(main) = main {
                state.content.push(main);
            }

            for (name, content) in named {
                state.named_content.entry(name).or_default().push(content);
            }
        }
    }
    Either::Left(state)
}

fn merge_route<T: Clone>(
    values: &[&str],
    route: &Route<T>,
    mut state: RouterState<T>,
    fallback: Option<RouteContent<T>>,
    clear_fallback: bool,
) -> Either<RouterState<T>, NavigationTarget> {
    // merge content
    if let Some(c) = &route.content {
        match merge(state, c.clone()) {
            Either::Left(s) => state = s,
            Either::Right(t) => return Either::Right(t),
        }
    }

    if let Some(n) = &route.name {
        state.names.insert(n.clone());
    }

    match (&route.nested, values.is_empty()) {
        (Some(n), _) => route_segment_internal(n, &values[1..], state, fallback, clear_fallback),
        (None, false) => merge_fallback(state, fallback, clear_fallback),
        _ => Either::Left(state),
    }
}

fn merge_parameter_route<T: Clone>(
    values: &[&str],
    route: &ParameterRoute<T>,
    mut state: RouterState<T>,
    fallback: Option<RouteContent<T>>,
    clear_fallback: bool,
) -> Either<RouterState<T>, NavigationTarget> {
    // merge content
    if let Some(c) = &route.content {
        match merge(state, c.clone()) {
            Either::Left(s) => state = s,
            Either::Right(t) => return Either::Right(t),
        }
    }

    if let Some(n) = &route.name {
        state.names.insert(n.clone());
    }

    state.parameters.insert(
        route.key.clone(),
        decode(values[0]).unwrap(/* string already is UTF-8 */).into_owned(),
    );

    match (&route.nested, values.is_empty()) {
        (Some(n), _) => route_segment_internal(n, &values[1..], state, fallback, clear_fallback),
        (None, false) => merge_fallback(state, fallback, clear_fallback),
        _ => Either::Left(state),
    }
}

fn merge_fallback<T: Clone>(
    mut state: RouterState<T>,
    fallback: Option<RouteContent<T>>,
    clear_fallback: bool,
) -> Either<RouterState<T>, NavigationTarget> {
    // fallback clearing
    if clear_fallback {
        state.content.clear();
        state.names.clear();
        state.parameters.clear();
    }

    // fallback content
    match fallback {
        Some(fallback) => merge(state, fallback),
        None => Either::Left(state),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap, HashSet};

    use crate::{
        routes::{multi, ContentAtom},
        Name,
    };

    use super::*;

    fn test_segment() -> Segment<&'static str> {
        Segment::content(ContentAtom("index"))
            .fixed("fixed", Route::content(ContentAtom("fixed")).name::<bool>())
            .matching(
                String::from("matching"),
                ParameterRoute::content::<u8>(ContentAtom("matching"))
                    .nested(Segment::empty().fixed("nested", ContentAtom("matching nested"))),
            )
            .catch_all(
                ParameterRoute::content::<u16>(ContentAtom("catch all"))
                    .nested(Segment::empty().fixed("nested", ContentAtom("catch all nested"))),
            )
            .fixed(
                "nested",
                Route::content(ContentAtom("nested")).name::<u32>().nested(
                    Segment::content(ContentAtom("nested index"))
                        .fixed("again", ContentAtom("nested again")),
                ),
            )
            .fixed("redirect", "/redirect")
            .fixed(
                "fallback",
                Route::content(ContentAtom("fallback")).nested(
                    Segment::empty()
                        .fixed(
                            "keep",
                            Route::content(ContentAtom("keep route")).nested(
                                Segment::content(ContentAtom("keep index"))
                                    .fallback(ContentAtom("keep")),
                            ),
                        )
                        .fixed(
                            "clear",
                            Route::content(ContentAtom("clear route")).nested(
                                Segment::empty()
                                    .fallback(ContentAtom("clear"))
                                    .clear_fallback(true),
                            ),
                        ),
                ),
            )
            .fixed(
                "no_fallback",
                Route::content(ContentAtom("no fallback")).nested(
                    Segment::empty()
                        .fixed(
                            "keep",
                            Route::content(ContentAtom("keep route"))
                                .nested(Segment::empty().clear_fallback(false)),
                        )
                        .fixed(
                            "clear",
                            Route::content(ContentAtom("clear route"))
                                .nested(Segment::empty().clear_fallback(true)),
                        ),
                ),
            )
            .fixed(
                "named_content",
                Route::content(
                    multi(None)
                        .add_named::<i8>(ContentAtom("1"))
                        .add_named::<i16>(ContentAtom("2")),
                )
                .nested(Segment::content(multi(Some(ContentAtom("3"))))),
            )
    }

    #[test]
    fn route_index() {
        let state = route_segment(
            &test_segment(),
            &[],
            RouterState {
                path: String::from("/"),
                can_go_back: false,
                can_go_forward: true,
                ..Default::default()
            },
        );
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(state.content, vec![ContentAtom("index")]);
        assert!(state.names.is_empty());
        assert!(state.parameters.is_empty());
        assert_eq!(state.path, String::from("/"));
        assert_eq!(state.can_go_back, false);
        assert_eq!(state.can_go_forward, true);
    }

    #[test]
    fn route_fixed() {
        let state = route_segment(&test_segment(), &["fixed"], Default::default());
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(state.content, vec![ContentAtom("fixed")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<bool>());
            r
        });
        assert!(state.parameters.is_empty());
    }

    #[test]
    fn route_matching() {
        let state = route_segment(&test_segment(), &["matching"], Default::default());
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(state.content, vec![ContentAtom("matching")]);
        assert!(state.names.is_empty());
        assert_eq!(state.parameters, {
            let mut r = HashMap::new();
            r.insert(Name::of::<u8>(), String::from("matching"));
            r
        });
    }

    #[test]
    fn route_matching_nested() {
        let state = route_segment(&test_segment(), &["matching", "nested"], Default::default());
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(
            state.content,
            vec![ContentAtom("matching"), ContentAtom("matching nested")]
        );
        assert!(state.names.is_empty());
        assert_eq!(state.parameters, {
            let mut r = HashMap::new();
            r.insert(Name::of::<u8>(), String::from("matching"));
            r
        });
    }

    #[test]
    fn route_catch_all() {
        let state = route_segment(&test_segment(), &["invalid"], Default::default());
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(state.content, vec![ContentAtom("catch all")]);
        assert!(state.names.is_empty());
        assert_eq!(state.parameters, {
            let mut r = HashMap::new();
            r.insert(Name::of::<u16>(), String::from("invalid"));
            r
        });
    }

    #[test]
    fn route_catch_all_nested() {
        let state = route_segment(&test_segment(), &["invalid", "nested"], Default::default());
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(
            state.content,
            vec![ContentAtom("catch all"), ContentAtom("catch all nested")]
        );
        assert!(state.names.is_empty());
        assert_eq!(state.parameters, {
            let mut r = HashMap::new();
            r.insert(Name::of::<u16>(), String::from("invalid"));
            r
        });
    }

    #[test]
    fn route_nested_index() {
        let state = route_segment(&test_segment(), &["nested"], Default::default());
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(
            state.content,
            vec![ContentAtom("nested"), ContentAtom("nested index")]
        );
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<u32>());
            r
        });
        assert!(state.parameters.is_empty());
    }

    #[test]
    fn route_nested_again() {
        let state = route_segment(&test_segment(), &["nested", "again"], Default::default());
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(
            state.content,
            vec![ContentAtom("nested"), ContentAtom("nested again")]
        );
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<u32>());
            r
        });
        assert!(state.parameters.is_empty());
    }

    #[test]
    fn route_redirect() {
        let state = route_segment(&test_segment(), &["redirect"], Default::default());
        assert_eq!(state.unwrap_right(), "/redirect".into());
    }

    #[test]
    fn route_fallback_keep() {
        let state = route_segment(
            &test_segment(),
            &["fallback", "keep", "invalid"],
            Default::default(),
        );
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(
            state.content,
            vec![
                ContentAtom("fallback"),
                ContentAtom("keep route"),
                ContentAtom("keep")
            ]
        );
        assert!(state.names.is_empty());
        assert!(state.parameters.is_empty());
    }

    #[test]
    fn route_fallback_clear() {
        let state = route_segment(
            &test_segment(),
            &["fallback", "clear", "invalid"],
            Default::default(),
        );
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(state.content, vec![ContentAtom("clear")]);
        assert!(state.names.is_empty());
        assert!(state.parameters.is_empty());
    }

    #[test]
    fn route_named_content() {
        let state = route_segment(&test_segment(), &["named_content"], Default::default());
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(state.content, vec![ContentAtom("3")]);
        assert_eq!(state.named_content, {
            let mut r = BTreeMap::new();
            r.insert(Name::of::<i8>(), vec![ContentAtom("1")]);
            r.insert(Name::of::<i16>(), vec![ContentAtom("2")]);
            r
        });
        assert!(state.names.is_empty());
        assert!(state.parameters.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn no_fallback() {
        let state = route_segment(
            &test_segment(),
            &["no_fallback", "keep", "invalid"],
            Default::default(),
        );
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(
            state.content,
            vec![
                ContentAtom("fallback"),
                ContentAtom("keep route"),
                ContentAtom("keep")
            ]
        );
        assert!(state.names.is_empty());
        assert!(state.parameters.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn no_fallback_with_clearing() {
        let state = route_segment(
            &test_segment(),
            &["fallback", "clear", "invalid"],
            Default::default(),
        );
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert!(state.content.is_empty());
        assert!(state.names.is_empty());
        assert!(state.parameters.is_empty());
    }

    #[test]
    fn url_encoding() {
        let state = route_segment(&test_segment(), &["%F0%9F%A5%B3"], Default::default());
        assert!(state.is_left());

        let state = state.unwrap_left();
        assert_eq!(state.content, vec![ContentAtom("catch all")]);
        assert!(state.names.is_empty());
        assert_eq!(state.parameters, {
            let mut r = HashMap::new();
            r.insert(Name::of::<u16>(), "🥳".to_string());
            r
        });
    }
}
