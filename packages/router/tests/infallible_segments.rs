// A segment whose parse error is `Infallible` (a `String`, or a catch-all
// collected into strings) must not make the derive emit unreachable code.
#![deny(unreachable_code)]

use dioxus::prelude::*;

#[component]
fn Echo(s: String) -> Element {
    unimplemented!()
}

#[component]
fn Files(path: Vec<String>) -> Element {
    unimplemented!()
}

#[derive(Routable, Clone, PartialEq, Debug)]
enum Route {
    #[route("/echo/:s")]
    Echo { s: String },
    #[route("/files/:..path")]
    Files { path: Vec<String> },
}

#[test]
fn infallible_segments_parse() {
    assert_eq!(
        "/echo/hi".parse::<Route>().unwrap(),
        Route::Echo { s: "hi".into() }
    );
    assert_eq!(
        "/files/a/b".parse::<Route>().unwrap(),
        Route::Files {
            path: vec!["a".into(), "b".into()]
        }
    );
}
