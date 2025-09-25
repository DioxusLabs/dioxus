use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_elements::SerializedHtmlEventConverter;
use std::{any::Any, rc::Rc};

// This test is intended to be run with Miri, and contains no assertions. If it completes under
// Miri, it has passed.
#[test]
fn miri_rollover() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    for _ in 0..3 {
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
            true,
        );
        dom.runtime().handle_event("click", event, ElementId(2));
        dom.process_events();
        _ = dom.render_immediate_to_vec();
    }
}

fn app() -> Element {
    let mut idx = use_signal(|| 0);
    let onhover = |_| println!("go!");

    rsx! {
        div {
            button {
                onclick: move |_| {
                    idx += 1;
                    println!("Clicked");
                },
                "+"
            }
            button { onclick: move |_| idx -= 1, "-" }
            ul {
                {(0..idx()).map(|i| rsx! {
                    ChildExample { i: i, onhover: onhover }
                })}
            }
        }
    }
}

#[component]
fn ChildExample(i: i32, onhover: EventHandler<MouseEvent>) -> Element {
    rsx! { li { onmouseover: move |e| onhover.call(e), "{i}" } }
}
