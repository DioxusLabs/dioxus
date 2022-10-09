#![allow(unused)]

use dioxus::prelude::*;

fn main() {}

struct AppSettings {}

// ANCHOR: wrap_context
fn use_settings(cx: &ScopeState) -> UseSharedState<AppSettings> {
    use_context::<AppSettings>(cx).expect("App settings not provided")
}
// ANCHOR_END: wrap_context
