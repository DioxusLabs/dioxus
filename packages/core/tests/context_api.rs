use dioxus::core::{ElementId, Mutation::*};
use dioxus::prelude::*;

#[test]
fn state_shares() {
    fn app(cx: Scope) -> Element {
        cx.provide_context(cx.generation() as i32);

        cx.render(rsx!(child_1 {}))
    }

    fn child_1(cx: Scope) -> Element {
        cx.render(rsx!(child_2 {}))
    }

    fn child_2(cx: Scope) -> Element {
        let value = cx.consume_context::<i32>().unwrap();
        cx.render(rsx!("Value is {value}"))
    }

    let mut dom = VirtualDom::new(app);
    assert_eq!(
        dom.rebuild().santize().edits,
        [
            CreateTextNode { value: "Value is 0", id: ElementId(1,) },
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate();
    assert_eq!(dom.base_scope().consume_context::<i32>().unwrap(), 1);

    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate();
    assert_eq!(dom.base_scope().consume_context::<i32>().unwrap(), 2);

    dom.mark_dirty(ScopeId(2));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [SetText { value: "Value is 2", id: ElementId(1,) },]
    );

    dom.mark_dirty(ScopeId::ROOT);
    dom.mark_dirty(ScopeId(2));
    let edits = dom.render_immediate();
    assert_eq!(
        edits.santize().edits,
        [SetText { value: "Value is 3", id: ElementId(1,) },]
    );
}
