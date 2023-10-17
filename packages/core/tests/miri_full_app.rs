use dioxus::prelude::*;
use dioxus_core::ElementId;
use std::rc::Rc;

#[test]
fn miri_rollover() {
    let mut dom = VirtualDom::new(App);

    _ = dom.rebuild();

    for _ in 0..3 {
        dom.handle_event("click", Rc::new(MouseData::default()), ElementId(2), true);
        dom.process_events();
        _ = dom.render_immediate();
    }
}

#[component]
fn App(cx: Scope) -> Element {
    let mut idx = use_state(cx, || 0);
    let onhover = |_| println!("go!");

    cx.render(rsx! {
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
                (0..**idx).map(|i| rsx! {
                    ChildExample { i: i, onhover: onhover }
                })
            }
        }
    })
}

#[component]
fn ChildExample<'a>(cx: Scope<'a>, i: i32, onhover: EventHandler<'a, MouseEvent>) -> Element {
    cx.render(rsx! {
        li {
            onmouseover: move |e| onhover.call(e),
            "{i}"
        }
    })
}
