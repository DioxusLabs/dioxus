use dioxus::prelude::*;
use std::{
    fmt::{self, Display},
    str::FromStr,
};

#[component]
fn Root() -> Element {
    unimplemented!()
}

#[component]
fn Test() -> Element {
    unimplemented!()
}

#[component]
fn Dynamic(id: usize) -> Element {
    unimplemented!()
}

// Make sure trailing '/'s work correctly
#[test]
fn trailing_slashes_parse() {
    #[derive(Routable, Clone, Copy, PartialEq, Debug)]
    enum Route {
        #[route("/")]
        Root {},
        #[route("/test/")]
        Test {},
        #[route("/:id/test/")]
        Dynamic { id: usize },
    }

    assert_eq!(Route::from_str("/").unwrap(), Route::Root {});
    assert_eq!(Route::from_str("/test/").unwrap(), Route::Test {});
    assert_eq!(Route::from_str("/test").unwrap(), Route::Test {});
    assert_eq!(
        Route::from_str("/123/test/").unwrap(),
        Route::Dynamic { id: 123 }
    );
    assert_eq!(
        Route::from_str("/123/test").unwrap(),
        Route::Dynamic { id: 123 }
    );
}

#[test]
fn without_trailing_slashes_parse() {
    #[derive(Routable, Clone, Copy, PartialEq, Debug)]
    enum RouteWithoutTrailingSlash {
        #[route("/")]
        Root {},
        #[route("/test")]
        Test {},
        #[route("/:id/test")]
        Dynamic { id: usize },
    }

    assert_eq!(
        RouteWithoutTrailingSlash::from_str("/").unwrap(),
        RouteWithoutTrailingSlash::Root {}
    );
    assert_eq!(
        RouteWithoutTrailingSlash::from_str("/test/").unwrap(),
        RouteWithoutTrailingSlash::Test {}
    );
    assert_eq!(
        RouteWithoutTrailingSlash::from_str("/test").unwrap(),
        RouteWithoutTrailingSlash::Test {}
    );
    assert_eq!(
        RouteWithoutTrailingSlash::from_str("/123/test/").unwrap(),
        RouteWithoutTrailingSlash::Dynamic { id: 123 }
    );
    assert_eq!(
        RouteWithoutTrailingSlash::from_str("/123/test").unwrap(),
        RouteWithoutTrailingSlash::Dynamic { id: 123 }
    );
}

// Regression test for https://github.com/DioxusLabs/dioxus/issues/2984
#[test]
fn query_segments_parse() {
    #[derive(Debug, Clone, PartialEq)]
    enum Query {
        Id(u64),
    }

    impl From<&str> for Query {
        fn from(_: &str) -> Self {
            // e.g. split query on `&` and split pairs on `=`
            Query::Id(10)
        }
    }

    impl Display for Query {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "id=10")
        }
    }

    #[component]
    fn Index(query: Query) -> Element {
        rsx! {
            h1 { "Index" }
        }
    }

    #[derive(Debug, Clone, PartialEq, Routable)]
    enum Route {
        #[route("/?:..query")]
        Index { query: Query },
    }

    let route = Route::Index {
        query: Query::Id(10),
    };
    assert_eq!(route.to_string(), "/?id=10");
    let parsed_route = "/?id=10".parse::<Route>().unwrap();
    assert_eq!(parsed_route, route);
}

#[test]
fn optional_query_segments_parse() {
    #[derive(Debug, Clone, PartialEq, Routable)]
    enum Route {
        #[route("/?:query&:other")]
        Index { query: Option<u64>, other: u64 },
    }

    #[component]
    fn Index(query: Option<u64>, other: u64) -> Element {
        rsx! {
            h1 { "Index" }
        }
    }

    let route = Route::Index {
        query: Some(10),
        other: 20,
    };
    assert_eq!(route.to_string(), "/?query=10&other=20");
    let parsed_route = "/?query=10&other=20".parse::<Route>().unwrap();
    assert_eq!(parsed_route, route);

    let route_without_query = Route::Index {
        query: None,
        other: 20,
    };
    assert_eq!(route_without_query.to_string(), "/?other=20");
    let parsed_route_without_query = "/?other=20".parse::<Route>().unwrap();
    assert_eq!(parsed_route_without_query, route_without_query);
    let route_without_query_and_other = Route::Index {
        query: None,
        other: 0,
    };
    assert_eq!(route_without_query_and_other.to_string(), "/?other=0");
    let parsed_route_without_query_and_other = "/".parse::<Route>().unwrap();
    assert_eq!(
        parsed_route_without_query_and_other,
        route_without_query_and_other
    );
}

#[test]
fn child_route_preserves_query_and_hash() {
    #[derive(Routable, Clone, PartialEq, Debug)]
    enum ChildRoute {
        #[route("/search?:query&:word_count")]
        Search { query: String, word_count: usize },
    }

    #[derive(Routable, Clone, PartialEq, Debug)]
    enum Route {
        #[child("")]
        App { child: ChildRoute },
    }

    #[component]
    fn Search(query: String, word_count: usize) -> Element {
        unimplemented!()
    }

    // A query on a child route must survive parsing.
    let parsed = Route::from_str("/search?query=hello&word_count=8").unwrap();
    assert_eq!(
        parsed,
        Route::App {
            child: ChildRoute::Search {
                query: "hello".to_string(),
                word_count: 8,
            }
        }
    );

    // to_string -> from_str must round-trip.
    let original = Route::App {
        child: ChildRoute::Search {
            query: "hello".to_string(),
            word_count: 8,
        },
    };
    assert_eq!(Route::from_str(&original.to_string()).unwrap(), original);

    // Values that percent-encode must round-trip without corruption.
    let encoded = Route::App {
        child: ChildRoute::Search {
            query: "a/b c".to_string(),
            word_count: 1,
        },
    };
    assert_eq!(Route::from_str(&encoded.to_string()).unwrap(), encoded);

    let reserved = Route::App {
        child: ChildRoute::Search {
            query: "a#b".to_string(),
            word_count: 1,
        },
    };
    assert_eq!(reserved.to_string(), "/search?query=a%23b&word_count=1");
    assert_eq!(Route::from_str(&reserved.to_string()).unwrap(), reserved);
}

#[test]
fn nested_child_dynamic_prefix_roundtrip() {
    #[derive(Routable, Clone, PartialEq, Debug)]
    enum ChildRoute {
        #[route("/view")]
        View {},
        #[route("/edit")]
        Edit {},
    }

    #[derive(Routable, Clone, PartialEq, Debug)]
    #[rustfmt::skip]
    enum Route {
        #[nest("/file/:file_id")]
            #[child("")]
            File { file_id: String, child: ChildRoute },
    }

    #[component]
    fn View() -> Element {
        unimplemented!()
    }

    #[component]
    fn Edit() -> Element {
        unimplemented!()
    }

    // A nest-supplied dynamic prefix above a `#[child]` must parse with the parent's
    // dynamic value bound.
    let parsed = Route::from_str("/file/abc/view").unwrap();
    assert_eq!(
        parsed,
        Route::File {
            file_id: "abc".to_string(),
            child: ChildRoute::View {},
        }
    );

    // to_string -> from_str must round-trip with the dynamic value preserved.
    let original = Route::File {
        file_id: "abc".to_string(),
        child: ChildRoute::Edit {},
    };
    assert_eq!(Route::from_str(&original.to_string()).unwrap(), original);
    assert_eq!(original.to_string(), "/file/abc/edit");

    // A space in the parent's dynamic value must percent-encode on emit and decode on parse.
    let spaced = Route::File {
        file_id: "hello world".to_string(),
        child: ChildRoute::View {},
    };
    assert_eq!(spaced.to_string(), "/file/hello%20world/view");
    assert_eq!(Route::from_str(&spaced.to_string()).unwrap(), spaced);
}

#[test]
fn nested_child_dynamic_prefix_with_query() {
    #[derive(Routable, Clone, PartialEq, Debug)]
    enum ChildRoute {
        #[route("/view?:zoom")]
        View { zoom: u32 },
    }

    #[derive(Routable, Clone, PartialEq, Debug)]
    #[rustfmt::skip]
    enum Route {
        #[nest("/file/:file_id")]
            #[child("")]
            File { file_id: String, child: ChildRoute },
    }

    #[component]
    fn View(zoom: u32) -> Element {
        unimplemented!()
    }

    // Parent's dynamic-segment value and child's query must both bind from the URL,
    // confirming the walk-first restructure preserves PR #5613's query-forwarding for
    // children whose parents carry a dynamic prefix.
    let parsed = Route::from_str("/file/abc/view?zoom=200").unwrap();
    assert_eq!(
        parsed,
        Route::File {
            file_id: "abc".to_string(),
            child: ChildRoute::View { zoom: 200 },
        }
    );

    // Round-trip must preserve both the parent dynamic and the child query.
    let original = Route::File {
        file_id: "xyz".to_string(),
        child: ChildRoute::View { zoom: 50 },
    };
    assert_eq!(Route::from_str(&original.to_string()).unwrap(), original);
}

#[test]
fn child_route_typed_parent_segment_error_bubbles_parent() {
    #[derive(Routable, Clone, PartialEq, Debug)]
    enum ChildRoute {
        #[route("/view")]
        View {},
    }

    #[derive(Routable, Clone, PartialEq, Debug)]
    #[rustfmt::skip]
    enum Route {
        #[nest("/file/:file_id")]
            #[child("")]
            File { file_id: usize, child: ChildRoute },
    }

    #[component]
    fn View() -> Element {
        unimplemented!()
    }

    // Happy path: a parseable usize binds and the child variant matches.
    let parsed = Route::from_str("/file/42/view").unwrap();
    assert_eq!(
        parsed,
        Route::File {
            file_id: 42,
            child: ChildRoute::View {},
        }
    );

    // Walk-first failure-isolation: when the parent's typed dyn-seg cannot parse, the
    // error surfaced is the enclosing nest's segment-parse failure, not a child-route
    // mismatch. The stringification must name the failing segment so consumers can
    // locate it.
    let err = Route::from_str("/file/abc/view").unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("file_id") && msg.contains("invalid digit"),
        "expected the parent segment-parse failure in error; got: {msg}"
    );
}

#[test]
fn nested_child_dynamic_prefix_with_hash() {
    #[derive(Routable, Clone, PartialEq, Debug)]
    enum ChildRoute {
        #[route("/view#:anchor")]
        View { anchor: String },
    }

    #[derive(Routable, Clone, PartialEq, Debug)]
    #[rustfmt::skip]
    enum Route {
        #[nest("/file/:file_id")]
            #[child("")]
            File { file_id: String, child: ChildRoute },
    }

    #[component]
    fn View(anchor: String) -> Element {
        unimplemented!()
    }

    // Parent's dynamic-segment value and child's hash must both bind from the URL,
    // confirming the walk-first restructure forwards raw_hash in parallel with raw_query.
    let parsed = Route::from_str("/file/abc/view#section-2").unwrap();
    assert_eq!(
        parsed,
        Route::File {
            file_id: "abc".to_string(),
            child: ChildRoute::View {
                anchor: "section-2".to_string()
            },
        }
    );

    // Round-trip must preserve both the parent dynamic and the child hash.
    let original = Route::File {
        file_id: "xyz".to_string(),
        child: ChildRoute::View {
            anchor: "top".to_string(),
        },
    };
    assert_eq!(Route::from_str(&original.to_string()).unwrap(), original);
}

#[test]
fn dynamic_nest_scoped_layout_parses_variant_after_end_nest() {
    #[derive(Routable, Clone, PartialEq, Debug)]
    #[rustfmt::skip]
    enum Route {
        #[route("/")]
        Home {},
        #[nest("/x/:name")]
            #[layout(Frame)]
                #[route("/")]
                Inside { name: String },
            #[end_layout]
        #[end_nest]
        #[route("/:id")]
        After { id: String },
    }

    #[component]
    fn Home() -> Element {
        unimplemented!()
    }

    #[component]
    fn Frame(name: String) -> Element {
        unimplemented!()
    }

    #[component]
    fn Inside(name: String) -> Element {
        unimplemented!()
    }

    #[component]
    fn After(id: String) -> Element {
        unimplemented!()
    }

    // A dynamic-prefix nest whose layout is closed with an explicit #[end_layout]
    // must leave later variants untouched: no layout wrapping, no borrowed params.
    assert_eq!(
        Route::from_str("/x/foo").unwrap(),
        Route::Inside {
            name: "foo".to_string()
        }
    );
    assert_eq!(
        Route::from_str("/bar").unwrap(),
        Route::After {
            id: "bar".to_string()
        }
    );

    for route in [
        Route::Home {},
        Route::Inside {
            name: "foo".to_string(),
        },
        Route::After {
            id: "bar".to_string(),
        },
    ] {
        assert_eq!(Route::from_str(&route.to_string()).unwrap(), route);
    }
}

#[test]
fn dynamic_nest_scoped_layout_parses_child_after_end_nest() {
    #[derive(Routable, Clone, PartialEq, Debug)]
    enum ChildRoute {
        #[route("/view")]
        View {},
    }

    #[derive(Routable, Clone, PartialEq, Debug)]
    #[rustfmt::skip]
    enum Route {
        #[nest("/x/:name")]
            #[layout(Frame)]
                #[route("/")]
                Inside { name: String },
            #[end_layout]
        #[end_nest]
        #[child("/sub")]
        Sub { child: ChildRoute },
    }

    #[component]
    fn Frame(name: String) -> Element {
        unimplemented!()
    }

    #[component]
    fn Inside(name: String) -> Element {
        unimplemented!()
    }

    #[component]
    fn View() -> Element {
        unimplemented!()
    }

    // A #[child] variant declared after the closed nest/layout region must parse
    // and round-trip without inheriting the nest's layout or parameters.
    assert_eq!(
        Route::from_str("/x/n").unwrap(),
        Route::Inside {
            name: "n".to_string()
        }
    );

    let sub = Route::Sub {
        child: ChildRoute::View {},
    };
    assert_eq!(Route::from_str("/sub/view").unwrap(), sub);
    assert_eq!(Route::from_str(&sub.to_string()).unwrap(), sub);
}
