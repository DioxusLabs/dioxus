//! An alternative function syntax
//!

use std::marker::PhantomData;

use dioxus_core::prelude::VNode;

fn main() {}

struct Context2<'a> {
    _p: PhantomData<&'a ()>,
}

type FC2<'a, 'b, 'c: 'a + 'b, P> = fn(Context2<'a>, &'b P) -> VNode<'c>;

static Example: FC2<()> = |ctx, props| {
    //
    todo!()
};
