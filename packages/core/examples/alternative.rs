//! An alternative function syntax
//!

use std::marker::PhantomData;

use bumpalo::Bump;
use dioxus_core::prelude::{DomTree, VNode};

fn main() {}

struct Context2<'a, P> {
    props: &'a P, // _p: PhantomData<&'a ()>,
}
impl<'a, P> Context2<'a, P> {
    fn view(&self, f: impl FnOnce(&'a Bump) -> VNode<'a>) -> DTree {
        DTree {}
    }

    pub fn use_hook<'scope, InternalHookState: 'static, Output: 'a>(
        &'scope self,
        initializer: impl FnOnce() -> InternalHookState,
        runner: impl FnOnce(&'a mut InternalHookState) -> Output,
        cleanup: impl FnOnce(InternalHookState),
    ) -> Output {
        todo!()
    }
}

struct DTree;
type FC2<'a, T: 'a> = fn(&'a Context2<T>) -> DTree;

struct Props {
    c: String,
}

static Example: FC2<Props> = |ctx| {
    let val = use_state(&ctx, || String::from("asd"));

    ctx.view(move |b| {
        let g: VNode<'_> = virtual_child(b, Props2 { a: val }, alt_child);
        //
        dioxus_core::nodebuilder::div(b)
            .child(dioxus_core::nodebuilder::text(ctx.props.c.as_str()))
            .child(g)
            .finish()
    })
};

fn virtual_child<'a, T: 'a>(bump: &'a Bump, props: T, f: FC2<T>) -> VNode<'a> {
    todo!()
}

struct Props2<'a> {
    a: &'a String,
}

fn alt_child<'a>(ctx: &Context2<'a, Props2<'_>>) -> DomTree {
    todo!()
}

static CHILD: FC2<Props2> = |ctx| {
    //
    todo!()
};

fn use_state<'a, 'c, P, T: 'static, F: FnOnce() -> T>(
    ctx: &'a Context2<'a, P>,
    initial_state_fn: F,
) -> (&'a T) {
    todo!()
}
