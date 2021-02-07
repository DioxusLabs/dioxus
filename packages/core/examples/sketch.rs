use bumpalo::Bump;
use dioxus_core::{
    prelude::{Context, VElement, VNode, FC},
    virtual_dom::{Properties, Scope},
};
use std::{
    any::Any,
    borrow::BorrowMut,
    cell::RefCell,
    mem::swap,
    ops::{Deref, DerefMut},
    rc::Rc,
};
use std::{borrow::Borrow, sync::atomic::AtomicUsize};
use typed_arena::Arena;

fn main() {
    let mut scope = Scope::new(component);

    (0..5).for_each(|f| {
        let ctx = scope.create_context();
        component(ctx);
    });
}

// we need to do something about props and context being borrowed from different sources....
// kinda anooying
/// use_ref creates a new value when the component is created and then borrows that value on every render
fn component(ctx: Context<()>) -> VNode {
    (0..10).for_each(|f| {
        let r = use_ref(&ctx, move || f);
    });
    todo!()
}

pub struct UseRef<T: 'static> {
    current: RefCell<T>,
}
impl<T: 'static> UseRef<T> {
    fn new(val: T) -> Self {
        Self {
            current: RefCell::new(val),
        }
    }

    fn modify(&self, modifier: impl FnOnce(&mut T)) {
        let mut val = self.current.borrow_mut();
        let val_as_ref = val.deref_mut();
        modifier(val_as_ref);
    }
}

/// Store a mutable value between renders!
/// To read the value, borrow the ref.
/// To change it, use modify.
/// Modifications to this value do not cause updates to the component
pub fn use_ref<'a, P, T: 'static>(
    ctx: &'a Context<'a, P>,
    initial_state_fn: impl FnOnce() -> T + 'static,
) -> &'a UseRef<T> {
    ctx.use_hook(
        || UseRef::new(initial_state_fn()),
        |state, _| &*state,
        |_| {},
    )
}

struct UseState<T> {
    new_val: Rc<RefCell<Option<T>>>,
    current_val: T,
    caller: Box<dyn Fn(T) + 'static>,
}

/// Store state between component renders!
/// When called, this hook retrives a stored value and provides a setter to update that value.
/// When the setter is called, the component is re-ran with the new value.
///
/// This is behaves almost exactly the same way as React's "use_state".
///
/// Usage:
/// ```ignore
/// static Example: FC<()> = |ctx| {
///     let (counter, set_counter) = use_state(ctx, || 0);
///     let increment = || set_couter(counter + 1);
///     let decrement = || set_couter(counter + 1);
///
///     html! {
///         <div>
///             <h1>"Counter: {counter}" </h1>
///             <button onclick={increment}> "Increment" </button>
///             <button onclick={decrement}> "Decrement" </button>
///         </div>  
///     }
/// }
/// ```
pub fn use_state<'a, P: Properties + 'static, T: 'static, F: FnOnce() -> T + 'static>(
    ctx: &'a Context<'a, P>,
    initial_state_fn: F,
) -> (&T, &impl Fn(T)) {
    ctx.use_hook(
        move || UseState {
            new_val: Rc::new(RefCell::new(None)),
            current_val: initial_state_fn(),
            caller: Box::new(|_| println!("setter called!")),
        },
        move |hook, updater| {
            let inner = hook.new_val.clone();

            // todo: swap out the caller with a subscription call and an internal update
            hook.caller = Box::new(move |new_val| {
                // update the setter with the new value
                let mut new_inner = inner.as_ref().borrow_mut();
                *new_inner = Some(new_val);
            });

            // box gets derefed into a ref which is then taken as ref with the hook
            (&hook.current_val, &hook.caller)
        },
        |_| {},
    )
}

fn test_use_state(ctx: Context<()>) -> VNode {
    let (val, set_val) = use_state(&ctx, || 10);

    // gloriousness!
    // closures are taken with ref to ctx :)
    // Can freely use hooks
    let handler_0: _ = || set_val(val + 1);
    let handler_1: _ = || set_val(val + 10);
    let handler_2: _ = || set_val(val + 100);

    // these fns are captured, boxed into the bump arena, and then attached to the listeners
    // the vnodes share the lifetime of these closures (and the hook data)
    // whenever a listener wakes up, we take the reference directly from the bump arena and, with a small bit
    // of unsafe code, execute the associated closure / listener function
    // Those vnodes are then tossed out and new ones are installed, meaning and old references (potentially bad)
    // are removed and UB is prevented from affecting the program
    {
        VNode::Element(VElement::new("button"))
    }
}

fn test_use_state_2(ctx: Context<()>) -> VNode {
    let (val, set_val) = use_state(&ctx, || 0);

    let incr = || set_val(val + 1);
    let decr = || set_val(val - 1);

    todo!()
    // html! {
    //     <div>
    //         <nav class="menu">
    //             <button onclick=incr> { "Increment" } </button>
    //             <button onclick=decr> { "Decrement" } </button>
    //         </nav>
    //         <p> <b>{ "Current value: {val}" }</b> </p>
    //     </div>
    // }
}
