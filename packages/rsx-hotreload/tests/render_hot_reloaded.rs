//! Render an `rsx!` site through a hot-reloaded template produced by the real diff engine, the
//! same way the CLI does, to check the debug-build hot-reload path end to end.
#![cfg(debug_assertions)]

use dioxus::prelude::*;
use dioxus_core::internal::HotReloadedTemplate;
use dioxus_html::HtmlCtx;
use dioxus_rsx::CallBody;
use dioxus_rsx_hotreload::HotReloadResult;
use dioxus_signals::{GlobalKey, WritableExt, get_global_context};

// The `rsx!` invocation is kept at column 1 so its hot-reload key is known to the test.
const APP_RSX_LINE: u32 = line!() + 6;
#[rustfmt::skip]
#[allow(non_snake_case)]
fn App() -> Element {
    let count = 2;
    let label = "world";
rsx! {
    div { id: "root", class: "old-class",
        h1 { title: "{label}", "Hello {label}" }
        Greeting { name: "old-literal", times: 1 }
        Card { strong { "static child" } }
        Card { "text child" }
        p { hidden: count > 5, "count: {count}" }
        ul {
            for i in 0..count {
                li { key: "{i}", "item {i}" }
            }
        }
    }
}
}

const APP_RSX_OLD: &str = r#"
    div { id: "root", class: "old-class",
        h1 { title: "{label}", "Hello {label}" }
        Greeting { name: "old-literal", times: 1 }
        Card { strong { "static child" } }
        Card { "text child" }
        p { hidden: count > 5, "count: {count}" }
        ul {
            for i in 0..count {
                li { key: "{i}", "item {i}" }
            }
        }
    }
"#;

// Static text/attribute edits, a formatted dynamic attribute edit, a component literal edit, a
// dynamic node reorder, text and key edits inside a nested (`for` body) template, and an edit
// inside a fully static nested (component children) template, both an element one and a text-only
// one.
const APP_RSX_NEW: &str = r#"
    div { id: "root", class: "new-class",
        p { hidden: count > 5, "count: {count}" }
        h1 { title: "t-{label}", "Hi {label}!" }
        Greeting { name: "new-literal", times: 2 }
        Card { strong { "edited child" } }
        Card { "edited text" }
        ul {
            for i in 0..count {
                li { key: "k{i}", "row {i}" }
            }
        }
    }
"#;

#[component]
fn Greeting(name: String, times: i32) -> Element {
    rsx! {
        span { "{name}x{times}" }
    }
}

// A body with a dynamic node but no formatted text or component literals.
#[component]
fn Card(children: Element) -> Element {
    rsx! {
        section { {children} }
    }
}

fn hot_reload_app(dom: &VirtualDom) {
    let old: CallBody = syn::parse_str(APP_RSX_OLD).unwrap();
    let new: CallBody = syn::parse_str(APP_RSX_NEW).unwrap();
    let result = HotReloadResult::new::<HtmlCtx>(old.body(), new.body(), file!().to_string())
        .expect("the edit should be hot reloadable");

    let mut applied = 0;
    dom.in_runtime(|| {
        let ctx = get_global_context();
        for (index, template) in result.templates {
            if template.root_count() == 0 {
                continue;
            }
            let key = GlobalKey::File {
                file: file!(),
                line: APP_RSX_LINE,
                column: 1,
                index: index as u32,
            };
            // Like the CLI, ignore templates with no registered site (e.g. a component's empty
            // children body).
            if let Some(mut signal) = ctx.get_signal_with_key::<Option<HotReloadedTemplate>>(key) {
                signal.set(Some(template));
                applied += 1;
            }
        }
    });
    assert_eq!(
        applied, 4,
        "the root, both `Card` children and the `for` body templates should be hot reloaded"
    );
}

#[test]
fn renders_hot_reloaded_template() {
    let mut dom = VirtualDom::new(App);
    dom.rebuild_in_place();
    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<div id="root" class="old-class"><h1 title="world">Hello world</h1><span>old-literalx1</span><section><strong>static child</strong></section><section>text child</section><p>count: 2</p><ul><li>item 0</li><li>item 1</li></ul></div>"#
    );

    hot_reload_app(&dom);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<div id="root" class="new-class"><p>count: 2</p><h1 title="t-world">Hi world!</h1><span>new-literalx2</span><section><strong>edited child</strong></section><section>edited text</section><ul><li>row 0</li><li>row 1</li></ul></div>"#
    );

    // Re-rendering without a new template keeps the hot-reloaded output.
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<div id="root" class="new-class"><p>count: 2</p><h1 title="t-world">Hi world!</h1><span>new-literalx2</span><section><strong>edited child</strong></section><section>edited text</section><ul><li>row 0</li><li>row 1</li></ul></div>"#
    );
}
