use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::prelude::*;
use dioxus_core::{consume_context_from_scope, generation};

#[test]
fn state_shares() {
    fn app() -> Element {
        provide_context(generation() as i32);

        rsx!(child_1 {})
    }

    fn child_1() -> Element {
        rsx!(child_2 {})
    }

    fn child_2() -> Element {
        let value = consume_context::<i32>();
        rsx!("Value is {value}")
    }

    let mut dom = VirtualDom::new(app);
    assert_eq!(
        dom.rebuild_to_vec().edits,
        [
            CreateTextNode { value: "Value is 0".to_string(), id: ElementId(1,) },
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    );

    dom.mark_dirty(ScopeId::APP);
    _ = dom.render_immediate_to_vec();
    dom.in_runtime(|| {
        assert_eq!(consume_context_from_scope::<i32>(ScopeId::APP).unwrap(), 1);
    });

    dom.mark_dirty(ScopeId::APP);
    _ = dom.render_immediate_to_vec();
    dom.in_runtime(|| {
        assert_eq!(consume_context_from_scope::<i32>(ScopeId::APP).unwrap(), 2);
    });

    dom.mark_dirty(ScopeId(ScopeId::APP.0 + 2));
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [SetText { value: "Value is 2".to_string(), id: ElementId(1,) },]
    );

    dom.mark_dirty(ScopeId::APP);
    dom.mark_dirty(ScopeId(ScopeId::APP.0 + 2));
    let edits = dom.render_immediate_to_vec();
    assert_eq!(
        edits.edits,
        [SetText { value: "Value is 3".to_string(), id: ElementId(1,) },]
    );
}
