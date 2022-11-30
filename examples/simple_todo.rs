use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let mut idx = use_state(cx, || 0);
    let onhover = |h| println!("go!");

    cx.render(rsx! {
        div {
            button { onclick: move |_| idx += 1, "+" }
            button { onclick: move |_| idx -= 1, "-" }
            ul {
                (0..**idx).map(|i| rsx! {
                    Child { i: i, onhover: onhover }
                })
            }
        }
    })
}

#[inline_props]
fn Child<'a>(cx: Scope<'a>, i: i32, onhover: EventHandler<'a, MouseEvent>) -> Element {
    cx.render(rsx! {
        li {
            onmouseover: move |e| onhover.call(e),
            "{i}"
        }
    })
}
