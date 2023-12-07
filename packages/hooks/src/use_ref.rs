use dioxus_core::ScopeState;
use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    rc::Rc,
    sync::Arc,
};

/// `use_ref` is a key foundational hook for storing state in Dioxus.
///
/// It is different that `use_state` in that the value stored is not "immutable".
/// Instead, UseRef is designed to store larger values that will be mutated at will.
///
/// ## Writing Values
///
/// Generally, `use_ref` is just a wrapper around a RefCell that tracks mutable
/// writes through the `write` method. Whenever `write` is called, the component
/// that initialized the hook will be marked as "dirty".
///
/// ```rust, no_run
/// let val = use_ref(|| HashMap::<u32, String>::new());
///
/// // using `write` will give us a `RefMut` to the inner value, which we can call methods on
/// // This marks the component as "dirty"
/// val.write().insert(1, "hello".to_string());
/// ```
///
/// You can avoid this default behavior with `write_silent`
///
/// ```rust, no_run
/// // with `write_silent`, the component will not be re-rendered
/// val.write_silent().insert(2, "goodbye".to_string());
/// ```
///
/// ## Reading Values
///
/// To read values out of the refcell, you can use the `read` method which will retrun a `Ref`.
///
/// ```rust, no_run
/// let map: Ref<_> = val.read();
///
/// let item = map.get(&1);
/// ```
///
/// To get an &T out of the RefCell, you need to "reborrow" through the Ref:
///
/// ```rust, no_run
/// let read = val.read();
/// let map = &*read;
/// ```
///
/// ## Collections and iteration
///
/// A common usecase for `use_ref` is to store a large amount of data in a component.
/// Typically this will be a collection like a HashMap or a Vec. To create new
/// elements from the collection, we can use `read()` directly in our rsx!.
///
/// ```rust, no_run
/// rsx!{
///     val.read().iter().map(|(k, v)| {
///         rsx!{ key: "{k}", "{v}" }
///     })
/// }
/// ```
///
/// If you are generating elements outside of `rsx!` then you might need to call
/// "render" inside the iterator. For some cases you might need to collect into
/// a temporary Vec.
///
/// ```rust, no_run
/// let items = val.read().iter().map(|(k, v)| {
///     cx.render(rsx!{ key: "{k}", "{v}" })
/// });
///
/// // collect into a Vec
///
/// let items: Vec<Element> = items.collect();
/// ```
///
/// ## Use in Async
///
/// To access values from a `UseRef` in an async context, you need to detach it
/// from the current scope's lifetime, making it a `'static` value. This is done
/// by simply calling `to_owned` or `clone`.
///
/// ```rust, no_run
/// let val = use_ref(|| HashMap::<u32, String>::new());
///
/// cx.spawn({
///     let val = val.clone();
///     async move {
///         some_work().await;
///         val.write().insert(1, "hello".to_string());
///     }
/// })
/// ```
///
/// If you're working with lots of values like UseState and UseRef, you can use the
/// `to_owned!` macro to make it easier to write the above code.
///
/// ```rust, no_run
/// let val1 = use_ref(|| HashMap::<u32, String>::new());
/// let val2 = use_ref(|| HashMap::<u32, String>::new());
/// let val3 = use_ref(|| HashMap::<u32, String>::new());
///
/// cx.spawn({
///     to_owned![val1, val2, val3];
///     async move {
///         some_work().await;
///         val.write().insert(1, "hello".to_string());
///     }
/// })
/// ```
#[must_use]
pub fn use_ref<T: 'static>(cx: &ScopeState, initialize_refcell: impl FnOnce() -> T) -> &UseRef<T> {
    let hook = cx.use_hook(|| UseRef {
        update: cx.schedule_update(),
        value: Rc::new(RefCell::new(initialize_refcell())),
        dirty: Rc::new(Cell::new(false)),
        gen: 0,
    });

    if hook.dirty.get() {
        hook.gen += 1;
        hook.dirty.set(false);
    }

    hook
}

/// A type created by the [`use_ref`] hook. See its documentation for more details.
pub struct UseRef<T> {
    update: Arc<dyn Fn()>,
    value: Rc<RefCell<T>>,
    dirty: Rc<Cell<bool>>,
    gen: usize,
}

impl<T> Clone for UseRef<T> {
    fn clone(&self) -> Self {
        Self {
            update: self.update.clone(),
            value: self.value.clone(),
            dirty: self.dirty.clone(),
            gen: self.gen,
        }
    }
}

impl<T> UseRef<T> {
    /// Read the value in the RefCell into a `Ref`. If this method is called
    /// while other values are still being `read` or `write`, then your app will crash.
    ///
    /// Be very careful when working with this method. If you can, consider using
    /// the `with` and `with_mut` methods instead, choosing to render Elements
    /// during the read calls.
    pub fn read(&self) -> Ref<'_, T> {
        self.value.borrow()
    }

    /// Mutably unlock the value in the RefCell. This will mark the component as "dirty"
    ///
    /// Uses to `write` should be as short as possible.
    ///
    /// Be very careful when working with this method. If you can, consider using
    /// the `with` and `with_mut` methods instead, choosing to render Elements
    /// during the read and write calls.
    pub fn write(&self) -> RefMut<'_, T> {
        self.needs_update();
        self.value.borrow_mut()
    }

    /// Set the curernt value to `new_value`. This will mark the component as "dirty"
    ///
    /// This change will propagate immediately, so any other contexts that are
    /// using this RefCell will also be affected. If called during an async context,
    /// the component will not be re-rendered until the next `.await` call.
    pub fn set(&self, new: T) {
        *self.value.borrow_mut() = new;
        self.needs_update();
    }

    /// Mutably unlock the value in the RefCell. This will not mark the component as dirty.
    /// This is useful if you want to do some work without causing the component to re-render.
    ///
    /// Uses to `write` should be as short as possible.
    ///
    /// Be very careful when working with this method. If you can, consider using
    /// the `with` and `with_mut` methods instead, choosing to render Elements
    pub fn write_silent(&self) -> RefMut<'_, T> {
        self.value.borrow_mut()
    }

    /// Take a reference to the inner value termporarily and produce a new value
    ///
    /// Note: You can always "reborrow" the value through the RefCell.
    /// This method just does it for you automatically.
    ///
    /// ```rust, no_run
    /// let val = use_ref(|| HashMap::<u32, String>::new());
    ///
    ///
    /// // use reborrowing
    /// let inner = &*val.read();
    ///
    /// // or, be safer and use `with`
    /// val.with(|i| println!("{:?}", i));
    /// ```
    pub fn with<O>(&self, immutable_callback: impl FnOnce(&T) -> O) -> O {
        immutable_callback(&*self.read())
    }

    /// Take a reference to the inner value termporarily and produce a new value,
    /// modifying the original in place.
    ///
    /// Note: You can always "reborrow" the value through the RefCell.
    /// This method just does it for you automatically.
    ///
    /// ```rust, no_run
    /// let val = use_ref(|| HashMap::<u32, String>::new());
    ///
    ///
    /// // use reborrowing
    /// let inner = &mut *val.write();
    ///
    /// // or, be safer and use `with`
    /// val.with_mut(|i| i.insert(1, "hi"));
    /// ```
    pub fn with_mut<O>(&self, mutable_callback: impl FnOnce(&mut T) -> O) -> O {
        mutable_callback(&mut *self.write())
    }

    /// Call the inner callback to mark the originator component as dirty.
    ///
    /// This will cause the component to be re-rendered after the current scope
    /// has ended or the current async task has been yielded through await.
    pub fn needs_update(&self) {
        self.dirty.set(true);
        (self.update)();
    }
}

// UseRef memoizes not on value but on cell
// Memoizes on "generation" - so it will cause a re-render if the value changes
impl<T> PartialEq for UseRef<T> {
    fn eq(&self, other: &Self) -> bool {
        if Rc::ptr_eq(&self.value, &other.value) {
            self.gen == other.gen
        } else {
            false
        }
    }
}
