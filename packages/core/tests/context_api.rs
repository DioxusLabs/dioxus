use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::prelude::*;

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
        dom.rebuild_to_vec().santize().edits,
        [
            CreateTextNode { value: "Value is 0".to_string(), id: ElementId(1,) },
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate_to_vec();
    dom.in_runtime(|| {
        assert_eq!(ScopeId::ROOT.consume_context::<i32>().unwrap(), 1);
    });

    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate_to_vec();
    dom.in_runtime(|| {
        assert_eq!(ScopeId::ROOT.consume_context::<i32>().unwrap(), 2);
    });

    dom.mark_dirty(ScopeId(2));
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [SetText { value: "Value is 2".to_string(), id: ElementId(1,) },]
    );

    dom.mark_dirty(ScopeId::ROOT);
    dom.mark_dirty(ScopeId(2));
    let edits = dom.render_immediate_to_vec();
    assert_eq!(
        edits.santize().edits,
        [SetText { value: "Value is 3".to_string(), id: ElementId(1,) },]
    );
}
