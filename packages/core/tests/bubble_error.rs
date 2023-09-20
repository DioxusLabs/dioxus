//! we should properly bubble up errors from components

use dioxus::prelude::*;

fn app(cx: Scope) -> Element {
    let raw = match cx.generation() % 2 {
        0 => "123.123",
        1 => "123.123.123",
        _ => unreachable!(),
    };

    let value = raw.parse::<f32>().unwrap_or(123.123);

    cx.render(rsx! {
        div { "hello {value}" }
    })
}

#[test]
fn bubbles_error() {
    let mut dom = VirtualDom::new(app);

    {
        let _edits = dom.rebuild().santize();
    }

    dom.mark_dirty(ScopeId::ROOT);

    _ = dom.render_immediate();
}
