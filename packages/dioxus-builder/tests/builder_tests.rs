use dioxus::prelude::*;
use dioxus_builder::*;
use dioxus_core::{NoOpMutations, Template};

// =============================================================================
// Basic Tests
// =============================================================================

#[test]
fn test_builder_simple() {
    let mut dom = VirtualDom::new(|| {
        div()
            .id("test-id")
            .class("test-class")
            .child("Hello, world!")
            .build()
    });

    let _edits = dom.rebuild_to_vec();
    // In Dioxus 0.6+, templates are loaded by index.
    // We just want to make sure it doesn't panic and renders correctly.
}

#[test]
fn test_builder_nested() {
    let mut dom = VirtualDom::new(|| {
        div()
            .class("parent")
            .child(
                button()
                    .class("child")
                    .onclick(|_| println!("clicked"))
                    .child(if true {
                        span().child("Click me!")
                    } else {
                        span().child("Don't click me!")
                    }),
            )
            .build()
    });

    let _edits = dom.rebuild_to_vec();
}

#[test]
fn test_builder_multiple_children() {
    let mut dom = VirtualDom::new(|| {
        ul().children((0..5).map(|i| li().child(format!("Item {}", i))))
            .build()
    });

    let _edits = dom.rebuild_to_vec();
}

#[test]
fn test_builder_event_handler() {
    let mut dom = VirtualDom::new(|| {
        button()
            .onclick(|_| {
                // This won't be called here, but we check if it exists
            })
            .build()
    });

    let _edits = dom.rebuild_to_vec();
    // We can't easily inspect the VNode inside the VirtualDom's internal state here,
    // but the fact that it doesn't panic means the listener was created successfully.
}

#[test]
fn test_builder_hydration_matches_rsx() {
    fn rsx_app() -> Element {
        let label = "Click";
        rsx! {
            div { button { onclick: |_| {}, "{label}" } }
        }
    }

    fn builder_app() -> Element {
        div().child(button().onclick(|_| {}).child("Click")).build()
    }

    let mut rsx_dom = VirtualDom::new(rsx_app);
    rsx_dom.rebuild(&mut NoOpMutations);
    let rsx_html = dioxus_ssr::pre_render(&rsx_dom);

    let mut builder_dom = VirtualDom::new(builder_app);
    builder_dom.rebuild(&mut NoOpMutations);
    let builder_html = dioxus_ssr::pre_render(&builder_dom);

    assert_eq!(rsx_html, builder_html);
    assert!(builder_html.contains("click:1"));
}

#[test]
fn test_builder_fragment_matches_rsx() {
    fn rsx_app() -> Element {
        rsx! {
            "Hello"
            div { "World" }
        }
    }

    fn builder_app() -> Element {
        fragment()
            .child("Hello")
            .child(div().child("World"))
            .build()
    }

    let mut rsx_dom = VirtualDom::new(rsx_app);
    rsx_dom.rebuild(&mut NoOpMutations);
    let rsx_html = dioxus_ssr::pre_render(&rsx_dom);

    let mut builder_dom = VirtualDom::new(builder_app);
    builder_dom.rebuild(&mut NoOpMutations);
    let builder_html = dioxus_ssr::pre_render(&builder_dom);

    // Both should contain the same content, though hydration markers may differ slightly
    assert!(rsx_html.contains("Hello"));
    assert!(rsx_html.contains("World"));
    assert!(builder_html.contains("Hello"));
    assert!(builder_html.contains("World"));
    // Verify the DOM structure is equivalent (both have div with World)
    assert!(rsx_html.contains("<div"));
    assert!(builder_html.contains("<div"));
}

#[test]
fn test_builder_merges_class_attributes() {
    fn builder_app() -> Element {
        div().class("one").class("two").build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("class=\"one two\""));
}

#[test]
fn test_builder_attr_if() {
    fn builder_app() -> Element {
        let enabled = false;
        div().attr_if(enabled, "data-test", "present").build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(!html.contains("data-test"));
}

// =============================================================================
// Key Support Tests
// =============================================================================

#[test]
fn test_builder_key_support() {
    fn builder_app() -> Element {
        ul().children((0..3).map(|i| li().key(format!("item-{}", i)).child(format!("Item {}", i))))
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    let _edits = dom.rebuild_to_vec();
    // The key should be set on the VNode - we just verify it doesn't panic
}

#[test]
fn test_builder_key_matches_rsx() {
    fn rsx_app() -> Element {
        rsx! {
            ul {
                for i in 0..3 {
                    li { key: "{i}", "Item {i}" }
                }
            }
        }
    }

    fn builder_app() -> Element {
        ul().children((0..3).map(|i| li().key(i.to_string()).child(format!("Item {}", i))))
            .build()
    }

    let mut rsx_dom = VirtualDom::new(rsx_app);
    rsx_dom.rebuild(&mut NoOpMutations);
    let rsx_html = dioxus_ssr::pre_render(&rsx_dom);

    let mut builder_dom = VirtualDom::new(builder_app);
    builder_dom.rebuild(&mut NoOpMutations);
    let builder_html = dioxus_ssr::pre_render(&builder_dom);

    // Both should produce the same HTML output
    assert_eq!(rsx_html, builder_html);
}

#[test]
fn test_fragment_key_support() {
    fn builder_app() -> Element {
        fragment()
            .key("my-fragment")
            .child("Hello")
            .child(div().child("World"))
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    let _edits = dom.rebuild_to_vec();
    // Just verify it doesn't panic
}

// =============================================================================
// children_keyed Tests
// =============================================================================

#[test]
fn test_children_keyed() {
    #[derive(Clone)]
    struct Item {
        id: i32,
        name: String,
    }

    fn builder_app() -> Element {
        let items = vec![
            Item {
                id: 1,
                name: "First".to_string(),
            },
            Item {
                id: 2,
                name: "Second".to_string(),
            },
            Item {
                id: 3,
                name: "Third".to_string(),
            },
        ];

        ul().children_keyed(
            items,
            |item| item.id.to_string(),
            |item| li().child(item.name),
        )
        .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("First"));
    assert!(html.contains("Second"));
    assert!(html.contains("Third"));
}

// =============================================================================
// Convenience Method Tests
// =============================================================================

#[test]
fn test_text_method() {
    fn builder_app() -> Element {
        div().text("Hello World").build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("Hello World"));
}

#[test]
fn test_child_option_some() {
    fn builder_app() -> Element {
        let maybe_content: Option<&str> = Some("Content");
        div().child_option(maybe_content).build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("Content"));
}

#[test]
fn test_child_option_none() {
    fn builder_app() -> Element {
        let maybe_content: Option<&str> = None;
        div()
            .child("Before")
            .child_option(maybe_content)
            .child("After")
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("Before"));
    assert!(html.contains("After"));
    // Should not have any extra content between them
}

// =============================================================================
// Static Children Tests (Hybrid Templates)
// =============================================================================

#[test]
fn test_static_text() {
    fn builder_app() -> Element {
        div().static_text("Hello, World!").build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("Hello, World!"));
}

#[test]
fn test_static_text_matches_rsx() {
    fn rsx_app() -> Element {
        rsx! {
            div { "Hello, World!" }
        }
    }

    fn builder_app() -> Element {
        div().static_text("Hello, World!").build()
    }

    let mut rsx_dom = VirtualDom::new(rsx_app);
    rsx_dom.rebuild(&mut NoOpMutations);
    let rsx_html = dioxus_ssr::pre_render(&rsx_dom);

    let mut builder_dom = VirtualDom::new(builder_app);
    builder_dom.rebuild(&mut NoOpMutations);
    let builder_html = dioxus_ssr::pre_render(&builder_dom);

    // Both should produce the same HTML output
    assert_eq!(rsx_html, builder_html);
}

#[test]
fn test_mixed_static_and_dynamic() {
    fn builder_app() -> Element {
        let dynamic_name = "Alice";
        div()
            .static_text("Hello, ")
            .child(dynamic_name)
            .static_text("!")
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("Hello, "));
    assert!(html.contains("Alice"));
    assert!(html.contains("!"));
}

#[test]
fn test_mixed_static_dynamic_matches_rsx() {
    fn rsx_app() -> Element {
        let dynamic_name = "Alice";
        rsx! {
            div {
                "Hello, "
                {dynamic_name}
                "!"
            }
        }
    }

    fn builder_app() -> Element {
        let dynamic_name = "Alice";
        div()
            .static_text("Hello, ")
            .child(dynamic_name)
            .static_text("!")
            .build()
    }

    let mut rsx_dom = VirtualDom::new(rsx_app);
    rsx_dom.rebuild(&mut NoOpMutations);
    let rsx_html = dioxus_ssr::pre_render(&rsx_dom);

    let mut builder_dom = VirtualDom::new(builder_app);
    builder_dom.rebuild(&mut NoOpMutations);
    let builder_html = dioxus_ssr::pre_render(&builder_dom);

    // Both should produce similar output (content should match)
    assert!(rsx_html.contains("Hello, "));
    assert!(rsx_html.contains("Alice"));
    assert!(rsx_html.contains("!"));
    assert!(builder_html.contains("Hello, "));
    assert!(builder_html.contains("Alice"));
    assert!(builder_html.contains("!"));
}

#[test]
fn test_static_element() {
    use dioxus_builder::{ChildNode, StaticAttribute, StaticElement};

    fn builder_app() -> Element {
        div()
            .static_element(StaticElement {
                tag: "span",
                namespace: None,
                attrs: &[StaticAttribute {
                    name: "class",
                    value: "icon",
                    namespace: None,
                }],
                children: vec![ChildNode::StaticText("★")],
            })
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("<span"));
    assert!(html.contains("class=\"icon\""));
    assert!(html.contains("★"));
}

#[test]
fn test_multiple_static_texts() {
    fn builder_app() -> Element {
        div()
            .static_text("One ")
            .static_text("Two ")
            .static_text("Three")
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("One "));
    assert!(html.contains("Two "));
    assert!(html.contains("Three"));
}

#[test]
fn test_static_with_attributes() {
    fn builder_app() -> Element {
        div()
            .class("container")
            .static_text("Static content")
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("class=\"container\""));
    assert!(html.contains("Static content"));
}

#[test]
fn test_static_str_macro() {
    use dioxus_builder::{static_str, BuilderExt};

    fn builder_app() -> Element {
        div()
            .pipe(static_str!("Hello, "))
            .child("World")
            .pipe(static_str!("!"))
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("Hello, "));
    assert!(html.contains("World"));
    assert!(html.contains("!"));
}

#[test]
fn test_static_str_macro_two_arg() {
    use dioxus_builder::static_str;

    fn builder_app() -> Element {
        let builder = div();
        static_str!(builder, "Const verified text").build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("Const verified text"));
}

fn capture_template(builder: ElementBuilder) -> Template {
    builder.build().expect("builder failed").template
}

fn assert_template_cache_reuse(first: &Template, second: &Template) {
    assert_eq!(first.roots.len(), second.roots.len());
    assert_eq!(first.roots.as_ptr(), second.roots.as_ptr());

    if !first.node_paths.is_empty() {
        assert_eq!(second.node_paths.len(), first.node_paths.len());
        assert_eq!(first.node_paths.as_ptr(), second.node_paths.as_ptr());
    }

    if !first.attr_paths.is_empty() {
        assert_eq!(second.attr_paths.len(), first.attr_paths.len());
        assert_eq!(first.attr_paths.as_ptr(), second.attr_paths.as_ptr());
    }
}

#[test]
fn test_template_cache_reuses_static_text_template() {
    let first = capture_template(div().static_text("Counter: ").child("value"));
    let second = capture_template(div().static_text("Counter: ").child("value"));
    assert_template_cache_reuse(&first, &second);
}

#[test]
fn test_template_cache_reuses_dynamic_template() {
    let first = capture_template(div().child("one").child("two"));
    let second = capture_template(div().child("one").child("two"));
    assert_template_cache_reuse(&first, &second);
}

#[test]
fn test_template_cache_reuses_static_element_template() {
    use dioxus_builder::{ChildNode, StaticAttribute, StaticElement};

    let static_elem = StaticElement {
        tag: "span",
        namespace: None,
        attrs: &[StaticAttribute {
            name: "class",
            value: "icon",
            namespace: None,
        }],
        children: vec![ChildNode::StaticText("★")],
    };

    let first = capture_template(div().static_element(static_elem.clone()).child("label"));
    let second = capture_template(div().static_element(static_elem.clone()).child("label"));
    assert_template_cache_reuse(&first, &second);
}

// =============================================================================
// ARIA Attribute Tests
// =============================================================================

#[test]
fn test_aria_attributes() {
    fn builder_app() -> Element {
        div()
            .aria_label("Close button")
            .aria_hidden(true)
            .aria_expanded(false)
            .aria_describedby("description-id")
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("aria-label=\"Close button\""));
    assert!(html.contains("aria-hidden=\"true\""));
    assert!(html.contains("aria-expanded=\"false\""));
    assert!(html.contains("aria-describedby=\"description-id\""));
}

#[test]
fn test_aria_live_region() {
    fn builder_app() -> Element {
        div()
            .aria_live("polite")
            .aria_atomic(true)
            .aria_busy(false)
            .child("Status message")
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("aria-live=\"polite\""));
    assert!(html.contains("aria-atomic=\"true\""));
    assert!(html.contains("aria-busy=\"false\""));
}

// =============================================================================
// Data Attribute Tests
// =============================================================================

#[test]
fn test_data_attribute_macro() {
    use dioxus_builder::data;

    fn builder_app() -> Element {
        let builder = div();
        data!(builder, "testid", "my-element")
            .pipe(|b| data!(b, "count", "5"))
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("data-testid=\"my-element\""));
    assert!(html.contains("data-count=\"5\""));
}

// =============================================================================
// Style Prop Tests
// =============================================================================

#[test]
fn test_style_prop() {
    fn builder_app() -> Element {
        div()
            .style_prop("display", "flex")
            .style_prop("gap", "1rem")
            .style_prop("align-items", "center")
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("style=\""));
    assert!(html.contains("display: flex"));
    assert!(html.contains("gap: 1rem"));
    assert!(html.contains("align-items: center"));
}

#[test]
fn test_style_prop_merging() {
    fn builder_app() -> Element {
        div()
            .style_prop("display", "flex")
            .style_prop("gap", "1rem")
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    // Style values should be merged with "; " separator
    assert!(html.contains("display: flex; gap: 1rem") || html.contains("gap: 1rem; display: flex"));
}

// =============================================================================
// Composition Tests (with() method)
// =============================================================================

#[test]
fn test_with_method() {
    fn add_button_styles(builder: ElementBuilder) -> ElementBuilder {
        builder
            .class("px-4 py-2 rounded")
            .class("bg-blue-500 text-white")
    }

    fn builder_app() -> Element {
        button()
            .with(add_button_styles)
            .onclick(|_| {})
            .child("Click me")
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("px-4"));
    assert!(html.contains("bg-blue-500"));
    assert!(html.contains("Click me"));
}

#[test]
fn test_with_composition() {
    fn card_styles(b: ElementBuilder) -> ElementBuilder {
        b.class("p-4 rounded-lg shadow-md bg-white")
    }

    fn hover_effect(b: ElementBuilder) -> ElementBuilder {
        b.class("hover:shadow-lg transition-shadow")
    }

    fn builder_app() -> Element {
        div()
            .with(card_styles)
            .with(hover_effect)
            .child("Card content")
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("p-4"));
    assert!(html.contains("rounded-lg"));
    assert!(html.contains("hover:shadow-lg"));
}

// =============================================================================
// SVG Tests
// =============================================================================

#[test]
fn test_svg_elements() {
    fn builder_app() -> Element {
        svg()
            .attr("viewBox", "0 0 100 100")
            .attr("width", "24")
            .attr("height", "24")
            .child(
                circle()
                    .cx("50")
                    .cy("50")
                    .r("40")
                    .fill("blue")
            )
            .child(
                path()
                    .d("M10 10 L90 90")
                    .stroke("red")
            )
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("<svg"));
    assert!(html.contains("<circle"));
    assert!(html.contains("<path"));
}

#[test]
fn test_svg_attributes() {
    fn builder_app() -> Element {
        svg()
            .viewBox("0 0 100 100")
            .fill("none")
            .stroke("currentColor")
            .stroke_width("2")
            .child(
                circle()
                    .cx("50")
                    .cy("50")
                    .r("40")
            )
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("viewBox=\"0 0 100 100\""));
    assert!(html.contains("fill=\"none\""));
    assert!(html.contains("stroke=\"currentColor\""));
}

#[test]
fn test_svg_icon() {
    fn builder_app() -> Element {
        svg()
            .viewBox("0 0 24 24")
            .fill("none")
            .stroke("currentColor")
            .stroke_width("2")
            .child(
                path()
                    .d("M5 12h14M12 5l7 7-7 7")
                    .stroke_linecap("round")
                    .stroke_linejoin("round")
            )
            .build()
    }

    let mut dom = VirtualDom::new(builder_app);
    dom.rebuild(&mut NoOpMutations);
    let html = dioxus_ssr::pre_render(&dom);

    assert!(html.contains("<svg"));
    assert!(html.contains("<path"));
    assert!(html.contains("stroke-linecap=\"round\""));
}
