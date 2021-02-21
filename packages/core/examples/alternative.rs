//! An alternative function syntax
//!

use std::marker::PhantomData;

use bumpalo::Bump;
use dioxus_core::prelude::{DomTree, VNode};

fn main() {}

struct Context2<'a, P> {
    _props: &'a P, // _p: PhantomData<&'a ()>,
    rops: &'a P,   // _p: PhantomData<&'a ()>,
}
impl<'a, P> Context2<'a, P> {
    fn view(self, f: impl FnOnce(&'a Bump) -> VNode<'a>) -> DTree {
        DTree {}
    }

    fn props(&self) -> &'a P {
        todo!()
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

trait Properties {}

struct DTree;
// type FC2<'a, T: 'a> = fn(Context2<T>) -> DTree;
fn virtual_child<'a, T: 'a>(bump: &'a Bump, props: T, f: FC2<T>) -> VNode<'a> {
    todo!()
}

struct Props {
    c: String,
}

fn Example(ctx: Context2<Props>) -> DTree {
    let val = use_state(&ctx, || String::from("asd"));
    let props = ctx.props();

    ctx.view(move |b| {
        dioxus_core::nodebuilder::div(b)
            .child(dioxus_core::nodebuilder::text(props.c.as_str()))
            .child(virtual_child(b, Props2 { a: val }, AltChild))
            .finish()
    })
}

// #[fc]
fn Example2(ctx: Context2<()>, name: &str, blah: &str) -> DTree {
    let val = use_state(&ctx, || String::from("asd"));

    ctx.view(move |b| {
        dioxus_core::nodebuilder::div(b)
            .child(dioxus_core::nodebuilder::text(name))
            .child(virtual_child(b, Props2 { a: val }, AltChild))
            .finish()
    })
}

type FC2<'a, T: 'a> = fn(Context2<T>) -> DTree;

// still works if you don't take any references in your props (ie, something copy or cloneable)
static CHILD: FC2<Props2> = |ctx: Context2<Props2>| {
    //
    todo!()
};

struct Props2<'a> {
    a: &'a String,
}
impl Properties for Props2<'_> {}

fn AltChild(ctx: Context2<Props2>) -> DTree {
    ctx.view(|b| {
        //
        todo!()
    })
}

fn use_state<'a, 'c, P, T: 'static, F: FnOnce() -> T>(
    ctx: &'_ Context2<'a, P>,
    initial_state_fn: F,
) -> (&'a T) {
    todo!()
}
