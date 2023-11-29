use dioxus_core::ScopeState;
use dioxus_signals::CopyValue;
use std::sync::Arc;

#[must_use]
pub fn use_signal<T>(cx: &ScopeState, initialize_refcell: impl FnOnce() -> T) -> &UseSignal<T> {
    let hook = cx.use_hook(|| UseSignal {
        update: cx.schedule_update(),
        value: CopyValue::new(initialize_refcell()),
        dirty: CopyValue::new(false),
        gen: 0,
    });

    // if hook.dirty.get() {
    //     hook.gen += 1;
    //     hook.dirty.set(false);
    // }

    hook
}

pub struct UseSignal<T: 'static> {
    update: Arc<dyn Fn()>,
    value: CopyValue<T>,
    dirty: CopyValue<bool>,
    gen: usize,
}

impl<T> Clone for UseSignal<T> {
    fn clone(&self) -> Self {
        Self {
            update: self.update.clone(),
            value: self.value.clone(),
            dirty: self.dirty.clone(),
            gen: self.gen,
        }
    }
}

impl<T> UseSignal<T> {
    pub fn read(&self) -> CopyValue<T> {
        self.value
    }

    // pub fn write(&self) -> CopyValue<T> {
    //     self.write_silent()
    // }

    // pub fn set(&self, new: T) {
    //     *self.value.borrow_mut() = new;
    //     self.needs_update();
    // }

    // pub fn write_silent(&self) -> CopyValue<T> {
    //     self.value.borrow_mut()
    // }

    // pub fn with<O>(&self, immutable_callback: impl FnOnce(&T) -> O) -> O {
    //     immutable_callback(&*self.read())
    // }

    // pub fn with_mut<O>(&self, mutable_callback: impl FnOnce(&mut T) -> O) -> O {
    //     mutable_callback(&mut *self.write())
    // }

    // pub fn needs_update(&self) {
    //     self.dirty.set(true);
    //     (self.update)();
    // }
}

// impl<T> PartialEq for UseSignal<T> {
//     fn eq(&self, other: &Self) -> bool {
//         if Rc::ptr_eq(&self.value, &other.value) {
//             self.gen == other.gen
//         } else {
//             false
//         }
//     }
// }
