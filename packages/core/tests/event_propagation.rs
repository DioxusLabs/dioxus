use dioxus::prelude::*;
use dioxus_core::ScopeId;
use dioxus_renderer_oracle::RendererOracle;
use std::{any::Any, rc::Rc, sync::Mutex};

static CLICKS: Mutex<usize> = Mutex::new(0);
static CLICK_ORDER: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

fn click_event() -> Event<dyn Any> {
    Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
        true,
    )
}

#[test]
fn events_propagate() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    *CLICKS.lock().unwrap() = 0;

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
            button { onclick: move |evt: MouseEvent| {
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

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    // 1. A click on the top-level div fires the outer handler, so CLICKS = 1.
    let target = oracle.element_id_by_tag("div");
    dom.runtime().handle_event("click", click_event(), target);
    assert_eq!(*CLICKS.lock().unwrap(), 1);

    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);

    // 2. A click on the inner button propagates to the outer div, so CLICKS = 3.
    let target = oracle.element_id_by_tag("button");
    dom.runtime().handle_event("click", click_event(), target);
    assert_eq!(*CLICKS.lock().unwrap(), 3);

    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);

    // 3. Stop-propagation in the button blocks the outer handler, so CLICKS stays at 3.
    let target = oracle.element_id_by_tag("button");
    dom.runtime().handle_event("click", click_event(), target);
    assert_eq!(*CLICKS.lock().unwrap(), 3);

    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);
}

#[test]
fn nested_listeners_in_one_vnode_bubble_from_target_first() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    CLICK_ORDER.lock().unwrap().clear();

    fn app() -> Element {
        rsx! {
            div {
                onclick: move |_| {
                    CLICK_ORDER.lock().unwrap().push("parent");
                },
                button {
                    onclick: move |evt: MouseEvent| {
                        CLICK_ORDER.lock().unwrap().push("child");
                        evt.stop_propagation();
                    }
                }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    let target = oracle.element_id_by_tag("button");
    dom.runtime().handle_event("click", click_event(), target);

    let order = CLICK_ORDER.lock().unwrap();
    assert_eq!(&*order, &["child"]);
}
