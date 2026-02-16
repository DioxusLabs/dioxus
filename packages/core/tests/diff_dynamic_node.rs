use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::prelude::*;
use dioxus_core::generation;
use pretty_assertions::assert_eq;

#[test]
fn toggle_option_text() {
    let mut dom = VirtualDom::new(|| {
        let generation_count = generation();
        let text = if generation_count % 2 != 0 {
            Some("hello")
        } else {
            None
        };
        println!("{:?}", text);

        rsx! {
            div {
                {text}
            }
        }
    });

    // load the div and then assign the None as a placeholder
    assert_eq!(
        dom.rebuild_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(1,) },
            CreatePlaceholder { id: ElementId(2,) },
            ReplacePlaceholder { path: &[0], m: 1 },
            AppendChildren { id: ElementId(0), m: 1 },
        ]
    );

    // Rendering again should replace the placeholder with an text node
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            CreateTextNode { value: "hello".to_string(), id: ElementId(3,) },
            ReplaceWith { id: ElementId(2,), m: 1 },
        ]
    );

    // Rendering again should replace the placeholder with an text node
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            CreatePlaceholder { id: ElementId(2,) },
            ReplaceWith { id: ElementId(3,), m: 1 },
        ]
    );
}

// Regression test for https://github.com/DioxusLabs/dioxus/issues/2815
#[test]
fn toggle_template() {
    fn app() -> Element {
        rsx!(
            Comp {
                if true {
                    "{true}"
                }
            }
        )
    }

    #[component]
    fn Comp(children: Element) -> Element {
        let show = generation() % 2 == 0;

        rsx! {
            if show {
                {children}
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    // Rendering again should replace the placeholder with an text node
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            CreatePlaceholder { id: ElementId(2) },
            ReplaceWith { id: ElementId(1), m: 1 },
        ]
    );

    dom.mark_dirty(ScopeId(ScopeId::APP.0 + 1));
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            CreateTextNode { value: "true".to_string(), id: ElementId(1) },
            ReplaceWith { id: ElementId(2), m: 1 },
        ]
    );

    dom.mark_dirty(ScopeId(ScopeId::APP.0 + 1));
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            CreatePlaceholder { id: ElementId(2) },
            ReplaceWith { id: ElementId(1), m: 1 },
        ]
    );

    dom.mark_dirty(ScopeId(ScopeId::APP.0 + 1));
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            CreateTextNode { value: "true".to_string(), id: ElementId(1) },
            ReplaceWith { id: ElementId(2), m: 1 },
        ]
    );
}
