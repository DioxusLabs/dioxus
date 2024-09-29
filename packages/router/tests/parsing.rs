use dioxus::prelude::*;
use std::str::FromStr;

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
