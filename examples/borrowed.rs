#![allow(non_snake_case)]

/*
Dioxus manages borrow lifetimes for you. This means any child may borrow from its parent. However, it is not possible
to hand out an &mut T to children - all props are consumed by &P, so you'd only get an &&mut T.

How does it work?

Dioxus will manually drop closures and props - things that borrow data before the component is ran again. This is done
"bottom up" from the lowest child all the way to the initiating parent. As it traverses each listener and prop, the
drop implementation is manually called, freeing any memory and ensuring that memory is not leaked.

We cannot drop from the parent to the children - if the drop implementation modifies the data, downstream references
might be broken since we take an &mut T and and &T to the data. Instead, we work bottom up, making sure to remove any
potential references to the data before finally giving out an &mut T. This prevents us from mutably aliasing the data,
and is proven to be safe with MIRI.
*/

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let text = cx.use_hook(|| vec![String::from("abc=def")]);

    let first = text.get_mut(0).unwrap();

    cx.render(rsx! {
        div {
            Child1 {
                text: first
            }
        }
    })
}

#[derive(Props)]
struct C1Props<'a> {
    text: &'a mut String,
}

fn Child1<'a>(cx: Scope<'a, C1Props<'a>>) -> Element {
    let (left, right) = cx.props.text.split_once('=').unwrap();

    cx.render(rsx! {
        div {
            Child2 { text: left  }
            Child2 { text: right  }
        }
    })
}

#[derive(Props)]
struct C2Props<'a> {
    text: &'a str,
}

fn Child2<'a>(cx: Scope<'a, C2Props<'a>>) -> Element {
    cx.render(rsx! {
        Child3 {
            text: cx.props.text
        }
    })
}

#[derive(Props)]
struct C3Props<'a> {
    text: &'a str,
}

fn Child3<'a>(cx: Scope<'a, C3Props<'a>>) -> Element {
    cx.render(rsx! {
        div { "{cx.props.text}"}
    })
}
