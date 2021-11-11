#![allow(unused, non_upper_case_globals)]

//! Tests for the lifecycle of components.
use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core::DomEdit::*;
use dioxus_core::ScopeId;

use dioxus_core_macro::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
use std::sync::{Arc, Mutex};

mod test_logging;

const IS_LOGGING_ENABLED: bool = true;
type Shared<T> = Arc<Mutex<T>>;

#[test]
fn manual_diffing() {
    test_logging::set_up_logging(IS_LOGGING_ENABLED);

    struct AppProps {
        value: Shared<&'static str>,
    }

    static App: FC<AppProps> = |cx, props| {
        let val = props.value.lock().unwrap();
        cx.render(rsx! { div { "{val}" } })
    };

    let value = Arc::new(Mutex::new("Hello"));
    let mut dom = VirtualDom::new_with_props(
        App,
        AppProps {
            value: value.clone(),
        },
    );

    let _ = dom.rebuild();

    *value.lock().unwrap() = "goodbye";

    let edits = dom.rebuild();

    log::debug!("edits: {:?}", edits);
}

#[test]
fn events_generate() {
    static App: FC<()> = |cx, _| {
        let mut count = use_state(cx, || 0);

        let inner = match *count {
            0 => {
                rsx! {
                    div {
                        onclick: move |_| count += 1,
                        div {
                            "nested"
                        }
                        "Click me!"
                    }
                }
            }
            _ => todo!(),
        };

        cx.render(inner)
    };

    let mut dom = VirtualDom::new(App);
    let mut channel = dom.get_scheduler_channel();
    assert!(dom.has_any_work());

    let edits = dom.work_with_deadline(|| false);
    assert_eq!(
        edits[0].edits,
        [
            CreateElement {
                tag: "div",
                root: 0,
            },
            NewEventListener {
                event_name: "click",
                scope: ScopeId(0),
                root: 0,
            },
            CreateElement {
                tag: "div",
                root: 1,
            },
            CreateTextNode {
                text: "nested",
                root: 2,
            },
            AppendChildren { many: 1 },
            CreateTextNode {
                text: "Click me!",
                root: 3,
            },
            AppendChildren { many: 2 },
        ]
    )
}

#[test]
fn components_generate() {
    static App: FC<()> = |cx, _| {
        let mut render_phase = use_state(cx, || 0);
        render_phase += 1;

        cx.render(match *render_phase {
            0 => rsx!("Text0"),
            1 => rsx!(div {}),
            2 => rsx!("Text2"),
            3 => rsx!(Child {}),
            4 => rsx!({ None as Option<()> }),
            5 => rsx!("text 3"),
            6 => rsx!({ (0..2).map(|f| rsx!("text {f}")) }),
            7 => rsx!(Child {}),
            _ => todo!(),
        })
    };

    static Child: FC<()> = |cx, _| {
        println!("running child");
        cx.render(rsx! {
            h1 {}
        })
    };

    let mut dom = VirtualDom::new(App);
    let edits = dom.rebuild();
    assert_eq!(
        edits.edits,
        [
            CreateTextNode {
                text: "Text0",
                root: 0,
            },
            AppendChildren { many: 1 },
        ]
    );

    let edits = dom.hard_diff(&ScopeId(0)).unwrap();
    assert_eq!(
        edits.edits,
        [
            CreateElement {
                tag: "div",
                root: 1,
            },
            ReplaceWith { root: 0, m: 1 },
        ]
    );

    let edits = dom.hard_diff(&ScopeId(0)).unwrap();
    assert_eq!(
        edits.edits,
        [
            CreateTextNode {
                text: "Text2",
                root: 2,
            },
            ReplaceWith { root: 1, m: 1 },
        ]
    );

    let edits = dom.hard_diff(&ScopeId(0)).unwrap();
    assert_eq!(
        edits.edits,
        [
            CreateElement { tag: "h1", root: 3 },
            ReplaceWith { root: 2, m: 1 },
        ]
    );

    let edits = dom.hard_diff(&ScopeId(0)).unwrap();
    assert_eq!(
        edits.edits,
        [CreatePlaceholder { root: 4 }, ReplaceWith { root: 3, m: 1 },]
    );

    let edits = dom.hard_diff(&ScopeId(0)).unwrap();
    assert_eq!(
        edits.edits,
        [
            CreateTextNode {
                text: "text 3",
                root: 5,
            },
            ReplaceWith { root: 4, m: 1 },
        ]
    );

    let edits = dom.hard_diff(&ScopeId(0)).unwrap();
    assert_eq!(
        edits.edits,
        [
            CreateTextNode {
                text: "text 0",
                root: 6,
            },
            CreateTextNode {
                text: "text 1",
                root: 7,
            },
            ReplaceWith { root: 5, m: 2 },
        ]
    );

    let edits = dom.hard_diff(&ScopeId(0)).unwrap();
    assert_eq!(
        edits.edits,
        [
            CreateElement { tag: "h1", root: 8 },
            ReplaceWith { root: 6, m: 1 },
            Remove { root: 7 },
        ]
    );
}
