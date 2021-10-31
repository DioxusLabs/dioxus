use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;

use dioxus::component::Scope;
use dioxus::events::on::MouseEvent;
use dioxus::nodes::{annotate_lazy, IntoVNode, VComponent, VFragment, VText};
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {}

fn t() {
    let g = rsx! {
        div {
            div {

            }
        }
    };

    let g = {
        let ___p: Box<dyn FnOnce(NodeFactory) -> VNode> = Box::new(|__cx: NodeFactory| {
            use dioxus_elements::{GlobalAttributes, SvgAttributes};
            __cx.element(dioxus_elements::div, [], [], [], None)
        });
        // let __z = ___p as ;
        // __z
    };
}

// #[derive(PartialEq, Props)]
// struct OurProps {
//     foo: String,
// }

// fn App<'a>((cx, props): Scope<'a, OurProps>) -> Element<'a> {
//     let a = rsx! {
//         div {
//             "asd"
//             "{props.foo}"
//         }
//     };

//     let p = (0..10).map(|f| {
//         rsx! {
//             div {

//             }
//         }
//     });

//     let g = match "text" {
//         "a" => {
//             rsx!("asd")
//         }
//         _ => {
//             rsx!("asd")
//         }
//     };

//     let items = ["bob", "bill", "jack"];

//     let f = items
//         .iter()
//         .filter(|f| f.starts_with('b'))
//         .map(|f| rsx!("hello {f}"));

//     // use dioxus_hooks;
//     // let g = use_state(|| "hello".to_string());

//     let s: &'a mut String = cx.use_hook(|_| String::new(), |f| f, |_| {});

//     /*
//     the final closure is allowed to borrow anything provided it
//     */
//     // cx.render({
//     //     let p: Option<Box<dyn FnOnce(_) -> _>> = Some(Box::new(move |__cx: NodeFactory| {
//     //         use dioxus_elements::{GlobalAttributes, SvgAttributes};

//     //         let props = Child2Props { foo: s };
//     //         let ch: VNode = __cx.component(Child2, props, None, []);
//     //         __cx.element(
//     //             dioxus_elements::div,
//     //             [],
//     //             [],
//     //             [ch],
//     //             // [__cx.component(Child2, fc_to_builder(Child2).foo(s).build(), None, [])],
//     //             None,
//     //         )
//     //     }));
//     //     p
//     //     // let ___p: Box<dyn FnOnce(NodeFactory) -> VNode> = Box::new(move |__cx| {
//     //     //     use dioxus_elements::{GlobalAttributes, SvgAttributes};

//     //     //     let props = Child2Props { foo: s };
//     //     //     let ch: VNode = __cx.component(Child2, props, None, []);
//     //     //     __cx.element(
//     //     //         dioxus_elements::div,
//     //     //         [],
//     //     //         [],
//     //     //         [ch],
//     //     //         // [__cx.component(Child2, fc_to_builder(Child2).foo(s).build(), None, [])],
//     //     //         None,
//     //     //     )
//     //     // });
//     //     // Some(___p)
//     // })

//     let a = annotate_lazy(move |f| {
//         //
//         todo!()
//     });
//     let b = annotate_lazy(move |f| {
//         //
//         f.text(format_args!("{}", props.foo))
//     });

//     let c = annotate_lazy(move |f| {
//         //
//         f.component(
//             Child,
//             OurProps {
//                 //
//                 foo: "hello".to_string(),
//             },
//             None,
//             [],
//         )
//     });

//     let st: &'a String = cx.use_hook(|_| "hello".to_string(), |f| f, |_| {});

//     let d = annotate_lazy(move |f| {
//         //
//         f.component(
//             Child2,
//             Child2Props {
//                 //
//                 foo: st,
//             },
//             None,
//             [],
//         )
//     });

//     let e = match "asd" {
//         b => {
//             //

//             annotate_lazy(move |f| {
//                 //
//                 f.text(format_args!("{}", props.foo))
//             })
//         }
//         a => {
//             //

//             annotate_lazy(move |f| {
//                 //
//                 f.text(format_args!("{}", props.foo))
//             })
//         }
//     };

//     cx.render(annotate_lazy(move |f| {
//         //

//         f.raw_element(
//             "div",
//             None,
//             [],
//             [],
//             [
//                 //
//                 f.fragment_from_iter(a),
//                 f.fragment_from_iter(b),
//                 f.fragment_from_iter(c),
//                 f.fragment_from_iter(e),
//             ],
//             None,
//         )
//         // todo!()
//     }))
//     // cx.render(rsx! {
//     //     div {
//     //         div {
//     //             {a}
//     //             // {p}
//     //             // {g}
//     //             // {f}
//     //         }
//     //         // div {
//     //         //     "asd"
//     //         //     div {
//     //         //         "asd"
//     //         //     }
//     //         // }
//     //         // Child {
//     //         //     foo: "asd".to_string(),
//     //         // }
//     //         Child2 {
//     //             foo: s,
//     //         }
//     //     }
//     // })
// }

// fn Child((cx, props): Scope<OurProps>) -> Element {
//     cx.render(rsx! {
//         div {
//             div {}
//         }
//     })
// }

#[derive(Props)]
struct Child2Props<'a> {
    foo: &'a String,
}

fn Child2<'a>((cx, props): Scope<'a, Child2Props>) -> Element<'a> {
    let node = cx
        .render(rsx! {
            div {

            }
        })
        .unwrap();

    let b = cx.bump();
    let node: &'a VNode<'a> = b.alloc(node);

    let children = ChildList { pthru: node };

    // let c = VComponent {
    //     key: todo!(),
    //     associated_scope: todo!(),
    //     is_static: todo!(),
    //     user_fc: todo!(),
    //     caller: todo!(),
    //     children: todo!(),
    //     comparator: todo!(),
    //     drop_props: todo!(),
    //     can_memoize: todo!(),
    //     raw_props: todo!(),
    // };

    // Vcomp
    // - borrowed
    // - memoized

    // cx.render({
    //     NodeFactory::annotate_lazy(move |__cx: NodeFactory| -> VNode {
    //         use dioxus_elements::{GlobalAttributes, SvgAttributes};
    //         __cx.element(
    //             dioxus_elements::div,
    //             [],
    //             [],
    //             [
    //                 __cx.component(ChildrenMemo, (), None, []),
    //                 __cx.component(
    //                     ChildrenComp,
    //                     //
    //                     ChildrenTest { node: children },
    //                     None,
    //                     [],
    //                 ),
    //                 // {
    //                 //     let _props: &_ = __cx.bump().alloc(ChildrenTest { node: children });
    //                 //     __cx.component_v2_borrowed(
    //                 //         //
    //                 //         move |c| ChildrenComp((c, _props)),
    //                 //         ChildrenComp,
    //                 //         _props,
    //                 //     )
    //                 // },
    //                 // {
    //                 //     let _props: &_ = __cx.bump().alloc(());
    //                 //     __cx.component_v2_borrowed(move |c| ChildrenMemo((c, _props)))
    //                 // },
    //             ],
    //             None,
    //         )
    //     })
    // })
    cx.render(rsx! {
        div {
            ChildrenComp {
                ..ChildrenTest {
                    node: children,
                }
            }
        }
    })
}

#[derive(Props)]
struct ChildrenTest<'a> {
    node: ChildList<'a>,
}

struct ChildList<'a> {
    pthru: &'a VNode<'a>,
}

impl<'a> Clone for ChildList<'a> {
    fn clone(&self) -> Self {
        Self { pthru: self.pthru }
    }
}
impl<'a> Copy for ChildList<'a> {}

impl<'a> IntoVNode<'a> for ChildList<'a> {
    fn into_vnode(self, _: NodeFactory<'a>) -> VNode<'a> {
        match self.pthru {
            VNode::Text(f) => VNode::Text(*f),
            VNode::Element(e) => VNode::Element(*e),
            VNode::Component(c) => VNode::Component(*c),
            VNode::Suspended(s) => VNode::Suspended(*s),
            VNode::Anchor(a) => VNode::Anchor(a),
            VNode::Fragment(f) => VNode::Fragment(VFragment {
                children: f.children,
                is_static: f.is_static,
                key: f.key,
            }),
        }
    }
}

fn ChildrenComp<'a>((cx, props): Scope<'a, ChildrenTest<'a>>) -> Element<'a> {
    cx.render(rsx! {
        div {
            div {

                // if the node's id is already assigned, then it's being passed in as a child
                // in these instances, we don't worry about re-checking the node?

                {Some(props.node)}
            }
        }
    })
}

fn ChildrenMemo((cx, props): Scope<()>) -> Element {
    todo!()
}
