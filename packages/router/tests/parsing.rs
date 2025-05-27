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
