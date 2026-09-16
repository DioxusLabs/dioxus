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

// Regression test for https://github.com/DioxusLabs/dioxus/issues/5792
#[test]
fn empty_query_omits_question_mark() {
    use dioxus_router::routable::FromQuery;

    #[derive(Debug, Clone, PartialEq, Default)]
    struct Params {
        search: Option<String>,
    }

    impl Display for Params {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match &self.search {
                Some(search) => write!(f, "search={search}"),
                None => Ok(()),
            }
        }
    }

    impl FromQuery for Params {
        fn from_query(query: &str) -> Self {
            let search = query
                .split('&')
                .filter_map(|segment| segment.split_once('='))
                .find(|(key, _)| *key == "search")
                .map(|(_, value)| value.to_owned())
                .filter(|value| !value.is_empty());
            Self { search }
        }
    }

    #[derive(Debug, Clone, PartialEq, Routable)]
    enum Route {
        #[route("/translations?:..params")]
        Translations { params: Params },
        #[route("/doc/:doc_id?:folder_id&:page")]
        Doc {
            doc_id: String,
            folder_id: Option<u64>,
            page: Option<u64>,
        },
    }

    #[component]
    fn Translations(params: Params) -> Element {
        unimplemented!()
    }

    #[component]
    fn Doc(doc_id: String, folder_id: Option<u64>, page: Option<u64>) -> Element {
        unimplemented!()
    }

    let cases = [
        (
            Route::Translations {
                params: Params::default(),
            },
            "/translations",
        ),
        (
            Route::Translations {
                params: Params {
                    search: Some("zone".to_owned()),
                },
            },
            "/translations?search=zone",
        ),
        (
            Route::Doc {
                doc_id: "abc".to_string(),
                folder_id: None,
                page: None,
            },
            "/doc/abc",
        ),
        (
            Route::Doc {
                doc_id: "abc".to_string(),
                folder_id: Some(10),
                page: None,
            },
            "/doc/abc?folder_id=10",
        ),
        (
            Route::Doc {
                doc_id: "abc".to_string(),
                folder_id: None,
                page: Some(2),
            },
            "/doc/abc?page=2",
        ),
        (
            Route::Doc {
                doc_id: "abc".to_string(),
                folder_id: Some(10),
                page: Some(2),
            },
            "/doc/abc?folder_id=10&page=2",
        ),
    ];

    for (route, expected) in cases {
        assert_eq!(route.to_string(), expected);
        assert_eq!(Route::from_str(expected).unwrap(), route);
    }
}
