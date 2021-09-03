use dioxus::prelude::*;
use dioxus::ssr;

pub static Example: FC<()> = |cx| {
    let as_string = use_state(cx, || {
        // Currently, SSR is only supported for whole VirtualDOMs
        // This is an easy/low hanging fruit to improve upon
        let mut dom = VirtualDom::new(SomeApp);
        dom.rebuild();
        ssr::render_vdom(&dom, |c| c)
    });

    cx.render(rsx! {
        div { "{as_string}" }
    })
};

static SomeApp: FC<()> = |cx| {
    cx.render(rsx! {
        div { style: {background_color: "blue"}
            h1 {"Some amazing app or component"}
            p {"Things are great"}
        }
    })
};
