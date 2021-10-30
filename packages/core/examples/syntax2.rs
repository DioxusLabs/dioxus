use std::marker::PhantomData;

use dioxus::component::Scope;
use dioxus::events::on::MouseEvent;
use dioxus::nodes::{annotate_lazy, IntoVNode};
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

#[derive(PartialEq, Props)]
struct OurProps {
    foo: String,
}

fn App<'a>((cx, props): Scope<'a, OurProps>) -> Element<'a> {
    let a = rsx! {
        div {
            "asd"
            "{props.foo}"
        }
    };

    let p = (0..10).map(|f| {
        rsx! {
            div {

            }
        }
    });

    let g = match "text" {
        "a" => {
            rsx!("asd")
        }
        _ => {
            rsx!("asd")
        }
    };

    let items = ["bob", "bill", "jack"];

    let f = items
        .iter()
        .filter(|f| f.starts_with('b'))
        .map(|f| rsx!("hello {f}"));

    // use dioxus_hooks;
    // let g = use_state(|| "hello".to_string());

    let s: &'a mut String = cx.use_hook(|_| String::new(), |f| f, |_| {});

    /*
    the final closure is allowed to borrow anything provided it
    */

    // cx.render({
    //     let p: Option<Box<dyn FnOnce(_) -> _>> = Some(Box::new(move |__cx: NodeFactory| {
    //         use dioxus_elements::{GlobalAttributes, SvgAttributes};

    //         let props = Child2Props { foo: s };
    //         let ch: VNode = __cx.component(Child2, props, None, []);
    //         __cx.element(
    //             dioxus_elements::div,
    //             [],
    //             [],
    //             [ch],
    //             // [__cx.component(Child2, fc_to_builder(Child2).foo(s).build(), None, [])],
    //             None,
    //         )
    //     }));
    //     p
    //     // let ___p: Box<dyn FnOnce(NodeFactory) -> VNode> = Box::new(move |__cx| {
    //     //     use dioxus_elements::{GlobalAttributes, SvgAttributes};

    //     //     let props = Child2Props { foo: s };
    //     //     let ch: VNode = __cx.component(Child2, props, None, []);
    //     //     __cx.element(
    //     //         dioxus_elements::div,
    //     //         [],
    //     //         [],
    //     //         [ch],
    //     //         // [__cx.component(Child2, fc_to_builder(Child2).foo(s).build(), None, [])],
    //     //         None,
    //     //     )
    //     // });
    //     // Some(___p)
    // })

    let a = annotate_lazy(move |f| {
        //
        todo!()
    });
    let b = annotate_lazy(move |f| {
        //
        f.text(format_args!("{}", props.foo))
    });

    let c = annotate_lazy(move |f| {
        //
        f.component(
            Child,
            OurProps {
                //
                foo: "hello".to_string(),
            },
            None,
            [],
        )
    });

    let st: &'a String = cx.use_hook(|_| "hello".to_string(), |f| f, |_| {});

    let d = annotate_lazy(move |f| {
        //
        f.component(
            Child2,
            Child2Props {
                //
                foo: st,
            },
            None,
            [],
        )
    });

    let e = match "asd" {
        b => {
            //

            annotate_lazy(move |f| {
                //
                f.text(format_args!("{}", props.foo))
            })
        }
        a => {
            //

            annotate_lazy(move |f| {
                //
                f.text(format_args!("{}", props.foo))
            })
        }
    };

    cx.render(annotate_lazy(move |f| {
        //

        f.raw_element(
            "div",
            None,
            [],
            [],
            [
                //
                f.fragment_from_iter(a),
                f.fragment_from_iter(b),
                f.fragment_from_iter(c),
                f.fragment_from_iter(e),
            ],
            None,
        )
        // todo!()
    }))
    // cx.render(rsx! {
    //     div {
    //         div {
    //             {a}
    //             // {p}
    //             // {g}
    //             // {f}
    //         }
    //         // div {
    //         //     "asd"
    //         //     div {
    //         //         "asd"
    //         //     }
    //         // }
    //         // Child {
    //         //     foo: "asd".to_string(),
    //         // }
    //         Child2 {
    //             foo: s,
    //         }
    //     }
    // })
}

fn Child((cx, props): Scope<OurProps>) -> Element {
    cx.render(rsx! {
        div {
            div {}
        }
    })
}

#[derive(Props)]
struct Child2Props<'a> {
    foo: &'a String,
}

fn Child2<'a>((cx, props): Scope<'a, Child2Props>) -> Element<'a> {
    cx.render(rsx! {
        div {
            // div {}
        }
    })
}
