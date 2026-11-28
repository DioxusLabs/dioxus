//! `<select>`: `onchange` receives the selected option's value, `selected` binds the option's
//! selectedness (as on the web, where it is the `selected` property), and `onchange` fires for
//! checkboxes and committed text edits.

use blitz_dom::{Document, DocumentConfig};
use blitz_traits::events::{BlitzKeyEvent, KeyState, UiEvent};
use dioxus::prelude::*;
use dioxus_native_dom::DioxusDocument;
use keyboard_types::{Code, Key, Location, Modifiers};

fn key(key: Key) -> UiEvent {
    UiEvent::KeyDown(BlitzKeyEvent {
        key,
        code: Code::Unidentified,
        modifiers: Modifiers::empty(),
        location: Location::Standard,
        is_auto_repeating: false,
        is_composing: false,
        state: KeyState::Pressed,
        text: None,
    })
}

fn app() -> Element {
    let mut choice = use_signal(|| "b".to_string());
    let mut changes = use_signal(|| 0);
    let mut checked = use_signal(|| false);
    let mut text = use_signal(String::new);
    rsx! {
        select {
            id: "s",
            onchange: move |e: FormEvent| {
                choice.set(e.value());
                changes += 1;
            },
            for v in ["a", "b", "c"] {
                option { value: v, selected: choice() == v, {v} }
            }
        }
        input {
            id: "c",
            r#type: "checkbox",
            onchange: move |e: FormEvent| checked.set(e.checked()),
        }
        input { id: "t", onchange: move |e: FormEvent| text.set(e.value()) }
        div { id: "out", "{choice}:{changes}:{checked}:{text}" }
        button { id: "reset", onclick: move |_| choice.set("a".to_string()), "reset" }
    }
}

struct Fixture {
    doc: DioxusDocument,
}

impl Fixture {
    fn new() -> Self {
        let mut doc = DioxusDocument::new(VirtualDom::new(app), DocumentConfig::default());
        doc.initial_build();
        doc.inner_mut().resolve(0.0);
        Self { doc }
    }

    fn id(&self, selector: &str) -> blitz_dom::NodeId {
        self.doc
            .inner()
            .query_selector(selector)
            .unwrap()
            .expect(selector)
    }

    fn focus(&mut self, selector: &str) {
        let id = self.id(selector);
        self.doc.inner_mut().set_focus_to(id);
    }

    fn send(&mut self, event: UiEvent) {
        self.doc.handle_ui_event(event);
        self.doc.poll(None);
        self.doc.inner_mut().resolve(0.0);
    }

    fn click(&mut self, selector: &str) {
        let id = self.id(selector);
        let event = self
            .doc
            .inner()
            .get_node(id)
            .unwrap()
            .synthetic_click_event_data(Modifiers::empty());
        self.send(UiEvent::PointerDown(event.clone()));
        self.send(UiEvent::PointerUp(event));
    }

    fn out(&self) -> String {
        let id = self.id("#out");
        self.doc.inner().get_node(id).unwrap().text_content()
    }

    fn select_value(&self) -> String {
        let id = self.id("#s");
        self.doc.inner().select_value(id)
    }
}

#[test]
fn select_onchange_and_selected_binding() {
    let mut f = Fixture::new();
    assert_eq!(f.select_value(), "b");

    f.focus("#s");
    f.send(key(Key::ArrowDown));
    assert_eq!(f.out(), "c:1:false:");
    assert_eq!(f.select_value(), "c");

    // The app changing `selected` after the user picked an option still applies
    f.click("#reset");
    assert_eq!(f.out(), "a:1:false:");
    assert_eq!(f.select_value(), "a");
}

#[test]
fn checkbox_and_text_onchange() {
    let mut f = Fixture::new();
    f.click("#c");
    assert_eq!(f.out(), "b:0:true:");

    f.focus("#t");
    f.send(key(Key::Character("x".into())));
    assert_eq!(f.out(), "b:0:true:", "no change while editing");
    f.send(key(Key::Enter));
    assert_eq!(f.out(), "b:0:true:x");
}
