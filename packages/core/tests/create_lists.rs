use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

// A real-world usecase of templates at peak performance
// In react, this would be a lot of node creation.
//
// In Dioxus, we memoize the rsx! body and simplify it down to a few template loads
//
// Also note that the IDs increase linearly. This lets us drive a vec on the renderer for O(1) re-indexing
fn app() -> Element {
    rsx! {
        div {
            for i in 0..3 {
                div {
                    h1 { "hello world! "}
                    p { "{i}" }
                }
            }
        }
    }
}

#[test]
fn list_renders() {
    Sequence::new()
        .render_with_expected(
            app,
            rsx! {
                div {
                    div {
                        h1 { "hello world! " }
                        p { "0" }
                    }
                    div {
                        h1 { "hello world! " }
                        p { "1" }
                    }
                    div {
                        h1 { "hello world! " }
                        p { "2" }
                    }
                }
            },
        )
        .run();
}
