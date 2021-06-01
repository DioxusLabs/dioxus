use dioxus_core::patch::Edit;
use dioxus_core::prelude::*;

static SERVER_RENDERED_KEY: &'static str = "abc123";
#[derive(Debug, PartialEq, Props)]
struct ServerRendered {
    name0: String,
    name1: String,
    name2: String,
    name3: String,
    name4: String,
    name5: String,
    name6: String,
    name7: String,
}

fn main() {}

#[cfg(old)]
fn blah() {
    // connect to the host server

    let server_rendered = use_server_rendered((111111, 11111, 11), SERVER_RENDERED_KEY, || {
        ServerRendered {
            name0: "abc".to_string(),
            name1: "abc".to_string(),
            name2: "abc".to_string(),
            name3: "abc".to_string(),
            name4: "abc".to_string(),
            name5: "abc".to_string(),
            name6: "abc".to_string(),
            name7: "abc".to_string(),
        }
    });

    let handler = dioxus_liveview::new_handler()
        .from_directory("abc123") // serve a given directory as the root
        .with_context(|| SomeContext {}) // build out a new context for all of the server-rendered components to share
        .with_route(SERVER_RENDERED_KEY, |ctx: &ServerRendered| {
            //
        })
        .with_route(SERVER_RENDERED_KEY, |ctx| {
            //
        })
        .with_route(SERVER_RENDERED_KEY, |ctx| {
            //
        })
        .with_route(SERVER_RENDERED_KEY, |ctx| {
            //
        })
        .with_route(SERVER_RENDERED_KEY, |ctx| {
            //
        })
        .with_route(SERVER_RENDERED_KEY, |ctx| {
            //
        })
        // depend on the framework, build a different type of handler
        // there are instructions on how to integrate this the various rusty web frameworks in the guide
        .build();

    server.get("abc", handler);
}

fn use_server_rendered<F: Properties>(_p: impl PartialEq, name: &'static str, f: impl Fn() -> F) {}
