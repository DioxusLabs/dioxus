use crate::{global_context::current_scope_id, properties::SuperFrom, Runtime, ScopeId};
use generational_box::GenerationalBox;
use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    rc::Rc,
};

/// A wrapper around some generic data that handles the event's state
///
///
/// Prevent this event from continuing to bubble up the tree to parent elements.
///
/// # Example
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// rsx! {
///     button {
///         onclick: move |evt: Event<MouseData>| {
///             evt.stop_propagation();
///         }
///     }
/// };
/// ```
pub struct Event<T: 'static + ?Sized> {
    /// The data associated with this event
    pub data: Rc<T>,
    pub(crate) propagates: Rc<Cell<bool>>,
}

impl<T: ?Sized + 'static> Event<T> {
    pub(crate) fn new(data: Rc<T>, bubbles: bool) -> Self {
        Self {
            data,
            propagates: Rc::new(Cell::new(bubbles)),
        }
    }
}

impl<T> Event<T> {
    /// Map the event data to a new type
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// rsx! {
    ///    button {
    ///       onclick: move |evt: MouseEvent| {
    ///          let data = evt.map(|data| data.client_coordinates());
    ///          println!("{:?}", data.data());
    ///       }
    ///    }
    /// };
    /// ```
    pub fn map<U: 'static, F: FnOnce(&T) -> U>(&self, f: F) -> Event<U> {
        Event {
            data: Rc::new(f(&self.data)),
            propagates: self.propagates.clone(),
        }
    }

    /// Prevent this event from continuing to bubble up the tree to parent elements.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// rsx! {
    ///     button {
    ///         onclick: move |evt: Event<MouseData>| {
    ///             # #[allow(deprecated)]
    ///             evt.cancel_bubble();
    ///         }
    ///     }
    /// };
    /// ```
    #[deprecated = "use stop_propagation instead"]
    pub fn cancel_bubble(&self) {
        self.propagates.set(false);
    }

    /// Prevent this event from continuing to bubble up the tree to parent elements.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// rsx! {
    ///     button {
    ///         onclick: move |evt: Event<MouseData>| {
    ///             evt.stop_propagation();
    ///         }
    ///     }
    /// };
    /// ```
    pub fn stop_propagation(&self) {
        self.propagates.set(false);
    }

    /// Get a reference to the inner data from this event
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// rsx! {
    ///     button {
    ///         onclick: move |evt: Event<MouseData>| {
    ///             let data = evt.data();
    ///             async move {
    ///                 println!("{:?}", data);
    ///             }
    ///         }
    ///     }
    /// };
    /// ```
    pub fn data(&self) -> Rc<T> {
        self.data.clone()
    }
}

impl<T: ?Sized> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self {
            propagates: self.propagates.clone(),
            data: self.data.clone(),
        }
    }
}

impl<T> std::ops::Deref for Event<T> {
    type Target = Rc<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiEvent")
            .field("bubble_state", &self.propagates)
            .field("data", &self.data)
            .finish()
    }
}

/// The callback type generated by the `rsx!` macro when an `on` field is specified for components.
///
/// This makes it possible to pass `move |evt| {}` style closures into components as property fields.
///
/// # Example
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// rsx!{
///     MyComponent { onclick: move |evt| tracing::debug!("clicked") }
/// };
///
/// #[derive(Props, Clone, PartialEq)]
/// struct MyProps {
///     onclick: EventHandler<MouseEvent>,
/// }
///
/// fn MyComponent(cx: MyProps) -> Element {
///     rsx!{
///         button {
///             onclick: move |evt| cx.onclick.call(evt),
///         }
///     }
/// }
/// ```
pub type EventHandler<T = ()> = Callback<T>;

/// The callback type generated by the `rsx!` macro when an `on` field is specified for components.
///
/// This makes it possible to pass `move |evt| {}` style closures into components as property fields.
///
///
/// # Example
///
/// ```rust, ignore
/// rsx!{
///     MyComponent { onclick: move |evt| {
///         tracing::debug!("clicked");
///         42
///     } }
/// }
///
/// #[derive(Props)]
/// struct MyProps {
///     onclick: Callback<MouseEvent, i32>,
/// }
///
/// fn MyComponent(cx: MyProps) -> Element {
///     rsx!{
///         button {
///             onclick: move |evt| println!("number: {}", cx.onclick.call(evt)),
///         }
///     }
/// }
/// ```
pub struct Callback<Args = (), Ret = ()> {
    pub(crate) origin: ScopeId,
    /// During diffing components with EventHandler, we move the EventHandler over in place instead of rerunning the child component.
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// #[component]
    /// fn Child(onclick: EventHandler<MouseEvent>) -> Element {
    ///     rsx!{
    ///         button {
    ///             // Diffing Child will not rerun this component, it will just update the callback in place so that if this callback is called, it will run the latest version of the callback
    ///             onclick: move |evt| onclick(evt),
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// This is both more efficient and allows us to avoid out of date EventHandlers.
    ///
    /// We double box here because we want the data to be copy (GenerationalBox) and still update in place (ExternalListenerCallback)
    /// This isn't an ideal solution for performance, but it is non-breaking and fixes the issues described in <https://github.com/DioxusLabs/dioxus/pull/2298>
    pub(super) callback: GenerationalBox<Option<ExternalListenerCallback<Args, Ret>>>,
}

impl<Args, Ret> std::fmt::Debug for Callback<Args, Ret> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Callback")
            .field("origin", &self.origin)
            .field("callback", &self.callback)
            .finish()
    }
}

impl<T: 'static, Ret: Default + 'static> Default for Callback<T, Ret> {
    fn default() -> Self {
        Callback::new(|_| Ret::default())
    }
}

/// A helper trait for [`Callback`]s that allows functions to accept a [`Callback`] that may return an async block which will automatically be spawned.
///
/// ```rust, no_run
/// use dioxus::prelude::*;
/// fn accepts_fn<Ret: dioxus_core::SpawnIfAsync<Marker>, Marker>(callback: impl FnMut(u32) -> Ret + 'static) {
///     let callback = Callback::new(callback);
/// }
/// // You can accept both async and non-async functions
/// accepts_fn(|x| async move { println!("{}", x) });
/// accepts_fn(|x| println!("{}", x));
/// ```
pub trait SpawnIfAsync<Marker, Ret = ()>: Sized {
    /// Spawn the value into the dioxus runtime if it is an async block
    fn spawn(self) -> Ret;
}

// Support for FnMut -> Ret for any return type
impl<Ret> SpawnIfAsync<(), Ret> for Ret {
    fn spawn(self) -> Ret {
        self
    }
}

// Support for FnMut -> async { unit } for the unit return type
#[doc(hidden)]
pub struct AsyncMarker;
impl<F: std::future::Future<Output = ()> + 'static> SpawnIfAsync<AsyncMarker> for F {
    fn spawn(self) {
        crate::prelude::spawn(async move {
            self.await;
        });
    }
}

// Support for FnMut -> async { Result(()) } for the unit return type
#[doc(hidden)]
pub struct AsyncResultMarker;

impl<T> SpawnIfAsync<AsyncResultMarker> for T
where
    T: std::future::Future<Output = crate::Result<()>> + 'static,
{
    #[inline]
    fn spawn(self) {
        crate::prelude::spawn(async move {
            if let Err(err) = self.await {
                crate::prelude::throw_error(err)
            }
        });
    }
}

// Support for FnMut -> Result(anything) for the unit return type
impl SpawnIfAsync<()> for crate::Result<()> {
    #[inline]
    fn spawn(self) {
        if let Err(err) = self {
            crate::prelude::throw_error(err)
        }
    }
}

// We can't directly forward the marker because it would overlap with a bunch of other impls, so we wrap it in another type instead
#[doc(hidden)]
pub struct MarkerWrapper<T>(PhantomData<T>);

// Closure can be created from FnMut -> async { anything } or FnMut -> Ret
impl<
        Function: FnMut(Args) -> Spawn + 'static,
        Args: 'static,
        Spawn: SpawnIfAsync<Marker, Ret> + 'static,
        Ret: 'static,
        Marker,
    > SuperFrom<Function, MarkerWrapper<Marker>> for Callback<Args, Ret>
{
    fn super_from(input: Function) -> Self {
        Callback::new(input)
    }
}

#[doc(hidden)]
pub struct UnitClosure<Marker>(PhantomData<Marker>);

// Closure can be created from FnMut -> async { anything } or FnMut -> Ret
impl<
        Function: FnMut() -> Spawn + 'static,
        Spawn: SpawnIfAsync<Marker, Ret> + 'static,
        Ret: 'static,
        Marker,
    > SuperFrom<Function, UnitClosure<Marker>> for Callback<(), Ret>
{
    fn super_from(mut input: Function) -> Self {
        Callback::new(move |()| input())
    }
}

#[test]
fn closure_types_infer() {
    #[allow(unused)]
    fn compile_checks() {
        // You should be able to use a closure as a callback
        let callback: Callback<(), ()> = Callback::new(|_| {});
        // Or an async closure
        let callback: Callback<(), ()> = Callback::new(|_| async {});

        // You can also pass in a closure that returns a value
        let callback: Callback<(), u32> = Callback::new(|_| 123);

        // Or pass in a value
        let callback: Callback<u32, ()> = Callback::new(|value: u32| async move {
            println!("{}", value);
        });

        // Unit closures shouldn't require an argument
        let callback: Callback<(), ()> = Callback::super_from(|| async move {
            println!("hello world");
        });
    }
}

impl<Args, Ret> Copy for Callback<Args, Ret> {}

impl<Args, Ret> Clone for Callback<Args, Ret> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Args: 'static, Ret: 'static> PartialEq for Callback<Args, Ret> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

type ExternalListenerCallback<Args, Ret> = Rc<RefCell<dyn FnMut(Args) -> Ret>>;

impl<Args: 'static, Ret: 'static> Callback<Args, Ret> {
    /// Create a new [`Callback`] from an [`FnMut`]. The callback is owned by the current scope and will be dropped when the scope is dropped.
    /// This should not be called directly in the body of a component because it will not be dropped until the component is dropped.
    #[track_caller]
    pub fn new<MaybeAsync: SpawnIfAsync<Marker, Ret>, Marker>(
        mut f: impl FnMut(Args) -> MaybeAsync + 'static,
    ) -> Self {
        let owner = crate::innerlude::current_owner::<generational_box::UnsyncStorage>();
        let callback = owner.insert(Some(
            Rc::new(RefCell::new(move |event: Args| f(event).spawn()))
                as Rc<RefCell<dyn FnMut(Args) -> Ret>>,
        ));
        Self {
            callback,
            origin: current_scope_id().expect("to be in a dioxus runtime"),
        }
    }

    /// Leak a new [`Callback`] that will not be dropped unless it is manually dropped.
    #[track_caller]
    pub fn leak(mut f: impl FnMut(Args) -> Ret + 'static) -> Self {
        let callback =
            GenerationalBox::leak(Some(Rc::new(RefCell::new(move |event: Args| f(event)))
                as Rc<RefCell<dyn FnMut(Args) -> Ret>>));
        Self {
            callback,
            origin: current_scope_id().expect("to be in a dioxus runtime"),
        }
    }

    /// Call this callback with the appropriate argument type
    ///
    /// This borrows the callback using a RefCell. Recursively calling a callback will cause a panic.
    pub fn call(&self, arguments: Args) -> Ret {
        if let Some(callback) = self.callback.read().as_ref() {
            Runtime::with(|rt| rt.scope_stack.borrow_mut().push(self.origin));
            let value = {
                let mut callback = callback.borrow_mut();
                callback(arguments)
            };
            Runtime::with(|rt| rt.scope_stack.borrow_mut().pop());
            value
        } else {
            panic!("Callback was manually dropped")
        }
    }

    /// Create a `impl FnMut + Copy` closure from the Closure type
    pub fn into_closure(self) -> impl FnMut(Args) -> Ret + Copy + 'static {
        move |args| self.call(args)
    }

    /// Forcibly drop the internal handler callback, releasing memory
    ///
    /// This will force any future calls to "call" to not doing anything
    pub fn release(&self) {
        self.callback.set(None);
    }

    #[doc(hidden)]
    /// This should only be used by the `rsx!` macro.
    pub fn __set(&mut self, value: ExternalListenerCallback<Args, Ret>) {
        self.callback.set(Some(value));
    }

    #[doc(hidden)]
    /// This should only be used by the `rsx!` macro.
    pub fn __take(&self) -> ExternalListenerCallback<Args, Ret> {
        self.callback
            .read()
            .clone()
            .expect("Callback was manually dropped")
    }
}

impl<Args: 'static, Ret: 'static> std::ops::Deref for Callback<Args, Ret> {
    type Target = dyn Fn(Args) -> Ret + 'static;

    fn deref(&self) -> &Self::Target {
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = std::mem::MaybeUninit::<Self>::uninit();
        // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
        let uninit_closure = move |t| Self::call(unsafe { &*uninit_callable.as_ptr() }, t);

        // Check that the size of the closure is the same as the size of Self in case the compiler changed the layout of the closure.
        let size_of_closure = std::mem::size_of_val(&uninit_closure);
        assert_eq!(size_of_closure, std::mem::size_of::<Self>());

        // Then cast the lifetime of the closure to the lifetime of &self.
        fn cast_lifetime<'a, T>(_a: &T, b: &'a T) -> &'a T {
            b
        }
        let reference_to_closure = cast_lifetime(
            {
                // The real closure that we will never use.
                &uninit_closure
            },
            // We transmute self into a reference to the closure. This is safe because we know that the closure has the same memory layout as Self so &Closure == &Self.
            unsafe { std::mem::transmute(self) },
        );

        // Cast the closure to a trait object.
        reference_to_closure as &_
    }
}
