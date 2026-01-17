use std::collections::HashSet;

use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::prelude::*;
use dioxus_core::{generation, Mutation};
use pretty_assertions::assert_eq;

#[test]
fn list_creates_one_by_one() {
    let mut dom = VirtualDom::new(|| {
        let gen = generation();

        rsx! {
            div {
                for i in 0..gen {
                    div { "{i}" }
                }
            }
        }
    });

    // load the div and then assign the empty fragment as a placeholder
    assert_eq!(
        dom.rebuild_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(1,) },
            CreatePlaceholder { id: ElementId(2,) },
            ReplacePlaceholder { path: &[0], m: 1 },
            AppendChildren { id: ElementId(0), m: 1 },
        ]
    );

    // Rendering the first item should replace the placeholder with an element
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(3,) },
            CreateTextNode { value: "0".to_string(), id: ElementId(4,) },
            ReplacePlaceholder { path: &[0], m: 1 },
            ReplaceWith { id: ElementId(2,), m: 1 },
        ]
    );

    // Rendering the next item should insert after the previous
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(2,) },
            CreateTextNode { value: "1".to_string(), id: ElementId(5,) },
            ReplacePlaceholder { path: &[0], m: 1 },
            InsertAfter { id: ElementId(3,), m: 1 },
        ]
    );

    // ... and again!
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(6,) },
            CreateTextNode { value: "2".to_string(), id: ElementId(7,) },
            ReplacePlaceholder { path: &[0], m: 1 },
            InsertAfter { id: ElementId(2,), m: 1 },
        ]
    );

    // once more
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(8,) },
            CreateTextNode { value: "3".to_string(), id: ElementId(9,) },
            ReplacePlaceholder { path: &[0], m: 1 },
            InsertAfter { id: ElementId(6,), m: 1 },
        ]
    );
}

#[test]
fn removes_one_by_one() {
    let mut dom = VirtualDom::new(|| {
        let gen = 3 - generation() % 4;

        rsx! {
            div {
                for i in 0..gen {
                    div { "{i}" }
                }
            }
        }
    });

    // load the div and then assign the empty fragment as a placeholder
    assert_eq!(
        dom.rebuild_to_vec().edits,
        [
            // The container
            LoadTemplate { index: 0, id: ElementId(1) },
            // each list item
            LoadTemplate { index: 0, id: ElementId(2) },
            CreateTextNode { value: "0".to_string(), id: ElementId(3) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 0, id: ElementId(4) },
            CreateTextNode { value: "1".to_string(), id: ElementId(5) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 0, id: ElementId(6) },
            CreateTextNode { value: "2".to_string(), id: ElementId(7) },
            ReplacePlaceholder { path: &[0], m: 1 },
            // replace the placeholder in the template with the 3 templates on the stack
            ReplacePlaceholder { m: 3, path: &[0] },
            // Mount the div
            AppendChildren { id: ElementId(0), m: 1 }
        ]
    );

    // Remove div(3)
    // Rendering the first item should replace the placeholder with an element
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [Remove { id: ElementId(6) }]
    );

    // Remove div(2)
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [Remove { id: ElementId(4) }]
    );

    // Remove div(1) and replace with a placeholder
    // todo: this should just be a remove with no placeholder
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            CreatePlaceholder { id: ElementId(4) },
            ReplaceWith { id: ElementId(2), m: 1 }
        ]
    );

    // load the 3 and replace the placeholder
    // todo: this should actually be append to, but replace placeholder is fine for now
    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(2) },
            CreateTextNode { value: "0".to_string(), id: ElementId(3) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 0, id: ElementId(5) },
            CreateTextNode { value: "1".to_string(), id: ElementId(6) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 0, id: ElementId(7) },
            CreateTextNode { value: "2".to_string(), id: ElementId(8) },
            ReplacePlaceholder { path: &[0], m: 1 },
            ReplaceWith { id: ElementId(4), m: 3 }
        ]
    );
}

#[test]
fn list_shrink_multiroot() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            div {
                for i in 0..generation() {
                    div { "{i}" }
                    div { "{i}" }
                }
            }
        }
    });

    assert_eq!(
        dom.rebuild_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(1,) },
            CreatePlaceholder { id: ElementId(2,) },
            ReplacePlaceholder { path: &[0,], m: 1 },
            AppendChildren { id: ElementId(0), m: 1 }
        ]
    );

    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(3) },
            CreateTextNode { value: "0".to_string(), id: ElementId(4) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 1, id: ElementId(5) },
            CreateTextNode { value: "0".to_string(), id: ElementId(6) },
            ReplacePlaceholder { path: &[0], m: 1 },
            ReplaceWith { id: ElementId(2), m: 2 }
        ]
    );

    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(2) },
            CreateTextNode { value: "1".to_string(), id: ElementId(7) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 1, id: ElementId(8) },
            CreateTextNode { value: "1".to_string(), id: ElementId(9) },
            ReplacePlaceholder { path: &[0], m: 1 },
            InsertAfter { id: ElementId(5), m: 2 }
        ]
    );

    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(10) },
            CreateTextNode { value: "2".to_string(), id: ElementId(11) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 1, id: ElementId(12) },
            CreateTextNode { value: "2".to_string(), id: ElementId(13) },
            ReplacePlaceholder { path: &[0], m: 1 },
            InsertAfter { id: ElementId(8), m: 2 }
        ]
    );
}

#[test]
fn removes_one_by_one_multiroot() {
    let mut dom = VirtualDom::new(|| {
        let gen = 3 - generation() % 4;

        rsx! {
            div {
                {(0..gen).map(|i| rsx! {
                    div { "{i}" }
                    div { "{i}" }
                })}
            }
        }
    });

    // load the div and then assign the empty fragment as a placeholder
    assert_eq!(
        dom.rebuild_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(1) },
            //
            LoadTemplate { index: 0, id: ElementId(2) },
            CreateTextNode { value: "0".to_string(), id: ElementId(3) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 1, id: ElementId(4) },
            CreateTextNode { value: "0".to_string(), id: ElementId(5) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 0, id: ElementId(6) },
            CreateTextNode { value: "1".to_string(), id: ElementId(7) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 1, id: ElementId(8) },
            CreateTextNode { value: "1".to_string(), id: ElementId(9) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 0, id: ElementId(10) },
            CreateTextNode { value: "2".to_string(), id: ElementId(11) },
            ReplacePlaceholder { path: &[0], m: 1 },
            LoadTemplate { index: 1, id: ElementId(12) },
            CreateTextNode { value: "2".to_string(), id: ElementId(13) },
            ReplacePlaceholder { path: &[0], m: 1 },
            ReplacePlaceholder { path: &[0], m: 6 },
            AppendChildren { id: ElementId(0), m: 1 }
        ]
    );

    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [Remove { id: ElementId(10) }, Remove { id: ElementId(12) }]
    );

    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [Remove { id: ElementId(6) }, Remove { id: ElementId(8) }]
    );

    dom.mark_dirty(ScopeId::APP);
    assert_eq!(
        dom.render_immediate_to_vec().edits,
        [
            CreatePlaceholder { id: ElementId(8) },
            Remove { id: ElementId(2) },
            ReplaceWith { id: ElementId(4), m: 1 }
        ]
    );
}

#[test]
fn two_equal_fragments_are_equal_static() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            for _ in 0..5 {
                div { "hello" }
            }
        }
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);
    assert!(dom.render_immediate_to_vec().edits.is_empty());
}

#[test]
fn two_equal_fragments_are_equal() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            for i in 0..5 {
                div { "hello {i}" }
            }
        }
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);
    assert!(dom.render_immediate_to_vec().edits.is_empty());
}

#[test]
fn remove_many() {
    let mut dom = VirtualDom::new(|| {
        let num = match generation() % 3 {
            0 => 0,
            1 => 1,
            2 => 5,
            _ => unreachable!(),
        };

        rsx! {
            for i in 0..num {
                div { "hello {i}" }
            }
        }
    });

    // len = 0
    {
        let edits = dom.rebuild_to_vec();
        assert_eq!(
            edits.edits,
            [
                CreatePlaceholder { id: ElementId(1,) },
                AppendChildren { id: ElementId(0), m: 1 },
            ]
        );
    }

    // len = 1
    {
        dom.mark_dirty(ScopeId::APP);
        let edits = dom.render_immediate_to_vec();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { index: 0, id: ElementId(2,) },
                CreateTextNode { value: "hello 0".to_string(), id: ElementId(3,) },
                ReplacePlaceholder { path: &[0,], m: 1 },
                ReplaceWith { id: ElementId(1,), m: 1 },
            ]
        );
    }

    // len = 5
    {
        dom.mark_dirty(ScopeId::APP);
        let edits = dom.render_immediate_to_vec();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { index: 0, id: ElementId(1,) },
                CreateTextNode { value: "hello 1".to_string(), id: ElementId(4,) },
                ReplacePlaceholder { path: &[0,], m: 1 },
                LoadTemplate { index: 0, id: ElementId(5,) },
                CreateTextNode { value: "hello 2".to_string(), id: ElementId(6,) },
                ReplacePlaceholder { path: &[0,], m: 1 },
                LoadTemplate { index: 0, id: ElementId(7,) },
                CreateTextNode { value: "hello 3".to_string(), id: ElementId(8,) },
                ReplacePlaceholder { path: &[0,], m: 1 },
                LoadTemplate { index: 0, id: ElementId(9,) },
                CreateTextNode { value: "hello 4".to_string(), id: ElementId(10,) },
                ReplacePlaceholder { path: &[0,], m: 1 },
                InsertAfter { id: ElementId(2,), m: 4 },
            ]
        );
    }

    // len = 0
    {
        dom.mark_dirty(ScopeId::APP);
        let edits = dom.render_immediate_to_vec();
        assert_eq!(edits.edits[0], CreatePlaceholder { id: ElementId(11,) });
        let removed = edits.edits[1..5]
            .iter()
            .map(|edit| match edit {
                Mutation::Remove { id } => *id,
                _ => panic!("Expected remove"),
            })
            .collect::<HashSet<_>>();
        assert_eq!(
            removed,
            [ElementId(7), ElementId(5), ElementId(2), ElementId(1)]
                .into_iter()
                .collect::<HashSet<_>>()
        );
        assert_eq!(edits.edits[5..], [ReplaceWith { id: ElementId(9,), m: 1 },]);
    }

    // len = 1
    {
        dom.mark_dirty(ScopeId::APP);
        let edits = dom.render_immediate_to_vec();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { index: 0, id: ElementId(9,) },
                CreateTextNode { value: "hello 0".to_string(), id: ElementId(10,) },
                ReplacePlaceholder { path: &[0,], m: 1 },
                ReplaceWith { id: ElementId(11,), m: 1 },
            ]
        )
    }
}

#[test]
fn replace_and_add_items() {
    let mut dom = VirtualDom::new(|| {
        let items = (0..generation()).map(|_| {
            if generation() % 2 == 0 {
                VNode::empty()
            } else {
                rsx! {
                    li {
                        "Fizz"
                    }
                }
            }
        });

        rsx! {
            ul {
                {items}
            }
        }
    });

    // The list starts empty with a placeholder
    {
        let edits = dom.rebuild_to_vec();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { index: 0, id: ElementId(1,) },
                CreatePlaceholder { id: ElementId(2,) },
                ReplacePlaceholder { path: &[0], m: 1 },
                AppendChildren { id: ElementId(0), m: 1 },
            ]
        );
    }

    // Rerendering adds an a static template
    {
        dom.mark_dirty(ScopeId::APP);
        let edits = dom.render_immediate_to_vec();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { index: 0, id: ElementId(3,) },
                ReplaceWith { id: ElementId(2,), m: 1 },
            ]
        );
    }

    // Rerendering replaces the old node with a placeholder and adds a new placeholder
    {
        dom.mark_dirty(ScopeId::APP);
        let edits = dom.render_immediate_to_vec();
        assert_eq!(
            edits.edits,
            [
                CreatePlaceholder { id: ElementId(2,) },
                InsertAfter { id: ElementId(3,), m: 1 },
                CreatePlaceholder { id: ElementId(4,) },
                ReplaceWith { id: ElementId(3,), m: 1 },
            ]
        );
    }

    // Rerendering replaces both placeholders with the static nodes and add a new static node
    {
        dom.mark_dirty(ScopeId::APP);
        let edits = dom.render_immediate_to_vec();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { index: 0, id: ElementId(3,) },
                InsertAfter { id: ElementId(2,), m: 1 },
                LoadTemplate { index: 0, id: ElementId(5,) },
                ReplaceWith { id: ElementId(4,), m: 1 },
                LoadTemplate { index: 0, id: ElementId(4,) },
                ReplaceWith { id: ElementId(2,), m: 1 },
            ]
        );
    }
}

// Simplified regression test for https://github.com/DioxusLabs/dioxus/issues/4924
#[test]
fn nested_unkeyed_lists() {
    let mut dom = VirtualDom::new(|| {
        let content = if generation() % 2 == 0 {
            vec!["5\n6"]
        } else {
            vec!["1\n2", "3\n4"]
        };

        rsx! {
            for one in &content {
                for line in one.lines() {
                    p { "{line}" }
                }
            }
        }
    });

    // The list starts with one placeholder
    {
        let edits = dom.rebuild_to_vec();
        assert_eq!(
            edits.edits,
            [
                // load the p tag template
                LoadTemplate { index: 0, id: ElementId(1) },
                // Create the first text node
                CreateTextNode { value: "5".into(), id: ElementId(2) },
                // Replace the placeholder inside the p tag with the text node
                ReplacePlaceholder { path: &[0], m: 1 },
                // load the p tag template
                LoadTemplate { index: 0, id: ElementId(3) },
                // Create the second text node
                CreateTextNode { value: "6".into(), id: ElementId(4) },
                // Replace the placeholder inside the p tag with the text node
                ReplacePlaceholder { path: &[0], m: 1 },
                // Add the text nodes to the root node
                AppendChildren { id: ElementId(0), m: 2 }
            ]
        );
    }

    // DOM state:
    // <pre> # Id 1 for if statement
    // <p> # Id 2
    //    "5" # Id 3
    // <p> # Id 4
    //    "6" # Id 5
    //
    // The diffing engine should add two new elements to the end and modify the first two elements in place
    {
        dom.mark_dirty(ScopeId::APP);
        let edits = dom.render_immediate_to_vec();
        assert_eq!(
            edits.edits,
            [
                // load the p tag template
                LoadTemplate { index: 0, id: ElementId(5) },
                // Create the third text node
                CreateTextNode { value: "3".into(), id: ElementId(6) },
                // Replace the placeholder inside the p tag with the text node
                ReplacePlaceholder { path: &[0], m: 1 },
                // load the p tag template
                LoadTemplate { index: 0, id: ElementId(7) },
                // Create the fourth text node
                CreateTextNode { value: "4".into(), id: ElementId(8) },
                // Replace the placeholder inside the p tag with the text node
                ReplacePlaceholder { path: &[0], m: 1 },
                // Insert the text nodes after the second p tag
                InsertAfter { id: ElementId(3), m: 2 },
                // Set the first text node to "1"
                SetText { value: "1".into(), id: ElementId(2) },
                // Set the second text node to "2"
                SetText { value: "2".into(), id: ElementId(4) }
            ]
        );
    }
}
