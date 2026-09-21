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
fn empty_query_arguments_are_omitted() {
    #[derive(Debug, Clone, PartialEq, Routable)]
    enum Route {
        #[route("/?:first&:middle&:last")]
        Index {
            first: Option<u64>,
            middle: String,
            last: String,
        },
    }

    #[component]
    fn Index(first: Option<u64>, middle: String, last: String) -> Element {
        rsx! {
            h1 { "Index" }
        }
    }

    let empty = Route::Index {
        first: None,
        middle: String::new(),
        last: String::new(),
    };
    assert_eq!(empty.to_string(), "/");
    assert_eq!("/".parse::<Route>().unwrap(), empty);
    // and the url the old serialization produced still parses to the same
    assert_eq!("/?middle=&last=".parse::<Route>().unwrap(), empty);

    // an empty argument at the end leaves no dangling `&`
    let only_first = Route::Index {
        first: Some(1),
        middle: String::new(),
        last: String::new(),
    };
    assert_eq!(only_first.to_string(), "/?first=1");
    assert_eq!("/?first=1".parse::<Route>().unwrap(), only_first);

    // nor does one in the middle
    let first_and_last = Route::Index {
        first: Some(1),
        middle: String::new(),
        last: "z".to_string(),
    };
    assert_eq!(first_and_last.to_string(), "/?first=1&last=z");
    assert_eq!("/?first=1&last=z".parse::<Route>().unwrap(), first_and_last);

    // nor does one at the start put a `&` first
    let only_last = Route::Index {
        first: None,
        middle: String::new(),
        last: "z".to_string(),
    };
    assert_eq!(only_last.to_string(), "/?last=z");
    assert_eq!("/?last=z".parse::<Route>().unwrap(), only_last);
}

// Also verify a spread query (`?:..query`) -- empty has no ?
#[test]
fn empty_spread_query_is_omitted() {
    #[derive(Debug, Clone, PartialEq, Default)]
    struct Query(String);

    impl From<&str> for Query {
        fn from(query: &str) -> Self {
            Query(query.to_string())
        }
    }

    impl Display for Query {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.0)
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

    let empty = Route::Index {
        query: Query::default(),
    };
    assert_eq!(empty.to_string(), "/");
    assert_eq!("/".parse::<Route>().unwrap(), empty);

    let full = Route::Index {
        query: Query("id=10".to_string()),
    };
    assert_eq!(full.to_string(), "/?id=10");
    assert_eq!("/?id=10".parse::<Route>().unwrap(), full);
}

// Make sure a route with both a query string and a hash look as desired
#[test]
fn query_and_hash_are_each_omitted_when_empty() {
    #[derive(Debug, Clone, PartialEq, Routable)]
    enum Route {
        #[route("/reset/?:next#:code")]
        Reset { next: String, code: String },
    }

    #[component]
    fn Reset(next: String, code: String) -> Element {
        rsx! {
            h1 { "Reset" }
        }
    }

    let cases = [
        ("/reset/", "", ""),
        ("/reset/?next=/home", "/home", ""),
        ("/reset/#abc", "", "abc"),
        ("/reset/?next=/home#abc", "/home", "abc"),
    ];
    for (url, next, code) in cases {
        let route = Route::Reset {
            next: next.to_string(),
            code: code.to_string(),
        };
        assert_eq!(route.to_string(), url);
        assert_eq!(url.parse::<Route>().unwrap(), route, "{url}");
    }
}
