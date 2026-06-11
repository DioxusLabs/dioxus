use dioxus::prelude::*;
use dioxus_core::ElementId;
use std::{any::Any, rc::Rc, sync::Mutex};

static CLICKS: Mutex<usize> = Mutex::new(0);

#[test]
fn events_propagate() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    // Top-level click is registered
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
        true,
    );
    dom.runtime().handle_event("click", event, ElementId(1));
    assert_eq!(*CLICKS.lock().unwrap(), 1);

    // break reference....
    for _ in 0..5 {
        dom.mark_dirty(ScopeId(0));
        _ = dom.render_immediate_to_vec();
    }

    // Lower click is registered
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
        true,
    );
    dom.runtime().handle_event("click", event, ElementId(2));
    assert_eq!(*CLICKS.lock().unwrap(), 3);

    // break reference....
    for _ in 0..5 {
        dom.mark_dirty(ScopeId(0));
        _ = dom.render_immediate_to_vec();
    }

    // Stop propagation occurs
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
        true,
    );
    dom.runtime().handle_event("click", event, ElementId(2));
    assert_eq!(*CLICKS.lock().unwrap(), 3);
}

fn app() -> Element {
    rsx! {
        div { onclick: move |_| {
                println!("top clicked");
                *CLICKS.lock().unwrap() += 1;
            },

            {vec![
                rsx! {
                    problematic_child {}
                }
            ].into_iter()}
        }
    }
}

fn problematic_child() -> Element {
    rsx! {
        button { onclick: move |evt| {
                println!("bottom clicked");
                let mut clicks = CLICKS.lock().unwrap();
                if *clicks == 3 {
                    evt.stop_propagation();
                } else {
                    *clicks += 1;
                }
            } }
    }
}
