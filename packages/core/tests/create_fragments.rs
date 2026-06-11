//! Do we create fragments properly across complex boundaries?

use dioxus::prelude::*;
use dioxus_renderer_oracle::RendererOracle;

#[test]
fn empty_fragment_creates_nothing() {
    fn app() -> Element {
        rsx!({})
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(app);
}

#[test]
fn root_fragments_work() {
    fn app() -> Element {
        rsx! {
            div { "hello" }
            div { "goodbye" }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(app);
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

    fn expected() -> Element {
        rsx! {
            div { "hello" }
            div { "goodbye" }
            div { "hello" }
            div { "goodbye" }
            div { "hello" }
            div { "goodbye" }
            div { "hello" }
            div { "goodbye" }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected);
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

    fn expected() -> Element {
        rsx! {
            "hellO!"
            "world"
            "hellO!"
            "world"
            "hellO!"
            "world"
            "hellO!"
            "world"
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected);
}

#[test]
fn list_fragments() {
    fn app() -> Element {
        rsx!(
            h1 { "hello" }
            {(0..6).map(|f| rsx!( span { "{f}" }))}
        )
    }

    fn expected() -> Element {
        rsx! {
            h1 { "hello" }
            span { "0" }
            span { "1" }
            span { "2" }
            span { "3" }
            span { "4" }
            span { "5" }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected);
}
