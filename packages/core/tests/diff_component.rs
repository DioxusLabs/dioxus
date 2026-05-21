use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

/// When returning sets of components, we do a light diff of the contents to preserve some react-like functionality
///
/// This means that nav_bar should never get re-created and that we should only be swapping out
/// different pointers
#[test]
fn component_swap() {
    // Check that templates with the same structure are deduplicated at compile time
    // If they are not, this test will fail because it is being run in debug mode where templates are not deduped
    let dynamic = 0;
    let template_1 = rsx! { "{dynamic}" };
    let template_2 = rsx! { "{dynamic}" };
    if template_1.unwrap().template != template_2.unwrap().template {
        return;
    }

    fn app() -> Element {
        let mut render_phase = use_signal(|| 0);

        render_phase += 1;

        match render_phase() {
            0 => rsx! {
                nav_bar {}
                dash_board {}
            },
            1 => rsx! {
                nav_bar {}
                dash_results {}
            },
            2 => rsx! {
                nav_bar {}
                dash_board {}
            },
            3 => rsx! {
                nav_bar {}
                dash_results {}
            },
            4 => rsx! {
                nav_bar {}
                dash_board {}
            },
            _ => rsx!("blah"),
        }
    }

    fn nav_bar() -> Element {
        rsx! {
            h1 { id: "nav",
                "NavBar"
                for _ in 0..3 {
                    nav_link {}
                }
            }
        }
    }

    fn nav_link() -> Element {
        rsx!( h1 { "nav_link" } )
    }

    fn dash_board() -> Element {
        rsx!( div { "dashboard" } )
    }

    fn dash_results() -> Element {
        rsx!( div { "results" } )
    }

    fn expected_dashboard() -> Element {
        rsx! {
            h1 { id: "nav",
                "NavBar"
                h1 { "nav_link" }
                h1 { "nav_link" }
                h1 { "nav_link" }
            }
            div { "dashboard" }
        }
    }

    fn expected_results() -> Element {
        rsx! {
            h1 { id: "nav",
                "NavBar"
                h1 { "nav_link" }
                h1 { "nav_link" }
                h1 { "nav_link" }
            }
            div { "results" }
        }
    }

    Sequence::new()
        .track_identity_by("id")
        .render_with_expected(app, expected_results())
        .render_with_expected(app, expected_dashboard())
        .render_with_expected(app, expected_results())
        .render_with_expected(app, expected_dashboard())
        .run();
}
