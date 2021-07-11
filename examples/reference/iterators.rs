use dioxus::prelude::*;
fn main() {}

static Example: FC<()> = |cx| {
    let g = use_state(cx, || 0);
    let v = (0..10).map(|f| {
        rsx! {
            li {
                onclick: move |_| g.set(10)
            }
        }
    });
    cx.render(rsx! {
        div {
            {v}
        }
    })
};
