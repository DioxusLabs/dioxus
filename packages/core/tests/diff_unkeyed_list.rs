use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::prelude::*;
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
        dom.rebuild_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            AssignId { path: &[0], id: ElementId(2,) },
            AppendChildren { id: ElementId(0), m: 1 },
        ]
    );

    // Rendering the first item should replace the placeholder with an element
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(3,) },
            HydrateText { path: &[0], value: "0".to_string(), id: ElementId(4,) },
            ReplaceWith { id: ElementId(2,), m: 1 },
        ]
    );

    // Rendering the next item should insert after the previous
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
            HydrateText { path: &[0], value: "1".to_string(), id: ElementId(5,) },
            InsertAfter { id: ElementId(3,), m: 1 },
        ]
    );

    // ... and again!
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(6,) },
            HydrateText { path: &[0], value: "2".to_string(), id: ElementId(7,) },
            InsertAfter { id: ElementId(2,), m: 1 },
        ]
    );

    // once more
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(8,) },
            HydrateText { path: &[0], value: "3".to_string(), id: ElementId(9,) },
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
        dom.rebuild_to_vec().santize().edits,
        [
            // The container
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            // each list item
            LoadTemplate { name: "template", index: 0, id: ElementId(2) },
            HydrateText { path: &[0], value: "0".to_string(), id: ElementId(3) },
            LoadTemplate { name: "template", index: 0, id: ElementId(4) },
            HydrateText { path: &[0], value: "1".to_string(), id: ElementId(5) },
            LoadTemplate { name: "template", index: 0, id: ElementId(6) },
            HydrateText { path: &[0], value: "2".to_string(), id: ElementId(7) },
            // replace the placeholder in the template with the 3 templates on the stack
            ReplacePlaceholder { m: 3, path: &[0] },
            // Mount the div
            AppendChildren { id: ElementId(0), m: 1 }
        ]
    );

    // Remove div(3)
    // Rendering the first item should replace the placeholder with an element
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [Remove { id: ElementId(6) }]
    );

    // Remove div(2)
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [Remove { id: ElementId(4) }]
    );

    // Remove div(1) and replace with a placeholder
    // todo: this should just be a remove with no placeholder
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            CreatePlaceholder { id: ElementId(4) },
            ReplaceWith { id: ElementId(2), m: 1 }
        ]
    );

    // load the 3 and replace the placeholder
    // todo: this should actually be append to, but replace placeholder is fine for now
    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2) },
            HydrateText { path: &[0], value: "0".to_string(), id: ElementId(3) },
            LoadTemplate { name: "template", index: 0, id: ElementId(5) },
            HydrateText { path: &[0], value: "1".to_string(), id: ElementId(6) },
            LoadTemplate { name: "template", index: 0, id: ElementId(7) },
            HydrateText { path: &[0], value: "2".to_string(), id: ElementId(8) },
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
        dom.rebuild_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            AssignId { path: &[0,], id: ElementId(2,) },
            AppendChildren { id: ElementId(0), m: 1 }
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(3) },
            HydrateText { path: &[0], value: "0".to_string(), id: ElementId(4) },
            LoadTemplate { name: "template", index: 1, id: ElementId(5) },
            HydrateText { path: &[0], value: "0".to_string(), id: ElementId(6) },
            ReplaceWith { id: ElementId(2), m: 2 }
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2) },
            HydrateText { path: &[0], value: "1".to_string(), id: ElementId(7) },
            LoadTemplate { name: "template", index: 1, id: ElementId(8) },
            HydrateText { path: &[0], value: "1".to_string(), id: ElementId(9) },
            InsertAfter { id: ElementId(5), m: 2 }
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(10) },
            HydrateText { path: &[0], value: "2".to_string(), id: ElementId(11) },
            LoadTemplate { name: "template", index: 1, id: ElementId(12) },
            HydrateText { path: &[0], value: "2".to_string(), id: ElementId(13) },
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
        dom.rebuild_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            //
            LoadTemplate { name: "template", index: 0, id: ElementId(2) },
            HydrateText { path: &[0], value: "0".to_string(), id: ElementId(3) },
            LoadTemplate { name: "template", index: 1, id: ElementId(4) },
            HydrateText { path: &[0], value: "0".to_string(), id: ElementId(5) },
            //
            LoadTemplate { name: "template", index: 0, id: ElementId(6) },
            HydrateText { path: &[0], value: "1".to_string(), id: ElementId(7) },
            LoadTemplate { name: "template", index: 1, id: ElementId(8) },
            HydrateText { path: &[0], value: "1".to_string(), id: ElementId(9) },
            //
            LoadTemplate { name: "template", index: 0, id: ElementId(10) },
            HydrateText { path: &[0], value: "2".to_string(), id: ElementId(11) },
            LoadTemplate { name: "template", index: 1, id: ElementId(12) },
            HydrateText { path: &[0], value: "2".to_string(), id: ElementId(13) },
            //
            ReplacePlaceholder { path: &[0], m: 6 },
            //
            AppendChildren { id: ElementId(0), m: 1 }
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [Remove { id: ElementId(10) }, Remove { id: ElementId(12) }]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
        [Remove { id: ElementId(6) }, Remove { id: ElementId(8) }]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate_to_vec().santize().edits,
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

    {
        let edits = dom.rebuild_to_vec().santize();
        assert!(edits.templates.is_empty());
        assert_eq!(
            edits.edits,
            [
                CreatePlaceholder { id: ElementId(1,) },
                AppendChildren { id: ElementId(0), m: 1 },
            ]
        );
    }

    {
        dom.mark_dirty(ScopeId::ROOT);
        let edits = dom.render_immediate_to_vec().santize();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
                HydrateText { path: &[0,], value: "hello 0".to_string(), id: ElementId(3,) },
                ReplaceWith { id: ElementId(1,), m: 1 },
            ]
        );
    }

    {
        dom.mark_dirty(ScopeId::ROOT);
        let edits = dom.render_immediate_to_vec().santize();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
                HydrateText { path: &[0,], value: "hello 1".to_string(), id: ElementId(4,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(5,) },
                HydrateText { path: &[0,], value: "hello 2".to_string(), id: ElementId(6,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(7,) },
                HydrateText { path: &[0,], value: "hello 3".to_string(), id: ElementId(8,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(9,) },
                HydrateText { path: &[0,], value: "hello 4".to_string(), id: ElementId(10,) },
                InsertAfter { id: ElementId(2,), m: 4 },
            ]
        );
    }

    {
        dom.mark_dirty(ScopeId::ROOT);
        let edits = dom.render_immediate_to_vec().santize();
        assert_eq!(
            edits.edits,
            [
                CreatePlaceholder { id: ElementId(11,) },
                Remove { id: ElementId(9,) },
                Remove { id: ElementId(7,) },
                Remove { id: ElementId(5,) },
                Remove { id: ElementId(1,) },
                ReplaceWith { id: ElementId(2,), m: 1 },
            ]
        );
    }

    {
        dom.mark_dirty(ScopeId::ROOT);
        let edits = dom.render_immediate_to_vec().santize();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
                HydrateText { path: &[0,], value: "hello 0".to_string(), id: ElementId(3,) },
                ReplaceWith { id: ElementId(11,), m: 1 },
            ]
        )
    }
}
