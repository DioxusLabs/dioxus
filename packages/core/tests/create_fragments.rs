//! Do we create fragments properly across complex boundaries?

use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

#[test]
fn empty_fragment_creates_nothing() {
    fn app() -> Element {
        rsx!({})
    }

    Sequence::new().render_with_expected(app, rsx!({})).run();
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

    Sequence::new()
        .render_with_expected(
            app,
            rsx! {
                div { "hello" }
                div { "goodbye" }
                div { "hello" }
                div { "goodbye" }
                div { "hello" }
                div { "goodbye" }
                div { "hello" }
                div { "goodbye" }
            },
        )
        .run();
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

    Sequence::new()
        .render_with_expected(
            app,
            rsx! {
                "hellO!"
                "world"
                "hellO!"
                "world"
                "hellO!"
                "world"
                "hellO!"
                "world"
            },
        )
        .run();
}

#[test]
fn list_fragments() {
    fn app() -> Element {
        rsx!(
            h1 { "hello" }
            {(0..6).map(|f| rsx!( span { "{f}" }))}
        )
    }

    Sequence::new()
        .render_with_expected(
            app,
            rsx! {
                h1 { "hello" }
                span { "0" }
                span { "1" }
                span { "2" }
                span { "3" }
                span { "4" }
                span { "5" }
            },
        )
        .run();
}
