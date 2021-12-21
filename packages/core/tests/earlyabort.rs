#![allow(unused, non_upper_case_globals, non_snake_case)]

//! Prove that the dom works normally through virtualdom methods.
//!
//! This methods all use "rebuild" which completely bypasses the scheduler.
//! Hard rebuilds don't consume any events from the event queue.

use dioxus::{prelude::*, DomEdit, ScopeId};
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

mod test_logging;
use DomEdit::*;

const IS_LOGGING_ENABLED: bool = false;

fn new_dom<P: 'static + Send>(app: Component<P>, props: P) -> VirtualDom {
    test_logging::set_up_logging(IS_LOGGING_ENABLED);
    VirtualDom::new_with_props(app, props)
}

/// This test ensures that if a component aborts early, it is replaced with a placeholder.
/// In debug, this should also toss a warning.
#[test]
fn test_early_abort() {
    const app: Component<()> = |cx| {
        let val = cx.use_hook(|_| 0, |f| f);

        *val += 1;

        if *val == 2 {
            return None;
        }

        rsx!(cx, div { "Hello, world!" })
    };

    let mut dom = new_dom(app, ());

    let edits = dom.rebuild();
    assert_eq!(
        edits.edits,
        [
            CreateElement {
                tag: "div",
                root: 1,
            },
            CreateTextNode {
                text: "Hello, world!",
                root: 2,
            },
            AppendChildren { many: 1 },
            AppendChildren { many: 1 },
        ]
    );

    let edits = dom.hard_diff(ScopeId(0));
    assert_eq!(
        edits.edits,
        [CreatePlaceholder { root: 3 }, ReplaceWith { root: 1, m: 1 },],
    );

    let edits = dom.hard_diff(ScopeId(0));
    assert_eq!(
        edits.edits,
        [
            CreateElement {
                tag: "div",
                root: 2,
            },
            CreateTextNode {
                text: "Hello, world!",
                root: 4,
            },
            AppendChildren { many: 1 },
            ReplaceWith { root: 3, m: 1 },
        ]
    );
}
