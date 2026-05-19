use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;
use std::{any::Any, rc::Rc, sync::Mutex};

static CLICKS: Mutex<usize> = Mutex::new(0);

fn click_event() -> Event<dyn Any> {
    Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
        true,
    )
}

#[test]
fn events_propagate() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

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

    Sequence::new()
        // Initial render. The DOM doesn't change across steps; what changes is
        // the internal CLICKS counter that the click handlers mutate.
        .render_with(app)
        // 1. A click on the top-level div fires the outer handler, so CLICKS = 1.
        .then(|dom, oracle| {
            let target = oracle.element_id_by_tag("div");
            dom.runtime().handle_event("click", click_event(), target);
            assert_eq!(*CLICKS.lock().unwrap(), 1);
        })
        .render_with(app)
        // 2. A click on the inner button propagates to the outer div, so CLICKS = 3.
        .then(|dom, oracle| {
            let target = oracle.element_id_by_tag("button");
            dom.runtime().handle_event("click", click_event(), target);
            assert_eq!(*CLICKS.lock().unwrap(), 3);
        })
        .render_with(app)
        // 3. Stop-propagation in the button blocks the outer handler, so CLICKS stays at 3.
        .then(|dom, oracle| {
            let target = oracle.element_id_by_tag("button");
            dom.runtime().handle_event("click", click_event(), target);
            assert_eq!(*CLICKS.lock().unwrap(), 3);
        })
        .render_with(app)
        .run();
}
