use std::rc::Rc;

use dioxus_core::{hooks::use_ref, prelude::Context};

use crate::{Atom, AtomValue, Readable, RecoilApi, RecoilRoot};

/// This hook initializes the Recoil Context - should only be placed once per app.
///
///
/// ```ignore
///
///
///
/// ```
pub fn init_root(ctx: Context) {
    ctx.use_create_context(move || RecoilRoot::new())
}

/// Use an atom and its setter
///
/// This hook subscribes the component to any updates to the atom.
///
/// ```rust
/// const TITLE: Atom<&str> = atom(|_| "default");
///
/// static App: FC<()> = |ctx, props| {
///     let (title, set_title) = recoil::use_state(ctx, &TITLE);
///     ctx.render(rsx!{
///         div {
///             "{title}"
///             button {"on", onclick: move |_| set_title("on")}
///             button {"off", onclick: move |_| set_title("off")}
///         }
///     })
/// }
///
/// ```
pub fn use_recoil_state<'a, T: PartialEq + 'static>(
    ctx: Context<'a>,
    readable: &'static impl Readable<T>,
) -> (&'a T, &'a Rc<dyn Fn(T)>) {
    struct RecoilStateInner<G: AtomValue> {
        root: Rc<RecoilRoot>,
        value: Rc<G>,
        setter: Rc<dyn Fn(G)>,
    }

    let root = ctx.use_context::<RecoilRoot>();
    let (subscriber_id, value) = root.subscribe_consumer(readable, ctx.schedule_update());

    ctx.use_hook(
        move || RecoilStateInner {
            value,
            root: root.clone(),
            setter: Rc::new(move |new_val| root.update_atom(readable, new_val)),
        },
        move |hook| {
            hook.value = hook.root.load_value(readable);
            (hook.value.as_ref(), &hook.setter)
        },
        // Make sure we unsubscribe
        // It's not *wrong* for a dead component to receive updates, but it is less performant
        move |hook| hook.root.drop_consumer(subscriber_id),
    )
}

///
///
///
/// ```ignore
/// let (title, set_title) = recoil::use_state()
///
///
///
/// ```
pub fn use_recoil_value<'a, T: PartialEq>(ctx: Context<'a>, t: &'static impl Readable<T>) -> &'a T {
    todo!()
}

/// Update an atom's value without
///
/// Enable the ability to set a value without subscribing the componet
///
///
///
/// ```ignore
/// let (title, set_title) = recoil::use_state()
///
///
///
/// ```
pub fn use_set_state<'a, T: PartialEq>(c: Context<'a>, t: &'static Atom<T>) -> &'a Rc<dyn Fn(T)> {
    todo!()
}

///
///
///
/// ```ignore
/// let (title, set_title) = recoil::use_state()
///
///
///
/// ```
pub fn use_recoil_callback<'a, F: 'a>(
    ctx: Context<'a>,
    f: impl Fn(RecoilApi) -> F + 'static,
) -> &F {
    todo!()
}
