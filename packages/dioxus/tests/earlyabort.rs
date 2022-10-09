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
            CreateTemplate { id: 0 },
            CreateElementTemplate {
                root: 4503599627370495,
                tag: "div",
                locally_static: true,
                fully_static: true
            },
            CreateTextNodeTemplate {
                root: 4503599627370496,
                text: "Hello, world!",
                locally_static: true
            },
            AppendChildren { many: 1 },
            FinishTemplate { len: 1 },
            CreateTemplateRef { id: 1, template_id: 0 },
            AppendChildren { many: 1 }
        ]
    );

    let edits = dom.hard_diff(ScopeId(0));
    assert_eq!(
        edits.edits,
        [CreatePlaceholder { root: 2 }, ReplaceWith { root: 1, m: 1 },],
    );

    let edits = dom.hard_diff(ScopeId(0));
    assert_eq!(
        edits.edits,
        [
            CreateTemplateRef { id: 1, template_id: 0 }, // gets reused
            ReplaceWith { root: 2, m: 1 }
        ]
    );
}
