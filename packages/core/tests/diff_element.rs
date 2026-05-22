use dioxus::dioxus_core::AttributeValue;
use dioxus::prelude::*;
use dioxus_core::{ScopeId, generation};
use dioxus_renderer_oracle::{EditSummary, RendererOracle};

#[test]
fn text_diff() {
    fn app() -> Element {
        let g = generation();
        rsx!( h1 { "hello {g}" } )
    }

    fn expected_0() -> Element {
        rsx!( h1 { "hello 0" } )
    }

    fn expected_1() -> Element {
        rsx!( h1 { "hello 1" } )
    }

    fn expected_2() -> Element {
        rsx!( h1 { "hello 2" } )
    }

    fn expected_3() -> Element {
        rsx!( h1 { "hello 3" } )
    }

    let (mut dom, mut oracle, _) = rebuild(app, expected_0);
    assert_eq!(rerender(&mut dom, &mut oracle, expected_1).set_texts, 1);
    assert_eq!(rerender(&mut dom, &mut oracle, expected_2).set_texts, 1);
    assert_eq!(rerender(&mut dom, &mut oracle, expected_3).set_texts, 1);
}

#[test]
fn element_swap() {
    fn app() -> Element {
        let g = generation();

        match g % 2 {
            0 => rsx!( h1 { "hello 1" } ),
            1 => rsx!( h2 { "hello 2" } ),
            _ => unreachable!(),
        }
    }

    fn expected_h1() -> Element {
        rsx!( h1 { "hello 1" } )
    }

    fn expected_h2() -> Element {
        rsx!( h2 { "hello 2" } )
    }

    // Anchor diff: swapping the root element to a different tag emits
    // `load_template` + `remove_node` for the old root (no `replace_node_with`).
    let (mut dom, mut oracle, _) = rebuild(app, expected_h1);
    for expected in [expected_h2, expected_h1, expected_h2, expected_h1] {
        let summary = rerender(&mut dom, &mut oracle, expected);
        assert_eq!(summary.loads, 1);
        assert_eq!(summary.removes, 1);
        assert_eq!(summary.replaces, 0);
    }
}

#[test]
fn attribute_diff() {
    fn attr(name: &'static str, value: &'static str) -> Attribute {
        Attribute::new(name, AttributeValue::Text(value.into()), None, false)
    }

    fn app() -> Element {
        let g = generation();

        // attributes have to be sorted by name
        let attrs = match g % 5 {
            0 => vec![Attribute::new(
                "a",
                AttributeValue::Text("hello".into()),
                None,
                false,
            )],
            1 => vec![
                Attribute::new("a", AttributeValue::Text("hello".into()), None, false),
                Attribute::new("b", AttributeValue::Text("hello".into()), None, false),
                Attribute::new("c", AttributeValue::Text("hello".into()), None, false),
            ],
            2 => vec![
                Attribute::new("c", AttributeValue::Text("hello".into()), None, false),
                Attribute::new("d", AttributeValue::Text("hello".into()), None, false),
                Attribute::new("e", AttributeValue::Text("hello".into()), None, false),
            ],
            3 => vec![Attribute::new(
                "d",
                AttributeValue::Text("world".into()),
                None,
                false,
            )],
            _ => unreachable!(),
        };

        rsx!(
            div {
                ..attrs,
                "hello"
            }
        )
    }

    fn expected_0() -> Element {
        rsx!( div { ..vec![attr("a", "hello")], "hello" } )
    }

    fn expected_1() -> Element {
        rsx!( div { ..vec![attr("a", "hello"), attr("b", "hello"), attr("c", "hello")], "hello" } )
    }

    fn expected_2() -> Element {
        rsx!( div { ..vec![attr("c", "hello"), attr("d", "hello"), attr("e", "hello")], "hello" } )
    }

    fn expected_3() -> Element {
        rsx!( div { ..vec![attr("d", "world")], "hello" } )
    }

    let (mut dom, mut oracle, _) = rebuild(app, expected_0);
    assert_eq!(rerender(&mut dom, &mut oracle, expected_1).set_attrs, 2);
    assert_eq!(rerender(&mut dom, &mut oracle, expected_2).set_attrs, 4);
    assert_eq!(rerender(&mut dom, &mut oracle, expected_3).set_attrs, 3);
}

#[test]
fn dynamic_attr_override_restores_static_attr() {
    fn attr(name: &'static str, value: &'static str) -> Attribute {
        Attribute::new(name, AttributeValue::Text(value.into()), None, false)
    }

    fn app() -> Element {
        let attrs = if generation() % 2 == 0 {
            vec![attr("class", "active")]
        } else {
            vec![]
        };

        rsx! {
            div {
                class: "base",
                ..attrs,
            }
        }
    }

    fn expected_active() -> Element {
        rsx! { div { class: "active" } }
    }

    fn expected_base() -> Element {
        rsx! { div { class: "base" } }
    }

    let (mut dom, mut oracle, _) = rebuild(app, expected_active);
    rerender(&mut dom, &mut oracle, expected_base);
    rerender(&mut dom, &mut oracle, expected_active);
}

#[test]
fn dynamic_attr_override_restores_raw_static_attr() {
    fn attr(name: &'static str, value: &'static str) -> Attribute {
        Attribute::new(name, AttributeValue::Text(value.into()), None, false)
    }

    fn app() -> Element {
        let attrs = if generation() % 2 == 0 {
            vec![attr("as", "script")]
        } else {
            vec![]
        };

        rsx! {
            link {
                href: "/style.css",
                r#as: "style",
                ..attrs,
            }
        }
    }

    fn expected_script() -> Element {
        rsx! { link { href: "/style.css", r#as: "script" } }
    }

    fn expected_style() -> Element {
        rsx! { link { href: "/style.css", r#as: "style" } }
    }

    let (mut dom, mut oracle, _) = rebuild(app, expected_script);
    rerender(&mut dom, &mut oracle, expected_style);
    rerender(&mut dom, &mut oracle, expected_script);
}

#[test]
fn dynamic_attr_override_restores_aliased_static_attr() {
    fn attr(name: &'static str, value: &'static str) -> Attribute {
        Attribute::new(name, AttributeValue::Text(value.into()), None, false)
    }

    fn app() -> Element {
        let attrs = if generation() % 2 == 0 {
            vec![attr("http-equiv", "refresh")]
        } else {
            vec![]
        };

        rsx! {
            meta {
                "http.z": "custom",
                http_equiv: "content-type",
                ..attrs,
            }
        }
    }

    fn expected_refresh() -> Element {
        rsx! { meta { "http.z": "custom", http_equiv: "refresh" } }
    }

    fn expected_content_type() -> Element {
        rsx! { meta { "http.z": "custom", http_equiv: "content-type" } }
    }

    let (mut dom, mut oracle, _) = rebuild(app, expected_refresh);
    rerender(&mut dom, &mut oracle, expected_content_type);
    rerender(&mut dom, &mut oracle, expected_refresh);
}

#[test]
fn dynamic_attr_none_removes_static_attr() {
    fn app() -> Element {
        let attrs = if generation() % 2 == 0 {
            vec![Attribute::new("class", AttributeValue::None, None, false)]
        } else {
            vec![]
        };

        rsx! {
            div {
                class: "base",
                ..attrs,
            }
        }
    }

    fn expected_empty() -> Element {
        rsx! { div {} }
    }

    fn expected_base() -> Element {
        rsx! { div { class: "base" } }
    }

    let (mut dom, mut oracle, _) = rebuild(app, expected_empty);
    rerender(&mut dom, &mut oracle, expected_base);
    rerender(&mut dom, &mut oracle, expected_empty);
}

#[test]
fn duplicate_dynamic_attr_slots_use_final_effective_attr() {
    fn attr(value: &'static str) -> Attribute {
        Attribute::new("class", AttributeValue::Text(value.into()), None, false)
    }

    fn app() -> Element {
        let generation = generation();
        let first = match generation {
            0..=2 => vec![attr("first")],
            _ => vec![],
        };
        let second = match generation {
            0..=1 => vec![attr("second")],
            _ => vec![],
        };

        rsx! {
            div {
                ..first,
                ..second,
            }
        }
    }

    fn expected_second() -> Element {
        rsx! { div { class: "second" } }
    }

    fn expected_first() -> Element {
        rsx! { div { class: "first" } }
    }

    fn expected_empty() -> Element {
        rsx! { div {} }
    }

    let (mut dom, mut oracle, _) = rebuild(app, expected_second);
    rerender(&mut dom, &mut oracle, expected_second);
    rerender(&mut dom, &mut oracle, expected_first);
    rerender(&mut dom, &mut oracle, expected_empty);
}

#[test]
fn diff_empty() {
    fn app() -> Element {
        match generation() % 2 {
            0 => rsx! { div { "hello" } },
            1 => rsx! {},
            _ => unreachable!(),
        }
    }

    fn expected_div() -> Element {
        rsx! { div { "hello" } }
    }

    fn expected_empty() -> Element {
        rsx! {}
    }

    // Anchor diff: removing the root element emits `remove_node` only
    // (no placeholder needs to take its place in the markerless model).
    let (mut dom, mut oracle, _) = rebuild(app, expected_div);
    let summary = rerender(&mut dom, &mut oracle, expected_empty);
    assert_eq!(summary.removes, 1);
    assert_eq!(summary.replaces, 0);
}

fn rebuild(
    app: fn() -> Element,
    expected: fn() -> Element,
) -> (VirtualDom, RendererOracle, EditSummary) {
    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    let summary = oracle.rebuild(&mut dom);
    oracle.assert_matches(expected);
    (dom, oracle, summary)
}

fn rerender(
    dom: &mut VirtualDom,
    oracle: &mut RendererOracle,
    expected: fn() -> Element,
) -> EditSummary {
    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(dom);
    oracle.assert_matches(expected);
    summary
}
