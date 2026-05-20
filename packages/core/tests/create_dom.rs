#![allow(unused, non_upper_case_globals, non_snake_case)]

//! Prove that the dom works normally through virtualdom methods.

use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

#[test]
fn test_original_diff() {
    Sequence::new()
        .render(rsx! { div { div { "Hello, world!" } } })
        .run();
}

#[test]
fn create() {
    Sequence::new()
        .render({
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
        })
        .run();
}

#[test]
fn create_list() {
    fn app() -> Element {
        rsx! {{(0..3).map(|_| rsx!( div { "hello" } ))}}
    }

    Sequence::new()
        .render_with_expected(
            app,
            rsx! {
                div { "hello" }
                div { "hello" }
                div { "hello" }
            },
        )
        .run();
}

#[test]
fn create_simple() {
    Sequence::new()
        .render(rsx! { div {} div {} div {} div {} })
        .run();
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

    Sequence::new()
        .render_with_expected(
            app,
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
            },
        )
        .run();
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

    Sequence::new()
        .render_with_expected(
            app,
            rsx! {
                if true {
                     div { "hello" }
                }
                if false {
                    div { "goodbye" }
                }
            },
        )
        .run();
}

#[test]
fn empty_fragment_root_via_direct_vnode_api_is_diffable() {
    // `VNode::new` normalizes `DynamicNode::Fragment(Vec::new())` to
    // `DynamicNode::Placeholder(..)` so the diff path never sees an empty fragment.
    // Without that normalization, callers using the direct `VNode::new(..)` API would
    // bypass the rsx macro's `IntoDynNode` collapse and trip
    // `index out of bounds: the len is 0 but the index is 0` on the second rerender.
    use dioxus_core::{DynamicNode, ScopeId, Template, TemplateNode, VNode, VirtualDom};
    use dioxus_renderer_oracle::RendererOracle;

    fn app() -> Element {
        let template = Template::new(&[TemplateNode::Dynamic { id: 0 }], &[&[0u8] as &[u8]], &[]);
        Ok(VNode::new(
            None,
            template,
            Box::new([DynamicNode::Fragment(Vec::new())]),
            Vec::<Box<[dioxus_core::Attribute]>>::new().into_boxed_slice(),
        ))
    }

    let mut vdom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    vdom.rebuild(&mut oracle);
    vdom.mark_dirty(ScopeId::APP);
    vdom.render_immediate(&mut oracle);
    vdom.mark_dirty(ScopeId::APP);
    vdom.render_immediate(&mut oracle);
}
