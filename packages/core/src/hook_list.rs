use std::{
    any::Any,
    cell::{Cell, RefCell, UnsafeCell},
};

#[derive(Default)]
pub(crate) struct HookList {
    pub(crate) hooks: RefCell<Vec<Box<UnsafeCell<dyn Any>>>>,
    pub(crate) hook_idx: Cell<usize>,
}

static BORROW_ERR: &str = r#"The hook list is already borrowed.
This error is likely caused by trying to use a hook inside a hook which violates the rules of hooks."#;

impl HookList {
    /// Reset the hook index back to the original state, but don't clear the hooks
    pub(crate) fn new_render(&self) {
        self.hook_idx.set(0);
    }

    /// Clear the hook list and reset it back to the initial state
    pub(crate) fn clear(&self) {
        self.hooks.borrow_mut().clear();
        self.hook_idx.set(0);
    }

    pub(crate) fn use_hook<State: 'static>(
        &self,
        initializer: impl FnOnce() -> State,
    ) -> &mut State {
        let cur_hook = self.hook_idx.get();

        let mut hooks = self.hooks.try_borrow_mut().expect(BORROW_ERR);

        if cur_hook >= hooks.len() {
            hooks.push(Box::new(UnsafeCell::new(initializer())));
        }

        hooks
            .get(cur_hook)
            .and_then(|inn| {
                self.hook_idx.set(cur_hook + 1);
                let raw_ref = unsafe { &mut *inn.get() };
                raw_ref.downcast_mut::<State>()
            })
            .expect(
                r###"
                Unable to retrieve the hook that was initialized at this index.
                Consult the `rules of hooks` to understand how to use hooks properly.

                You likely used the hook in a conditional. Hooks rely on consistent ordering between renders.
                Functions prefixed with "use" should never be called conditionally.
                "###,
            )
    }
}
