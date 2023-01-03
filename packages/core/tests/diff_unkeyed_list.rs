use dioxus::core::{ElementId, Mutation::*};
use dioxus::prelude::*;
use pretty_assertions::assert_eq;

#[test]
fn list_creates_one_by_one() {
    let mut dom = VirtualDom::new(|cx| {
        let gen = cx.generation();

        cx.render(rsx! {
            div {
                (0..gen).map(|i| rsx! {
                    div { "{i}" }
                })
            }
        })
    });

    // load the div and then assign the empty fragment as a placeholder
    assert_eq!(
        dom.rebuild().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            AssignId { path: &[0], id: ElementId(2,) },
            AppendChildren { id: ElementId(0), m: 1 },
        ]
    );

    // Rendering the first item should replace the placeholder with an element
    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(3,) },
            HydrateText { path: &[0], value: "0", id: ElementId(4,) },
            ReplaceWith { id: ElementId(2,), m: 1 },
        ]
    );

    // Rendering the next item should insert after the previous
    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
            HydrateText { path: &[0], value: "1", id: ElementId(5,) },
            InsertAfter { id: ElementId(3,), m: 1 },
        ]
    );

    // ... and again!
    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(6,) },
            HydrateText { path: &[0], value: "2", id: ElementId(7,) },
            InsertAfter { id: ElementId(2,), m: 1 },
        ]
    );

    // once more
    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(8,) },
            HydrateText { path: &[0], value: "3", id: ElementId(9,) },
            InsertAfter { id: ElementId(6,), m: 1 },
        ]
    );
}

#[test]
fn removes_one_by_one() {
    let mut dom = VirtualDom::new(|cx| {
        let gen = 3 - cx.generation() % 4;

        cx.render(rsx! {
            div {
                (0..gen).map(|i| rsx! {
                    div { "{i}" }
                })
            }
        })
    });

    // load the div and then assign the empty fragment as a placeholder
    assert_eq!(
        dom.rebuild().santize().edits,
        [
            // The container
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            // each list item
            LoadTemplate { name: "template", index: 0, id: ElementId(2) },
            HydrateText { path: &[0], value: "0", id: ElementId(3) },
            LoadTemplate { name: "template", index: 0, id: ElementId(4) },
            HydrateText { path: &[0], value: "1", id: ElementId(5) },
            LoadTemplate { name: "template", index: 0, id: ElementId(6) },
            HydrateText { path: &[0], value: "2", id: ElementId(7) },
            // replace the placeholder in the template with the 3 templates on the stack
            ReplacePlaceholder { m: 3, path: &[0] },
            // Mount the div
            AppendChildren { id: ElementId(0), m: 1 }
        ]
    );

    // Remove div(3)
    // Rendering the first item should replace the placeholder with an element
    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [Remove { id: ElementId(6) }]
    );

    // Remove div(2)
    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [Remove { id: ElementId(4) }]
    );

    // Remove div(1) and replace with a placeholder
    // todo: this should just be a remove with no placeholder
    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            CreatePlaceholder { id: ElementId(4) },
            ReplaceWith { id: ElementId(2), m: 1 }
        ]
    );

    // load the 3 and replace the placeholder
    // todo: this should actually be append to, but replace placeholder is fine for now
    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2) },
            HydrateText { path: &[0], value: "0", id: ElementId(3) },
            LoadTemplate { name: "template", index: 0, id: ElementId(5) },
            HydrateText { path: &[0], value: "1", id: ElementId(6) },
            LoadTemplate { name: "template", index: 0, id: ElementId(7) },
            HydrateText { path: &[0], value: "2", id: ElementId(8) },
            ReplaceWith { id: ElementId(4), m: 3 }
        ]
    );
}

#[test]
fn list_shrink_multiroot() {
    let mut dom = VirtualDom::new(|cx| {
        cx.render(rsx! {
            div {
                (0..cx.generation()).map(|i| rsx! {
                    div { "{i}" }
                    div { "{i}" }
                })
            }
        })
    });

    assert_eq!(
        dom.rebuild().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            AssignId { path: &[0,], id: ElementId(2,) },
            AppendChildren { id: ElementId(0), m: 1 }
        ]
    );

    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(3) },
            HydrateText { path: &[0], value: "0", id: ElementId(4) },
            LoadTemplate { name: "template", index: 1, id: ElementId(5) },
            HydrateText { path: &[0], value: "0", id: ElementId(6) },
            ReplaceWith { id: ElementId(2), m: 2 }
        ]
    );

    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2) },
            HydrateText { path: &[0], value: "1", id: ElementId(7) },
            LoadTemplate { name: "template", index: 1, id: ElementId(8) },
            HydrateText { path: &[0], value: "1", id: ElementId(9) },
            InsertAfter { id: ElementId(5), m: 2 }
        ]
    );

    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(10) },
            HydrateText { path: &[0], value: "2", id: ElementId(11) },
            LoadTemplate { name: "template", index: 1, id: ElementId(12) },
            HydrateText { path: &[0], value: "2", id: ElementId(13) },
            InsertAfter { id: ElementId(8), m: 2 }
        ]
    );
}

#[test]
fn removes_one_by_one_multiroot() {
    let mut dom = VirtualDom::new(|cx| {
        let gen = 3 - cx.generation() % 4;

        cx.render(rsx! {
            div {
                (0..gen).map(|i| rsx! {
                    div { "{i}" }
                    div { "{i}" }
                })
            }
        })
    });

    // load the div and then assign the empty fragment as a placeholder
    assert_eq!(
        dom.rebuild().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            //
            LoadTemplate { name: "template", index: 0, id: ElementId(2) },
            HydrateText { path: &[0], value: "0", id: ElementId(3) },
            LoadTemplate { name: "template", index: 1, id: ElementId(4) },
            HydrateText { path: &[0], value: "0", id: ElementId(5) },
            //
            LoadTemplate { name: "template", index: 0, id: ElementId(6) },
            HydrateText { path: &[0], value: "1", id: ElementId(7) },
            LoadTemplate { name: "template", index: 1, id: ElementId(8) },
            HydrateText { path: &[0], value: "1", id: ElementId(9) },
            //
            LoadTemplate { name: "template", index: 0, id: ElementId(10) },
            HydrateText { path: &[0], value: "2", id: ElementId(11) },
            LoadTemplate { name: "template", index: 1, id: ElementId(12) },
            HydrateText { path: &[0], value: "2", id: ElementId(13) },
            //
            ReplacePlaceholder { path: &[0], m: 6 },
            //
            AppendChildren { id: ElementId(0), m: 1 }
        ]
    );

    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [Remove { id: ElementId(10) }, Remove { id: ElementId(12) }]
    );

    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [Remove { id: ElementId(6) }, Remove { id: ElementId(8) }]
    );

    dom.mark_dirty(ScopeId(0));
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            CreatePlaceholder { id: ElementId(8) },
            Remove { id: ElementId(2) },
            ReplaceWith { id: ElementId(4), m: 1 }
        ]
    );
}

#[test]
fn two_equal_fragments_are_equal_static() {
    let mut dom = VirtualDom::new(|cx| {
        cx.render(rsx! {
            (0..5).map(|_| rsx! {
                div { "hello" }
            })
        })
    });

    _ = dom.rebuild();
    assert!(dom.render_immediate().edits.is_empty());
}

#[test]
fn two_equal_fragments_are_equal() {
    let mut dom = VirtualDom::new(|cx| {
        cx.render(rsx! {
            (0..5).map(|i| rsx! {
                div { "hello {i}" }
            })
        })
    });

    _ = dom.rebuild();
    assert!(dom.render_immediate().edits.is_empty());
}

#[test]
fn remove_many() {
    let mut dom = VirtualDom::new(|cx| {
        let num = match cx.generation() % 3 {
            0 => 0,
            1 => 1,
            2 => 5,
            _ => unreachable!(),
        };

        cx.render(rsx! {
            (0..num).map(|i| rsx! { div { "hello {i}" } })
        })
    });

    {
        let edits = dom.rebuild().santize();
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
        dom.mark_dirty(ScopeId(0));
        let edits = dom.render_immediate().santize();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
                HydrateText { path: &[0,], value: "hello 0", id: ElementId(3,) },
                ReplaceWith { id: ElementId(1,), m: 1 },
            ]
        );
    }

    {
        dom.mark_dirty(ScopeId(0));
        let edits = dom.render_immediate().santize();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
                HydrateText { path: &[0,], value: "hello 1", id: ElementId(4,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(5,) },
                HydrateText { path: &[0,], value: "hello 2", id: ElementId(6,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(7,) },
                HydrateText { path: &[0,], value: "hello 3", id: ElementId(8,) },
                LoadTemplate { name: "template", index: 0, id: ElementId(9,) },
                HydrateText { path: &[0,], value: "hello 4", id: ElementId(10,) },
                InsertAfter { id: ElementId(2,), m: 4 },
            ]
        );
    }

    {
        dom.mark_dirty(ScopeId(0));
        let edits = dom.render_immediate().santize();
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
        dom.mark_dirty(ScopeId(0));
        let edits = dom.render_immediate().santize();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
                HydrateText { path: &[0,], value: "hello 0", id: ElementId(3,) },
                ReplaceWith { id: ElementId(11,), m: 1 },
            ]
        )
    }
}
