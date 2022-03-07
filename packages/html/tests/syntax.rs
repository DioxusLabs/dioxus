#![allow(clippy::mut_from_ref)]

use dioxus_core::prelude::*;
use dioxus_html::builder::*;

#[test]
fn test_builder() {
    struct Div<'a> {
        builder: ElementBuilder<'a>,
    }

    impl HtmlElement for Div<'_> {
        fn builder(&mut self) -> &mut ElementBuilder {
            todo!()
        }
    }

    pub trait Builder {
        fn build(&mut self) -> VNode {
            todo!()
        }
    }

    impl Builder for Div<'_> {
        fn build(&mut self) -> VNode {
            todo!()
        }
    }

    pub trait HtmlElement {
        fn builder(&mut self) -> &mut ElementBuilder;

        fn contenteditable(&mut self) -> &mut Self {
            todo!()
        }

        fn data(&mut self) -> &mut Self {
            todo!()
        }

        fn dir(&mut self) -> &mut Self {
            todo!()
        }

        fn draggable(&mut self) -> &mut Self {
            todo!()
        }

        fn hidden(&mut self) -> &mut Self {
            todo!()
        }

        fn onclick(&mut self, f: impl FnOnce(())) -> &mut Self {
            todo!()
        }

        fn children<const LEN: usize>(&mut self, children: [&mut dyn Builder; LEN]) -> &mut Self {
            todo!()
        }
    }

    fn div(cx: &ScopeState) -> &mut Div {
        todo!()
    }

    impl<'a> Div<'a> {
        fn href(&mut self) -> &mut Self {
            self
        }
    }

    fn please(cx: Scope) -> Element {
        let r = div(&cx)
            .href()
            .hidden()
            .dir()
            .data()
            .draggable()
            .children([
                div(&cx),
                div(&cx),
                div(&cx),
                div(&cx),
                div(&cx),
                div(&cx),
                div(&cx),
            ])
            .onclick(move |_| {
                //
            });

        None
    }

    //     #[allow(unused)]
    //     fn please(cx: Scope) -> Element {
    //         div(&cx)
    //             .class("a")
    //             .draggable(false)
    //             .id("asd")
    //             .accesskey(false)
    //             .class(false)
    //             .contenteditable(false)
    //             .data(false)
    //             .dir(false)
    //             .dangerous_inner_html(false)
    //             .attr("name", "asd")
    //             .onclick(move |_| println!("clicked"))
    //             .onclick(move |evt| println!("clicked"))
    //             .onclick(move |_| println!("clicked"))
    //             .children([
    //                 match true {
    //                     true => div(&cx),
    //                     false => div(&cx).class("asd"),
    //                 },
    //                 match 10 {
    //                     10 => div(&cx),
    //                     _ => div(&cx).class("asd"),
    //                 },
    //                 match true {
    //                     true => div(&cx),
    //                     false => div(&cx).class("asd"),
    //                 },
    //                 fragment(&cx).child_iter((0..10).map(|i| {
    //                     div(&cx)
    //                         .class("val")
    //                         .class(format_args!("{}", i))
    //                         .class("val")
    //                 })),
    //                 fragment(&cx).child_iter((0..10).map(|i| {
    //                     div(&cx)
    //                         .class("val")
    //                         .class(format_args!("{}", i))
    //                         .class("val")
    //                 })),
    //                 fragment(&cx).child_iter((0..10).map(|i| {
    //                     button(&cx).class("val").onclick(move |_| {
    //                         // do thing here
    //                     })
    //                 })),
    //                 Component.builder(&cx),
    //             ])
    //             .build()
    //     }
}

// struct MyProps {}

// fn Component(cx: Scope<MyProps>) -> Element {
//     todo!()
// }

// trait ComponentBuilder<P> {
//     #[allow(clippy::mut_from_ref)]
//     fn builder(self, cx: &ScopeState) -> &mut ElementBuilder<P>;
// }

// impl<P, F> ComponentBuilder<P> for F
// where
//     F: for<'a> Fn(Scope<'a, P>) -> Element<'a>,
// {
//     fn builder(self, cx: &ScopeState) -> &mut ElementBuilder<P> {
//         todo!()
//     }
// }

// // // aaack
// // // element builder is not extensible.
// // impl<'a> ElementBuilder<'a, MyProps> {
// //     fn blah(&mut self) -> &mut Self {
// //         self
// //     }
// // }

// pub trait HtmlNamespace {
//     fn class(&self, class: impl Into<String>) -> &Self;
// }

// pub struct Miv<'a> {
//     builder: ElementBuilder<'a>,
// }
