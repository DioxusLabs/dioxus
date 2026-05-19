//! Do we create fragments properly across complex boundaries?

use dioxus::dioxus_core::Mutation::*;
use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

#[test]
fn empty_fragment_creates_nothing() {
    fn app() -> Element {
        rsx!({})
    }

    let mut vdom = VirtualDom::new(app);
    let edits = vdom.rebuild_to_vec();

    assert_eq!(edits.edits.len(), 2);
    assert!(matches!(edits.edits[0], CreatePlaceholder { .. }));
    assert!(matches!(edits.edits[1], AppendChildren { m: 1, .. }));
}

#[test]
fn root_fragments_work() {
    Sequence::new()
        .render({
            rsx! {
                div { "hello" }
                div { "goodbye" }
            }
        })
        .run();
}

#[test]
fn fragments_nested() {
    fn app() -> Element {
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
    }

    Sequence::new().render_with(app).run();
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
        rsx! { "hellO!" {world} }
    }

    Sequence::new().render_with(app).run();
}

#[test]
fn list_fragments() {
    fn app() -> Element {
        rsx!(
            h1 { "hello" }
            {(0..6).map(|f| rsx!( span { "{f}" }))}
        )
    }

    Sequence::new().render_with(app).run();
}
