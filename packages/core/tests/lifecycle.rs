#![allow(unused, non_upper_case_globals)]
#![allow(non_snake_case)]

//! Tests for the lifecycle of components.
use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::html::SerializedHtmlEventConverter;
use dioxus::prelude::*;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

type Shared<T> = Arc<Mutex<T>>;

#[test]
fn manual_diffing() {
    #[derive(Clone)]
    struct AppProps {
        value: Shared<&'static str>,
    }

    fn app(cx: AppProps) -> Element {
        let val = cx.value.lock().unwrap();
        rsx! { div { "{val}" } }
    };

    let value = Arc::new(Mutex::new("Hello"));
    let mut dom = VirtualDom::new_with_props(app, AppProps { value: value.clone() });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    *value.lock().unwrap() = "goodbye";

    assert_eq!(
        dom.rebuild_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(3) },
            HydrateText { path: &[0], value: "goodbye".to_string(), id: ElementId(4) },
            AppendChildren { m: 1, id: ElementId(0) }
        ]
    );
}

#[test]
fn events_generate() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    fn app() -> Element {
        let mut count = use_signal(|| 0);

        match count() {
            0 => rsx! {
                div { onclick: move |_| count += 1,
                    div { "nested" }
                    "Click me!"
                }
            },
            _ => None,
        }
    };

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.handle_event(
        "click",
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())),
        ElementId(1),
        true,
    );

    dom.mark_dirty(ScopeId::ROOT);
    let edits = dom.render_immediate_to_vec();

    assert_eq!(
        edits.edits,
        [
            CreatePlaceholder { id: ElementId(2) },
            ReplaceWith { id: ElementId(1), m: 1 }
        ]
    )
}
