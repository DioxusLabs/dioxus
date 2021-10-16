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
    dioxus::desktop::launch(App, |c| c);
}

fn App((cx, props): Component<()>) -> DomTree {
    let text: &mut Vec<String> = cx.use_hook(|_| vec![String::from("abc=def")], |f| f, |_| {});

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

impl<'a> Drop for C1Props<'a> {
    fn drop(&mut self) {}
}

fn Child1<'a>((cx, props): Component<'a, C1Props>) -> DomTree<'a> {
    let (left, right) = props.text.split_once("=").unwrap();

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

fn Child2<'a>((cx, props): Component<'a, C2Props>) -> DomTree<'a> {
    cx.render(rsx! {
        Child3 {
            text: props.text
        }
    })
}

#[derive(Props)]
struct C3Props<'a> {
    text: &'a str,
}

fn Child3<'a>((cx, props): Component<'a, C3Props>) -> DomTree<'a> {
    cx.render(rsx! {
        div { "{props.text}"}
    })
}
