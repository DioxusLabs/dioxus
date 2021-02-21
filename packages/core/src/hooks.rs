//! Useful, foundational hooks that 3rd parties can implement.
//! Currently implemented:
//! - [x] use_ref
//! - [x] use_state
//! - [ ] use_reducer
//! - [ ] use_effect

pub use use_ref_def::use_ref;
pub use use_state_def::use_state;

mod use_state_def {
    use crate::innerlude::*;
    use std::{cell::RefCell, ops::DerefMut, rc::Rc};

    struct UseState<T: 'static> {
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
    pub fn use_state<'a, 'c, T: 'static, F: FnOnce() -> T>(
        ctx: &'c Context<'a>,
        initial_state_fn: F,
    ) -> (&'a T, &'a impl Fn(T)) {
        ctx.use_hook(
            move || UseState {
                new_val: Rc::new(RefCell::new(None)),
                current_val: initial_state_fn(),
                caller: Box::new(|_| println!("setter called!")),
            },
            move |hook| {
                let inner = hook.new_val.clone();
                let scheduled_update = ctx.schedule_update();

                // get ownership of the new val and replace the current with the new
                // -> as_ref -> borrow_mut -> deref_mut -> take
                // -> rc     -> &RefCell   -> RefMut    -> &Option<T> -> T
                if let Some(new_val) = hook.new_val.as_ref().borrow_mut().deref_mut().take() {
                    hook.current_val = new_val;
                }

                // todo: swap out the caller with a subscription call and an internal update
                hook.caller = Box::new(move |new_val| {
                    // update the setter with the new value
                    let mut new_inner = inner.as_ref().borrow_mut();
                    *new_inner = Some(new_val);

                    // Ensure the component gets updated
                    scheduled_update();
                });

                // box gets derefed into a ref which is then taken as ref with the hook
                (&hook.current_val, &hook.caller)
            },
            |_| {},
        )
    }
}

mod use_ref_def {
    use crate::innerlude::*;
    use std::{cell::RefCell, ops::DerefMut};

    pub struct UseRef<T: 'static> {
        _current: RefCell<T>,
    }
    impl<T: 'static> UseRef<T> {
        fn new(val: T) -> Self {
            Self {
                _current: RefCell::new(val),
            }
        }

        pub fn modify(&self, modifier: impl FnOnce(&mut T)) {
            let mut val = self._current.borrow_mut();
            let val_as_ref = val.deref_mut();
            modifier(val_as_ref);
        }

        pub fn current(&self) -> std::cell::Ref<'_, T> {
            self._current.borrow()
        }
    }

    /// Store a mutable value between renders!
    /// To read the value, borrow the ref.
    /// To change it, use modify.
    /// Modifications to this value do not cause updates to the component
    /// Attach to inner context reference, so context can be consumed
    pub fn use_ref<'a, T: 'static>(
        ctx: &'_ Context<'a>,
        initial_state_fn: impl FnOnce() -> T + 'static,
    ) -> &'a UseRef<T> {
        ctx.use_hook(|| UseRef::new(initial_state_fn()), |state| &*state, |_| {})
    }
}
