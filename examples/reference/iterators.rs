use dioxus::prelude::*;
fn main() {}

static Example: FC<()> = |cx| {
    let (g, set_g) = use_state_classic(cx, || 0);
    let v = (0..10).map(|f| {
        rsx! {
            li {
                onclick: move |_| set_g(10)
            }
        }
    });
    cx.render(rsx! {
        div {
            {v}
        }
    })
};
