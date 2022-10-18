#![allow(unused, non_upper_case_globals, non_snake_case)]

//! Prove that the dom works normally through virtualdom methods.
//!
//! This methods all use "rebuild" which completely bypasses the scheduler.
//! Hard rebuilds don't consume any events from the event queue.

use dioxus::prelude::*;

use dioxus_core::{DomEdit::*, ScopeId};

const IS_LOGGING_ENABLED: bool = false;

fn new_dom<P: 'static + Send>(app: Component<P>, props: P) -> VirtualDom {
    VirtualDom::new_with_props(app, props)
}

/// This test ensures that if a component aborts early, it is replaced with a placeholder.
/// In debug, this should also toss a warning.
#[test]
fn test_early_abort() {
    const app: Component = |cx| {
        let val = cx.use_hook(|| 0);

        *val += 1;

        if *val == 2 {
            return None;
        }

        render!(div { "Hello, world!" })
    };

    let mut dom = new_dom(app, ());

    let edits = dom.rebuild();
    assert_eq!(
        edits.edits,
        [
            // create template
            CreateElement { root: Some(1), tag: "template", children: 1 },
            CreateElement { root: None, tag: "div", children: 1 },
            CreateTextNode { root: None, text: "Hello, world!" },
            // clone template
            CloneNodeChildren { id: Some(1), new_ids: vec![2] },
            AppendChildren { root: Some(0), children: vec![2] }
        ]
    );

    let edits = dom.hard_diff(ScopeId(0));
    assert_eq!(
        edits.edits,
        [
            CreatePlaceholder { root: Some(3) },
            ReplaceWith { root: Some(2), nodes: vec![3] }
        ]
    );

    let edits = dom.hard_diff(ScopeId(0));
    assert_eq!(
        edits.edits,
        [
            CloneNodeChildren { id: Some(1), new_ids: vec![2] },
            ReplaceWith { root: Some(3), nodes: vec![2] }
        ]
    );
}
