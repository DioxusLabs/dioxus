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
    vdom.rebuild();

    vdom.mark_dirty_scope(ScopeId(0));
    assert_eq!(
        vdom.render_immediate().edits,
        [SetText { value: "hello 1", id: ElementId(2) }]
    );

    vdom.mark_dirty_scope(ScopeId(0));
    assert_eq!(
        vdom.render_immediate().edits,
        [SetText { value: "hello 2", id: ElementId(2) }]
    );

    vdom.mark_dirty_scope(ScopeId(0));
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
    vdom.rebuild();

    vdom.mark_dirty_scope(ScopeId(0));
    dbg!(vdom.render_immediate());

    vdom.mark_dirty_scope(ScopeId(0));
    dbg!(vdom.render_immediate());

    vdom.mark_dirty_scope(ScopeId(0));
    dbg!(vdom.render_immediate());

    vdom.mark_dirty_scope(ScopeId(0));
    dbg!(vdom.render_immediate());
}
