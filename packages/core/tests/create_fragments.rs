//! Do we create fragments properly across complex boundaries?

use dioxus::dioxus_core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::ElementId;

#[test]
fn empty_fragment_creates_nothing() {
    fn app() -> Element {
        rsx!({})
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
    let mut vdom = VirtualDom::new(|| {
        rsx!(
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
    let mut vdom = VirtualDom::new(|| {
        rsx!(
            div { "hello" }
            div { "goodbye" }
            {rsx! {
                div { "hello" }
                div { "goodbye" }
                {rsx! {
                    div { "hello" }
                    div { "goodbye" }
                    {rsx! {
                        div { "hello" }
                        div { "goodbye" }
                    }}
                }}
            }}
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
        rsx! {
            demo_child {}
            demo_child {}
            demo_child {}
            demo_child {}
        }
    }

    fn demo_child() -> Element {
        let world = "world";
        rsx! { "hellO!", {world} }
    }

    assert_eq!(
        VirtualDom::new(app).rebuild_to_vec().edits.last().unwrap(),
        &AppendChildren { id: ElementId(0), m: 8 }
    );
}

#[test]
fn list_fragments() {
    fn app() -> Element {
        rsx!(
            h1 { "hello" }
            {(0..6).map(|f| rsx!( span { "{f}" }))}
        )
    }
    assert_eq!(
        VirtualDom::new(app).rebuild_to_vec().edits.last().unwrap(),
        &AppendChildren { id: ElementId(0), m: 7 }
    );
}
