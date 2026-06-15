#![allow(unused, non_upper_case_globals, non_snake_case)]

//! Prove that the dom works normally through virtualdom methods.

use dioxus::prelude::*;
use dioxus_renderer_oracle::RendererOracle;

#[test]
fn test_original_diff() {
    fn app() -> Element {
        rsx! { div { div { "Hello, world!" } } }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(app);
}

#[test]
fn create() {
    fn app() -> Element {
        rsx! {
            div {
                div {
                    "Hello, world!"
                    div {
                        div {
                            Fragment { "hello" "world" }
                        }
                    }
                }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(app);
}

#[test]
fn create_list() {
    fn app() -> Element {
        rsx! {{(0..3).map(|_| rsx!( div { "hello" } ))}}
    }

    fn expected() -> Element {
        rsx! {
            div { "hello" }
            div { "hello" }
            div { "hello" }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected);
}

#[test]
fn create_simple() {
    fn app() -> Element {
        rsx! { div {} div {} div {} div {} }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(app);
}

#[test]
fn create_components() {
    fn app() -> Element {
        rsx! {
            Child { "abc1" }
            Child { "abc2" }
            Child { "abc3" }
        }
    }

    #[derive(Props, Clone, PartialEq)]
    struct ChildProps {
        children: Element,
    }

    fn Child(cx: ChildProps) -> Element {
        rsx! {
            h1 {}
            div { {cx.children} }
            p {}
        }
    }

    fn expected() -> Element {
        rsx! {
            h1 {}
            div { "abc1" }
            p {}
            h1 {}
            div { "abc2" }
            p {}
            h1 {}
            div { "abc3" }
            p {}
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected);
}

#[test]
fn anchors() {
    fn app() -> Element {
        rsx! {
            if true {
                 div { "hello" }
            }
            if false {
                div { "goodbye" }
            }
        }
    }

    fn expected() -> Element {
        rsx! {
            div { "hello" }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected);
}

#[test]
fn empty_fragment_root_via_direct_vnode_api_is_diffable() {
    // `VNode::new` normalizes `DynamicNode::Fragment(Vec::new())` to
    // `DynamicNode::Placeholder(..)` so the diff path never sees an empty fragment.
    // Without that normalization, callers using the direct `VNode::new(..)` API would
    // bypass the rsx macro's `IntoDynNode` collapse and trip
    // `index out of bounds: the len is 0 but the index is 0` on the second rerender.
    use dioxus_core::{DynamicNode, ScopeId, Template, TemplateCursor, VNode, VirtualDom};
    use dioxus_renderer_oracle::RendererOracle;

    fn app() -> Element {
        static NODE_CURSORS: &[TemplateCursor] = &[TemplateCursor::new(&[0u8])];
        static TEMPLATE: Template = Template::new(&[], NODE_CURSORS, &[]);
        Ok(VNode::new(
            None,
            TEMPLATE,
            Box::new([DynamicNode::Fragment(Vec::new())]),
            Vec::<Box<[dioxus_core::Attribute]>>::new().into_boxed_slice(),
        ))
    }

    let mut vdom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut vdom);
    vdom.mark_dirty(ScopeId::APP);
    oracle.render(&mut vdom);
    vdom.mark_dirty(ScopeId::APP);
    oracle.render(&mut vdom);
}
