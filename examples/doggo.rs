use dioxus::prelude::*;
fn main() {}

pub static Example: FC<()> = |cx| {
    let (g, set_g) = use_state(cx, || 0).classic();
    let v = (0..10).map(move |f| {
        rsx!(li {
            onclick: move |_| set_g(10)
        })
    });
    cx.render(rsx! {
        div {
            {v}
        }
    })
};
