use dioxus::prelude::*;
use dioxus_core::{ScopeId, consume_context_from_scope, current_scope_id, generation};
use dioxus_renderer_oracle::RendererOracle;
use std::cell::Cell;

#[test]
fn state_shares() {
    thread_local! {
        static CHILD_2_SCOPE: Cell<Option<ScopeId>> = const { Cell::new(None) };
    }

    fn app() -> Element {
        provide_context(generation() as i32);

        rsx!(child_1 {})
    }

    fn child_1() -> Element {
        rsx!(child_2 {})
    }

    fn child_2() -> Element {
        CHILD_2_SCOPE.with(|scope| scope.set(Some(current_scope_id())));
        let value = consume_context::<i32>();
        rsx!("Value is {value}")
    }

    fn expected_0() -> Element {
        rsx!("Value is 0")
    }

    fn expected_2() -> Element {
        rsx!("Value is 2")
    }

    fn expected_3() -> Element {
        rsx!("Value is 3")
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    CHILD_2_SCOPE.with(|scope| scope.set(None));
    oracle.rebuild(&mut dom);
    let child_2_scope =
        CHILD_2_SCOPE.with(|scope| scope.get().expect("child_2 should have rendered"));
    oracle.assert_matches(expected_0);

    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);
    dom.in_runtime(|| {
        assert_eq!(consume_context_from_scope::<i32>(ScopeId::APP).unwrap(), 1);
    });

    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);
    dom.in_runtime(|| {
        assert_eq!(consume_context_from_scope::<i32>(ScopeId::APP).unwrap(), 2);
    });

    dom.mark_dirty(child_2_scope);
    let summary = oracle.render(&mut dom);
    oracle.assert_matches(expected_2);
    assert_eq!(summary.set_texts, 1);

    dom.mark_dirty(ScopeId::APP);
    dom.mark_dirty(child_2_scope);
    let summary = oracle.render(&mut dom);
    oracle.assert_matches(expected_3);
    assert_eq!(summary.set_texts, 1);
}
