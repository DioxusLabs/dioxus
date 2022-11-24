//! we should properly bubble up errors from components

use dioxus::prelude::*;

fn app(cx: Scope) -> Element {
    let raw = match cx.generation() % 2 {
        0 => "123.123",
        1 => "123.123.123",
        _ => unreachable!(),
    };

    let value = raw.parse::<f32>()?;

    cx.render(rsx! {
        div { "hello {value}" }
    })
}

#[test]
fn it_goes() {
    let mut dom = VirtualDom::new(app);

    let edits = dom.rebuild().santize();

    dbg!(edits);

    dom.mark_dirty_scope(ScopeId(0));

    dom.render_immediate();
}
