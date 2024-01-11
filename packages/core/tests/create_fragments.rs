//! Do we create fragments properly across complex boundaries?

use dioxus::core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::ElementId;

#[test]
fn empty_fragment_creates_nothing() {
    fn app() -> Element {
        render!(())
    }

    let mut vdom = VirtualDom::new(app);
    let edits = vdom.rebuild_to_vec();

    assert_eq!(
        edits.edits,
        [
            CreatePlaceholder { id: ElementId(1) },
            AppendChildren { id: ElementId(0), m: 1 }
        ]
    );
}

#[test]
fn root_fragments_work() {
    let mut vdom = VirtualDom::new(|cx| {
        render!(
            div { "hello" }
            div { "goodbye" }
        )
    });

    assert_eq!(
        vdom.rebuild_to_vec().edits.last().unwrap(),
        &AppendChildren { id: ElementId(0), m: 2 }
    );
}

#[test]
fn fragments_nested() {
    let mut vdom = VirtualDom::new(|cx| {
        render!(
            div { "hello" }
            div { "goodbye" }
            render! {
                div { "hello" }
                div { "goodbye" }
                render! {
                    div { "hello" }
                    div { "goodbye" }
                    render! {
                        div { "hello" }
                        div { "goodbye" }
                    }
                }
            }
        )
    });

    assert_eq!(
        vdom.rebuild_to_vec().edits.last().unwrap(),
        &AppendChildren { id: ElementId(0), m: 8 }
    );
}

#[test]
fn fragments_across_components() {
    fn app() -> Element {
        render! {
            demo_child {}
            demo_child {}
            demo_child {}
            demo_child {}
        }
    }

    fn demo_child(cx: Scope) -> Element {
        let world = "world";
        render! { "hellO!", world }
    }

    assert_eq!(
        VirtualDom::new(app).rebuild_to_vec().edits.last().unwrap(),
        &AppendChildren { id: ElementId(0), m: 8 }
    );
}

#[test]
fn list_fragments() {
    fn app() -> Element {
        render!(
            h1 { "hello" }
            (0..6).map(|f| render!( span { "{f}" }))
        )
    }
    assert_eq!(
        VirtualDom::new(app).rebuild_to_vec().edits.last().unwrap(),
        &AppendChildren { id: ElementId(0), m: 7 }
    );
}
