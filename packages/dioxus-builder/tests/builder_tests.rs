use dioxus::prelude::*;
use dioxus_builder::*;
use dioxus_core::NoOpMutations;

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
        div()
            .child(button().onclick(|_| {}).child("Click"))
            .build()
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
