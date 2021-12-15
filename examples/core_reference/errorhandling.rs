//! Example: Error Handling
//! ------------------------
//!
//! Error handling in Dioxus comes in a few flavors. Because Dioxus is a Rust project, Options and Results are obviously
//! the go-to way of wrapping possibly-errored data. However, if a component fails when "unwrapping," everything will crash,
//! the page will deadlock, and your users will be sad.
//!
//! So, obviously, you need to handle your errors.
//!
//! Fortunately, it's easy to avoid panics, even during quick prototyping.
//!
//! Here are a few strategies:
//! - Leverage the ability to return "None" and propagate None directly
//! - Instead of propagating "None" manually, use the "?" syntax sugar
//! - Convert Results into Options with .ok()
//! - Manually display a separate screen by matching on Options/Results
//!
//! There *are* plans to add helpful screens for when apps completely panic in Wasm. However, you should really try to
//! avoid panicking.
use dioxus::prelude::*;
fn main() {}

/// This is one way to go about error handling (just toss things away with unwrap).
/// However, if you get it wrong, the whole app will crash.
/// This is pretty flimsy.
static App: Component<()> = |cx| {
    let data = get_data().unwrap();
    cx.render(rsx!( div { "{data}" } ))
};

/// This is a pretty verbose way of error handling
/// However, it's still pretty good since we don't panic, just fail to render anything
static App1: Component<()> = |cx| {
    let data = match get_data() {
        Some(data) => data,
        None => return None,
    };
    cx.render(rsx!( div { "{data}" } ))
};

/// This is an even better form of error handling.
/// However, it _does_ make the component go blank, which might not be desirable.
///
/// This type of error handling is good when you have "selectors" that produce Some/None based on some state that's
/// already controlled higher in the tree. i.e. displaying a "Username" in a component that should only be shown if
/// a user is logged in.
///
/// Dioxus will throw an error in the console if the None-path is ever taken.
static App2: Component<()> = |cx| {
    let data = get_data()?;
    cx.render(rsx!( div { "{data}" } ))
};

/// This is top-tier error handling since it displays a failure state.
///
/// However, the error is lacking in context.
static App3: Component<()> = |cx, props| match get_data() {
    Some(data) => cx.render(rsx!( div { "{data}" } )),
    None => cx.render(rsx!( div { "Failed to load data :(" } )),
};

/// For errors that return results, it's possible to short-circuit the match-based error handling with `.ok()` which converts
/// a Result<T, V> into an Option<T> and lets you abort rendering by early-returning `None`
static App4: Component<()> = |cx| {
    let data = get_data_err().ok()?;
    cx.render(rsx!( div { "{data}" } ))
};

/// This is great error handling since it displays a failure state... with context!
///
/// Hopefully you'll never need to display a screen like this. It's rather bad taste
static App5: Component<()> = |cx, props| match get_data_err() {
    Ok(data) => cx.render(rsx!( div { "{data}" } )),
    Err(c) => cx.render(rsx!( div { "Failed to load data: {c}" } )),
};

// this fetching function produces "nothing"
fn get_data() -> Option<String> {
    None
}

// this fetching function produces "nothing"
fn get_data_err() -> Result<String, &'static str> {
    Result::Err("Failed!")
}
