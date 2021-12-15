//! When building complex components, it's occasionally useful to dip into a pure MVC pattern instead of the
//! React hooks pattern. Hooks are useful to abstract over some reusable logic, but many models are not reusable
//! in the same way that hooks are.
//!
//! In these cases, we provide `use_model` - a convenient way of abstracting over some state and async functions.

use dioxus_core::prelude::ScopeState;
use futures::Future;
use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
};

pub fn use_model<'a, T: 'static>(cx: &'a ScopeState, f: impl FnOnce() -> T) -> UseModel<'a, T> {
    cx.use_hook(
        |_| UseModelInner {
            update_scheduled: Cell::new(false),
            update_callback: cx.schedule_update(),
            value: RefCell::new(f()),
            // tasks: RefCell::new(Vec::new()),
        },
        |inner| {
            inner.update_scheduled.set(false);
            UseModel { inner }
        },
    )
}

pub struct UseModel<'a, T> {
    inner: &'a UseModelInner<T>,
}

struct UseModelInner<T> {
    update_scheduled: Cell<bool>,
    update_callback: Rc<dyn Fn()>,
    value: RefCell<T>,
    // tasks: RefCell<Vec<ModelTask>>,
}

type ModelTask = Pin<Box<dyn Future<Output = ()> + 'static>>;

impl<'a, T: 'static> UseModel<'a, T> {
    pub fn read(&self) -> Ref<'_, T> {
        self.inner.value.borrow()
    }
    pub fn write(&self) -> RefMut<'_, T> {
        self.needs_update();
        self.inner.value.borrow_mut()
    }
    /// Allows the ability to write the value without forcing a re-render
    pub fn write_silent(&self) -> RefMut<'_, T> {
        self.inner.value.borrow_mut()
    }

    pub fn needs_update(&self) {
        if !self.inner.update_scheduled.get() {
            self.inner.update_scheduled.set(true);
            (self.inner.update_callback)();
        }
    }

    pub fn set(&self, new: T) {
        *self.inner.value.borrow_mut() = new;
        self.needs_update();
    }

    pub fn read_write(&self) -> (Ref<'_, T>, &Self) {
        (self.read(), self)
    }

    pub fn start(&self, _f: impl FnOnce() -> ModelTask) {
        todo!()
    }
}

// keep a coroutine going
pub fn use_model_coroutine<'a, T, F: Future<Output = ()> + 'static>(
    cx: &'a ScopeState,
    _model: UseModel<T>,
    _f: impl FnOnce(AppModels) -> F,
) -> UseModelCoroutine {
    cx.use_hook(
        |_| {
            //
            UseModelTaskInner {
                task: Default::default(),
            }
        },
        |inner| {
            if let Some(task) = inner.task.get_mut() {
                cx.push_task(|| task);
            }
            //
            todo!()
        },
    )
}

pub struct UseModelCoroutine {}

struct UseModelTaskInner {
    task: RefCell<Option<ModelTask>>,
}

impl UseModelCoroutine {
    pub fn start(&self) {}
}

pub struct ModelAsync<T> {
    _p: PhantomData<T>,
}
impl<T> ModelAsync<T> {
    pub fn write(&self) -> RefMut<'_, T> {
        todo!()
    }
    pub fn read(&self) -> Ref<'_, T> {
        todo!()
    }
}

pub struct AppModels {}

impl AppModels {
    pub fn get<T: 'static>(&self) -> ModelAsync<T> {
        unimplemented!()
    }
}

impl<T> Copy for UseModel<'_, T> {}
impl<'a, T> Clone for UseModel<'a, T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner }
    }
}
