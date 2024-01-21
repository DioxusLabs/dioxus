use dioxus::core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::{AttributeValue, ElementId};

#[test]
fn text_diff() {
    fn app(cx: Scope) -> Element {
        let gen = cx.generation();
        cx.render(rsx!( h1 { "hello {gen}" } ))
    }

    let mut vdom = VirtualDom::new(app);
    _ = vdom.rebuild();

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().edits,
        [SetText { value: "hello 1", id: ElementId(2) }]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().edits,
        [SetText { value: "hello 2", id: ElementId(2) }]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().edits,
        [SetText { value: "hello 3", id: ElementId(2) }]
    );
}

#[test]
fn element_swap() {
    fn app(cx: Scope) -> Element {
        let gen = cx.generation();

        match gen % 2 {
            0 => cx.render(rsx!( h1 { "hello 1" } )),
            1 => cx.render(rsx!( h2 { "hello 2" } )),
            _ => unreachable!(),
        }
    }

    let mut vdom = VirtualDom::new(app);
    _ = vdom.rebuild();

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
            ReplaceWith { id: ElementId(1,), m: 1 },
        ]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            ReplaceWith { id: ElementId(2,), m: 1 },
        ]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
            ReplaceWith { id: ElementId(1,), m: 1 },
        ]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            ReplaceWith { id: ElementId(2,), m: 1 },
        ]
    );
}

#[test]
fn attribute_diff() {
    fn app(cx: Scope) -> Element {
        let gen = cx.generation();

        let attrs = cx.bump().alloc(match gen % 2 {
            0 => vec![Attribute::new(
                "attr1",
                AttributeValue::Text("hello"),
                None,
                false,
            )],
            1 => vec![
                Attribute::new("attr1", AttributeValue::Text("hello"), None, false),
                Attribute::new("attr2", AttributeValue::Float(1.0), None, false),
                Attribute::new("attr3", AttributeValue::Int(1), None, false),
                Attribute::new("attr4", AttributeValue::Bool(true), None, false),
            ],
            _ => unreachable!(),
        });

        cx.render(rsx!(
            div {
                ..*attrs,
                "hello"
            }
        ))
    }

    let mut vdom = VirtualDom::new(app);
    _ = vdom.rebuild();

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().santize().edits,
        [
            SetAttribute {
                name: "attr2",
                value: (&AttributeValue::Float(1.0)).into(),
                id: ElementId(1),
                ns: None
            },
            SetAttribute {
                name: "attr3",
                value: (&AttributeValue::Int(1)).into(),
                id: ElementId(1),
                ns: None
            },
            SetAttribute {
                name: "attr4",
                value: (&AttributeValue::Bool(true)).into(),
                id: ElementId(1),
                ns: None
            },
        ]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().santize().edits,
        [
            SetAttribute {
                name: "attr2",
                value: (&AttributeValue::None).into(),
                id: ElementId(1),
                ns: None
            },
            SetAttribute {
                name: "attr3",
                value: (&AttributeValue::None).into(),
                id: ElementId(1),
                ns: None
            },
            SetAttribute {
                name: "attr4",
                value: (&AttributeValue::None).into(),
                id: ElementId(1),
                ns: None
            },
        ]
    );

    vdom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        vdom.render_immediate().santize().edits,
        [
            SetAttribute {
                name: "attr2",
                value: (&AttributeValue::Float(1.0)).into(),
                id: ElementId(1),
                ns: None
            },
            SetAttribute {
                name: "attr3",
                value: (&AttributeValue::Int(1)).into(),
                id: ElementId(1),
                ns: None
            },
            SetAttribute {
                name: "attr4",
                value: (&AttributeValue::Bool(true)).into(),
                id: ElementId(1),
                ns: None
            },
        ]
    );
}
