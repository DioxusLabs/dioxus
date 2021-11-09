use std::marker::PhantomData;

use dioxus::component::Scope;
use dioxus::events::on::MouseEvent;
use dioxus::nodes::IntoVNode;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {}

fn html_usage() {
    let mo = move |_| {};
    let r = rsx! {
        div {
            onclick: move |_| {}
            onmouseover: {mo}
            "type": "bar",
            "hello world"
        }
    };

    let items = ["bob", "bill", "jack"];

    let f = items
        .iter()
        .filter(|f| f.starts_with('b'))
        .map(|f| rsx!("hello {f}"));

    // let p = rsx!(div { {f} });
}

static App2: FC<()> = |(cx, _)| cx.render(rsx!("hello world!"));

static App: FC<()> = |(cx, props)| {
    let name = cx.use_state(|| 0);

    cx.render(rsx!(div {
        h1 {}
        h2 {}
    }))
};

pub trait UseState<'a, T: 'static> {
    fn use_state(self, f: impl FnOnce() -> T) -> &'a T;
}
impl<'a, T: 'static> UseState<'a, T> for Context<'a> {
    fn use_state(self, f: impl FnOnce() -> T) -> &'a T {
        todo!()
    }
}

fn App3((cx, props): Scope<()>) -> Element {
    let p = rsx! {
        Child {
            bame: 10,
        }
    };
    todo!()
    // cx.render(rsx!(Child {
    //     bame: 102,
    //     ..ChildProps { bame: 10 }
    // }))
}

#[derive(Props, PartialEq, Debug)]
struct ChildProps {
    bame: i32, // children: Children<'a>,
}

fn Child<'a>((cx, props): Scope<'a, ChildProps>) -> Element<'a> {
    cx.render(rsx!(div {
        // {props.children}
    }))
}

// Some(LazyNodes::new(|f| {
//     //
//     // let r = f.fragment_from_iter(&props.children);
//     r
//     // todo!()
// }))
// todo!()
// rsx!({ Some(p) })
// todo!()

pub struct Children<'a> {
    children: VNode<'static>,
    _p: PhantomData<&'a ()>,
}

impl<'a> Children<'a> {
    pub fn new(children: VNode<'a>) -> Self {
        Self {
            children: unsafe { std::mem::transmute(children) },
            _p: PhantomData,
        }
    }
}

static Bapp: FC<()> = |(cx, props)| {
    let name = cx.use_state(|| 0);

    cx.render(rsx!(
        div {
            div {

            }
            div {

            }
        }
    ))
};

static Match: FC<()> = |(cx, props)| {
    //
    let b: Box<dyn Fn(NodeFactory) -> VNode> = Box::new(|f| todo!());

    let b = match "ag" {
        "a" => {
            let __b: Box<dyn FnOnce(NodeFactory) -> VNode> = Box::new(|f: NodeFactory| todo!());
            __b
        }
        _ => {
            let __b: Box<dyn FnOnce(NodeFactory) -> VNode> = Box::new(|f: NodeFactory| todo!());
            __b
        }
    };

    // let b: Box<dyn Fn(NodeFactory) -> VNode> = match "alph" {
    //     "beta" => Box::new(|f: NodeFactory| {
    //         //
    //         todo!()
    //     }),
    //     _ => Box::new(|f: NodeFactory| {
    //         //
    //         todo!()
    //     }),
    // };

    cx.render(rsx! {
        div {

        }
    })
};
