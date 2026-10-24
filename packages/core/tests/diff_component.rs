use dioxus::prelude::*;
use dioxus_core::{NoOpMutations, ScopeId, current_scope_id, generation};
use dioxus_renderer_oracle::{OracleNodeId, RendererOracle};
use std::cell::{Cell, RefCell};

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
    if template_1.unwrap().template() != template_2.unwrap().template() {
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

#[test]
fn component_replacement_from_empty_output_uses_slot_site() {
    thread_local! {
        static EMPTY_SCOPE: Cell<Option<ScopeId>> = const { Cell::new(None) };
        static EMPTY_DROPS: Cell<usize> = const { Cell::new(0) };
        static LIVE_SCOPE: Cell<Option<ScopeId>> = const { Cell::new(None) };
    }

    fn app() -> Element {
        let phase = dioxus_core::generation() % 2;
        let child = if phase == 0 {
            rsx! { empty_child {} }
        } else {
            rsx! { live_child {} }
        };

        rsx! {
            div {
                "before"
                {child}
                "after"
            }
        }
    }

    #[component]
    fn empty_child() -> Element {
        EMPTY_SCOPE.with(|scope| scope.set(Some(current_scope_id())));
        use_drop(|| {
            EMPTY_DROPS.with(|drops| drops.set(drops.get() + 1));
        });
        rsx! {}
    }

    #[component]
    fn live_child() -> Element {
        LIVE_SCOPE.with(|scope| scope.set(Some(current_scope_id())));
        rsx! { span { "middle" } }
    }

    fn expected_empty() -> Element {
        rsx! {
            div {
                "before"
                "after"
            }
        }
    }

    fn expected_live() -> Element {
        rsx! {
            div {
                "before"
                span { "middle" }
                "after"
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected_empty);
    let empty_scope = EMPTY_SCOPE
        .with(|scope| scope.get())
        .expect("empty child rendered");

    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);
    oracle.assert_matches(expected_live);
    let live_scope = LIVE_SCOPE
        .with(|scope| scope.get())
        .expect("live child rendered");
    assert_ne!(empty_scope, live_scope);
    assert_eq!(EMPTY_DROPS.with(|drops| drops.get()), 1);
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn same_component_empty_output_rerender_uses_owner_slot_site() {
    thread_local! {
        static CHILD_SCOPE: Cell<Option<ScopeId>> = const { Cell::new(None) };
    }

    fn app() -> Element {
        rsx! {
            div {
                "before"
                Child {}
                "after"
            }
        }
    }

    #[component]
    #[allow(non_snake_case)]
    fn Child() -> Element {
        CHILD_SCOPE.with(|scope| scope.set(Some(current_scope_id())));
        if generation() % 2 == 0 {
            rsx! {}
        } else {
            rsx! { span { "middle" } }
        }
    }

    fn expected_empty() -> Element {
        rsx! {
            div {
                "before"
                "after"
            }
        }
    }

    fn expected_live() -> Element {
        rsx! {
            div {
                "before"
                span { "middle" }
                "after"
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected_empty);

    let child_scope = CHILD_SCOPE
        .with(|scope| scope.get())
        .expect("child component rendered");
    dom.mark_dirty(child_scope);
    let summary = oracle.render(&mut dom);
    oracle.assert_matches(expected_live);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn child_owned_signal_prop_updates_before_new_owner_drops() {
    thread_local! {
        static COUNT: RefCell<Option<Signal<i32>>> = const { RefCell::new(None) };
    }

    fn app() -> Element {
        let count = use_signal(|| 0);
        use_hook(|| {
            COUNT.with(|slot| {
                *slot.borrow_mut() = Some(count);
            });
        });

        rsx! {
            Child { count }
        }
    }

    #[component]
    fn Child(count: WriteSignal<i32>) -> Element {
        let doubled = use_memo(move || count() * 2);

        rsx! {
            div { "{doubled}" }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    let mut count = COUNT.with(|slot| slot.borrow().expect("count signal captured"));
    count.set(1);
    dom.render_immediate(&mut NoOpMutations);

    COUNT.with(|slot| {
        *slot.borrow_mut() = None;
    });
}
