#![allow(unused, non_upper_case_globals, non_snake_case)]

use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_signals::*;

#[test]
fn create_signals_global() {
    let mut dom = VirtualDom::new(|cx| {
        render! {
            for _ in 0..10 {
                Child {}
            }
        }
    });

    fn Child(cx: Scope) -> Element {
        let signal = create_without_cx();
        let mapped = MappedSignal::new(signal, |v| v.as_bytes());

        render! { "{signal:?}", "{mapped:?}" }
    }

    let _edits = dom.rebuild().santize();

    fn create_without_cx() -> Signal<String> {
        Signal::new("hello world".to_string())
    }
}
