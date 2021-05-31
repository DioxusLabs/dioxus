use std::{ops::Deref, rc::Rc};

use dioxus::virtual_dom::Scope;
use dioxus_core::prelude::*;

type RcStr = Rc<str>;

fn main() {
    let r: RcStr = "asdasd".into();
    let r: RcStr = String::from("asdasd").into();

    let g = rsx! {
        div {
            Example {}
        }
    };
}

static Example: FC<()> = |ctx, props| {
    let nodes = ctx.children();

    //
    rsx! { in ctx,
        div {
            {nodes}
        }
    }
};

#[derive(Clone, Copy)]
struct MyContext<'a, T> {
    props: &'a T,
    inner: &'a Scope,
}
impl<'a, T> MyContext<'a, T> {
    fn children(&self) -> Vec<VNode<'a>> {
        todo!()
    }
    pub fn render2<F: for<'b> FnOnce(&'b NodeCtx<'a>) -> VNode<'a> + 'a>(
        &self,
        lazy_nodes: LazyNodes<'a, F>,
    ) -> VNode<'a> {
        self.inner.render2(lazy_nodes)
    }
}

impl<'a, T> Deref for MyContext<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.props
    }
}

struct MyProps {
    title: String,
}

fn example(scope: MyContext<MyProps>) -> VNode {
    let childs = scope.children();

    scope.inner.render2(rsx! {
        div {
            "{scope.title}"
            {childs}
        }
    })
}
