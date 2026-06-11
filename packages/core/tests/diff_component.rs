use dioxus::prelude::*;
use dioxus_core::ScopeId;
use dioxus_renderer_oracle::{OracleNodeId, RendererOracle};

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

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected_results);
    let nav_identity = identity_by_attr(&oracle, "id", "nav");

    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);
    oracle.assert_matches(expected_dashboard);
    assert_eq!(identity_by_attr(&oracle, "id", "nav"), nav_identity);

    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);
    oracle.assert_matches(expected_results);
    assert_eq!(identity_by_attr(&oracle, "id", "nav"), nav_identity);

    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);
    oracle.assert_matches(expected_dashboard);
    assert_eq!(identity_by_attr(&oracle, "id", "nav"), nav_identity);
}

fn identity_by_attr(oracle: &RendererOracle, attr: &str, value: &str) -> OracleNodeId {
    oracle
        .identities_by_attr(attr)
        .into_iter()
        .find_map(|(current_value, id)| (current_value == value).then_some(id))
        .unwrap_or_else(|| panic!("no live element with `{attr}={value}` found in the oracle DOM"))
}
