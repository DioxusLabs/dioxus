use dioxus::core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::ElementId;

#[test]
fn text_diff() {
    fn app(cx: Scope) -> Element {
        let gen = cx.generation();
        cx.render(rsx!( h1 { "hello {gen}" } ))
    }

    let mut vdom = VirtualDom::new(app);
    _ = vdom.rebuild();

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().edits,
        [SetText { value: "hello 1", id: ElementId(2) }]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().edits,
        [SetText { value: "hello 2", id: ElementId(2) }]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().edits,
        [SetText { value: "hello 3", id: ElementId(2) }]
    );
}

#[test]
fn element_swap() {
    fn app(cx: Scope) -> Element {
        let gen = cx.generation();

        match gen % 2 {
            0 => cx.render(rsx!( h1 { "hello 1" } )),
            1 => cx.render(rsx!( h2 { "hello 2" } )),
            _ => unreachable!(),
        }
    }

    let mut vdom = VirtualDom::new(app);
    _ = vdom.rebuild();

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
            ReplaceWith { id: ElementId(1,), m: 1 },
        ]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            ReplaceWith { id: ElementId(2,), m: 1 },
        ]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
            ReplaceWith { id: ElementId(1,), m: 1 },
        ]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            ReplaceWith { id: ElementId(2,), m: 1 },
        ]
    );
}
