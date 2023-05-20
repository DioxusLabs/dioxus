#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router_core::*;
use dioxus_router_macro::*;
use std::str::FromStr;

#[inline_props]
fn Route1(cx: Scope, dynamic: String) -> Element {
    render! {
        div{
            "Route1: {dynamic}"
        }
    }
}

#[inline_props]
fn Route2(cx: Scope) -> Element {
    render! {
        div{
            "Route2"
        }
    }
}

#[inline_props]
fn Route3(cx: Scope, dynamic: u32) -> Element {
    render! {
        div{
            "Route3: {dynamic}"
        }
    }
}

#[inline_props]
fn Route4(cx: Scope, number1: u32, number2: u32) -> Element {
    render! {
        div{
            "Route4: {number1} {number2}"
        }
    }
}

#[inline_props]
fn Route5(cx: Scope, query: String) -> Element {
    render! {
        div{
            "Route5: {query}"
        }
    }
}

#[inline_props]
fn Route6(cx: Scope, extra: Vec<String>) -> Element {
    render! {
        div{
            "Route5: {extra:?}"
        }
    }
}

#[inline_props]
fn Nested(cx: Scope, nested: String) -> Element {
    render! {
        div{
            "Nested: {nested:?}"
        }
    }
}

#[rustfmt::skip]
#[routable]
#[derive(Clone, Debug, PartialEq)]
enum Route {
    #[route("/(dynamic)" Route1)]
    Route1 { dynamic: String },
    #[nest("/(nested)" nested { nested: String } Nested)]
        #[route("/" Route2)]
        Route2 {},
        // #[redirect("/(dynamic)/hello_world")]
        #[route("/(dynamic)" Route3)]
        Route3 { dynamic: u32 },
    #[end_nest]
    #[route("/(number1)/(number2)" Route4)]
    Route4 { number1: u32, number2: u32 },
    #[route("/?(query)" Route5)]
    Route5 { query: String },
    #[route("/(...extra)" Route6)]
    Route6 { extra: Vec<String> },
}

#[test]
fn display_works() {
    let route = Route::Route1 {
        dynamic: "hello".to_string(),
    };

    assert_eq!(route.to_string(), "/hello");

    let route = Route::Route3 { dynamic: 1234 };

    assert_eq!(route.to_string(), "/hello_world/1234");

    let route = Route::Route1 {
        dynamic: "hello_world2".to_string(),
    };

    assert_eq!(route.to_string(), "/hello_world2");

    let route = Route::Route4 {
        number1: 1234,
        number2: 5678,
    };

    assert_eq!(route.to_string(), "/1234/5678");

    let route = Route::Route5 {
        query: "hello".to_string(),
    };

    assert_eq!(route.to_string(), "/?hello");

    let route = Route::Route6 {
        extra: vec!["hello".to_string(), "world".to_string()],
    };

    assert_eq!(route.to_string(), "/hello/world");
}

#[test]
fn from_string_works() {
    let w = "/hello";
    assert_eq!(
        Route::from_str(w),
        Ok(Route::Route1 {
            dynamic: "hello".to_string()
        })
    );
    let w = "/hello/";
    assert_eq!(
        Route::from_str(w),
        Ok(Route::Route1 {
            dynamic: "hello".to_string()
        })
    );

    let w = "/hello_world/1234";
    assert_eq!(Route::from_str(w), Ok(Route::Route3 { dynamic: 1234 }));
    let w = "/hello_world/1234/";
    assert_eq!(Route::from_str(w), Ok(Route::Route3 { dynamic: 1234 }));

    let w = "/hello_world2";
    assert_eq!(
        Route::from_str(w),
        Ok(Route::Route1 {
            dynamic: "hello_world2".to_string()
        })
    );

    let w = "/?x=1234&y=hello";
    assert_eq!(
        Route::from_str(w),
        Ok(Route::Route5 {
            query: "x=1234&y=hello".to_string()
        })
    );

    let w = "/hello_world/hello_world/hello_world";
    assert_eq!(
        Route::from_str(w),
        Ok(Route::Route6 {
            extra: vec![
                "hello_world".to_string(),
                "hello_world".to_string(),
                "hello_world".to_string()
            ]
        })
    );
}

#[test]
fn round_trip() {
    // Route1
    let string = "hello_world2";
    let route = Route::Route1 {
        dynamic: string.to_string(),
    };
    assert_eq!(Route::from_str(&route.to_string()), Ok(route));

    // Route2
    for num in 0..100 {
        let route = Route::Route3 { dynamic: num };
        assert_eq!(Route::from_str(&route.to_string()), Ok(route));
    }

    // Route3
    for num1 in 0..100 {
        for num2 in 0..100 {
            let route = Route::Route4 {
                number1: num1,
                number2: num2,
            };
            assert_eq!(Route::from_str(&route.to_string()), Ok(route));
        }
    }

    // Route4
    let string = "x=1234&y=hello";
    let route = Route::Route5 {
        query: string.to_string(),
    };
    assert_eq!(Route::from_str(&route.to_string()), Ok(route));

    // Route5
    let route = Route::Route6 {
        extra: vec![
            "hello_world".to_string(),
            "hello_world".to_string(),
            "hello_world".to_string(),
        ],
    };
    assert_eq!(Route::from_str(&route.to_string()), Ok(route));
}

fn main() {}
