use bumpalo::Bump;
use dioxus_core::prelude::{Context, VNode};
use std::{any::Any, cell::RefCell, rc::Rc};
use std::{borrow::Borrow, sync::atomic::AtomicUsize};
use typed_arena::Arena;

fn main() {
    let ar = Arena::new();

    (0..5).for_each(|f| {
        // Create the temporary context obect
        let c = Context {
            _p: std::marker::PhantomData {},
            props: (),
            idx: 0.into(),
            arena: &ar,
            hooks: RefCell::new(Vec::new()),
        };

        component(c);
    });
}

// we need to do something about props and context being borrowed from different sources....
// kinda anooying
/// use_ref creates a new value when the component is created and then borrows that value on every render
fn component(ctx: Context<()>) {
    (0..10).for_each(|f| {
        let r = use_ref(&ctx, move || f);
        assert_eq!(*r, f);
    });
}

pub fn use_ref<'a, P, T: 'static>(
    ctx: &'a Context<'a, P>,
    initial_state_fn: impl FnOnce() -> T + 'static,
) -> &'a T {
    ctx.use_hook(
        || initial_state_fn(), // initializer
        |state| state,         // runner, borrows the internal value
        |b| {},                // tear down
    )
}
