use crate::{current_scope_id, properties::SuperFrom, runtime::RuntimeGuard, Runtime, ScopeId};
use futures_util::FutureExt;
use generational_box::GenerationalBox;
use std::{any::Any, cell::RefCell, marker::PhantomData, panic::Location, rc::Rc};

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
    pub(crate) metadata: Rc<RefCell<EventMetadata>>,
}

#[derive(Clone, Copy)]
pub(crate) struct EventMetadata {
    pub(crate) propagates: bool,
    pub(crate) prevent_default: bool,
}

impl<T: ?Sized + 'static> Event<T> {
    /// Create a new event from the inner data
    pub fn new(data: Rc<T>, propagates: bool) -> Self {
        Self {
            data,
            metadata: Rc::new(RefCell::new(EventMetadata {
                propagates,
                prevent_default: false,
            })),
        }
    }
}

impl<T: ?Sized> Event<T> {
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
            metadata: self.metadata.clone(),
        }
    }

    /// Convert this event into a boxed event with a dynamic type
    pub fn into_any(self) -> Event<dyn Any>
    where
        T: Sized,
    {
        Event {
            data: self.data as Rc<dyn Any>,
            metadata: self.metadata,
        }
    }

    /// Create a new event with different data but the same metadata.
    ///
    /// Unlike `map`, this takes an `Rc` directly, allowing you to share
    /// ownership of the data (e.g., for accessing it after the handler returns).
    pub fn with_data<U: 'static>(&self, data: Rc<U>) -> Event<U> {
        Event {
            data,
            metadata: self.metadata.clone(),
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
        self.metadata.borrow_mut().propagates = false;
    }

    /// Check if the event propagates up the tree to parent elements
    pub fn propagates(&self) -> bool {
        self.metadata.borrow().propagates
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
        self.metadata.borrow_mut().propagates = false;
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

    /// Prevent the default action of the event.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// fn App() -> Element {
    ///     rsx! {
    ///         a {
    ///             // You can prevent the default action of the event with `prevent_default`
    ///             onclick: move |event| {
    ///                 event.prevent_default();
    ///             },
    ///             href: "https://dioxuslabs.com",
    ///             "don't go to the link"
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// Note: This must be called synchronously when handling the event. Calling it after the event has been handled will have no effect.
    ///
    /// <div class="warning">
    ///
    /// This method is not available on the LiveView renderer because LiveView handles all events over a websocket which cannot block.
    ///
    /// </div>
    #[track_caller]
    pub fn prevent_default(&self) {
        self.metadata.borrow_mut().prevent_default = true;
    }

    /// Check if the default action of the event is enabled.
    pub fn default_action_enabled(&self) -> bool {
        !self.metadata.borrow().prevent_default
    }
}

impl<T: ?Sized> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
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
            .field("bubble_state", &self.propagates())
            .field("prevent_default", &!self.default_action_enabled())
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
/// rsx! {
///     MyComponent { onclick: move |evt| tracing::debug!("clicked") }
/// };
///
/// #[derive(Props, Clone, PartialEq)]
/// struct MyProps {
///     onclick: EventHandler<MouseEvent>,
/// }
///
/// fn MyComponent(cx: MyProps) -> Element {
///     rsx! {
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
/// rsx! {
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
///     rsx! {
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
    ///     rsx! {
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
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`SpawnIfAsync` is not implemented for `{Self}`",
        label = "Return Value",
        note = "Closures (or event handlers) in dioxus need to return either: nothing (the unit type `()`), or an async block that dioxus will automatically spawn",
        note = "You likely need to add a semicolon to the end of the event handler to make it return nothing",
    )
)]
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
        // Quick poll once to deal with things like prevent_default in the same tick
        let mut fut = Box::pin(self);
        let res = fut.as_mut().now_or_never();

        if res.is_none() {
            crate::spawn(async move {
                fut.await;
            });
        }
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
        // Quick poll once to deal with things like prevent_default in the same tick
        let mut fut = Box::pin(self);
        let res = fut.as_mut().now_or_never();

        if res.is_none() {
            crate::spawn(async move {
                if let Err(err) = fut.await {
                    crate::throw_error(err)
                }
            });
        }
    }
}

// Support for FnMut -> Result(()) for the unit return type
impl SpawnIfAsync<()> for crate::Result<()> {
    #[inline]
    fn spawn(self) {
        if let Err(err) = self {
            crate::throw_error(err)
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

impl<
        Function: FnMut(Event<T>) -> Spawn + 'static,
        T: 'static,
        Spawn: SpawnIfAsync<Marker> + 'static,
        Marker,
    > SuperFrom<Function, MarkerWrapper<Marker>> for ListenerCallback<T>
{
    fn super_from(input: Function) -> Self {
        ListenerCallback::new(input)
    }
}

// ListenerCallback<T> can be created from Callback<Event<T>>
impl<T: 'static> SuperFrom<Callback<Event<T>>> for ListenerCallback<T> {
    fn super_from(input: Callback<Event<T>>) -> Self {
        // https://github.com/rust-lang/rust-clippy/issues/15072
        #[allow(clippy::redundant_closure)]
        ListenerCallback::new(move |event| input(event))
    }
}

#[doc(hidden)]
pub struct UnitClosure<Marker>(PhantomData<Marker>);

// Closure can be created from FnMut -> async { () } or FnMut -> Ret
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
    fn eq(&self, other: &Self) -> bool {
        self.callback.ptr_eq(&other.callback) && self.origin == other.origin
    }
}

pub(super) struct ExternalListenerCallback<Args, Ret> {
    callback: Box<dyn FnMut(Args) -> Ret>,
    runtime: std::rc::Weak<Runtime>,
}

impl<Args: 'static, Ret: 'static> Callback<Args, Ret> {
    /// Create a new [`Callback`] from an [`FnMut`]. The callback is owned by the current scope and will be dropped when the scope is dropped.
    /// This should not be called directly in the body of a component because it will not be dropped until the component is dropped.
    #[track_caller]
    pub fn new<MaybeAsync: SpawnIfAsync<Marker, Ret>, Marker>(
        mut f: impl FnMut(Args) -> MaybeAsync + 'static,
    ) -> Self {
        let runtime = Runtime::current();
        let origin = runtime.current_scope_id();
        let owner = crate::innerlude::current_owner::<generational_box::UnsyncStorage>();
        let callback = owner.insert_rc(Some(ExternalListenerCallback {
            callback: Box::new(move |event: Args| f(event).spawn()),
            runtime: Rc::downgrade(&runtime),
        }));
        Self { callback, origin }
    }

    /// Leak a new [`Callback`] that will not be dropped unless it is manually dropped.
    #[track_caller]
    pub fn leak(mut f: impl FnMut(Args) -> Ret + 'static) -> Self {
        let runtime = Runtime::current();
        let origin = runtime.current_scope_id();
        let callback = GenerationalBox::leak_rc(
            Some(ExternalListenerCallback {
                callback: Box::new(move |event: Args| f(event).spawn()),
                runtime: Rc::downgrade(&runtime),
            }),
            Location::caller(),
        );
        Self { callback, origin }
    }

    /// Call this callback with the appropriate argument type
    ///
    /// This borrows the callback using a RefCell. Recursively calling a callback will cause a panic.
    #[track_caller]
    pub fn call(&self, arguments: Args) -> Ret {
        if let Some(callback) = self.callback.write().as_mut() {
            let runtime = callback
                .runtime
                .upgrade()
                .expect("Callback was called after the runtime was dropped");
            let _guard = RuntimeGuard::new(runtime.clone());
            runtime.with_scope_on_stack(self.origin, || (callback.callback)(arguments))
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

    /// Replace the function in the callback with a new one
    pub fn replace(&mut self, callback: Box<dyn FnMut(Args) -> Ret>) {
        let runtime = Runtime::current();
        self.callback.set(Some(ExternalListenerCallback {
            callback,
            runtime: Rc::downgrade(&runtime),
        }));
    }

    #[doc(hidden)]
    /// This should only be used by the `rsx!` macro.
    pub fn __point_to(&mut self, other: &Self) {
        self.callback.point_to(other.callback).unwrap();
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
            #[allow(clippy::missing_transmute_annotations)]
            // We transmute self into a reference to the closure. This is safe because we know that the closure has the same memory layout as Self so &Closure == &Self.
            unsafe {
                std::mem::transmute(self)
            },
        );

        // Cast the closure to a trait object.
        reference_to_closure as &_
    }
}

type AnyEventHandler = Rc<RefCell<dyn FnMut(Event<dyn Any>)>>;

/// An owned callback type used in [`AttributeValue::Listener`](crate::AttributeValue::Listener).
///
/// This is the type that powers the `on` attributes in the `rsx!` macro, allowing you to pass event
/// handlers to elements.
///
/// ```rust, ignore
/// rsx! {
///     button {
///         onclick: AttributeValue::Listener(ListenerCallback::new(move |evt: Event<MouseData>| {
///             // ...
///         }))
///     }
/// }
/// ```
pub struct ListenerCallback<T = ()> {
    pub(crate) origin: ScopeId,
    callback: AnyEventHandler,
    _marker: PhantomData<T>,
}

impl<T> Clone for ListenerCallback<T> {
    fn clone(&self) -> Self {
        Self {
            origin: self.origin,
            callback: self.callback.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> PartialEq for ListenerCallback<T> {
    fn eq(&self, other: &Self) -> bool {
        // We compare the pointers of the callbacks, since they are unique
        Rc::ptr_eq(&self.callback, &other.callback) && self.origin == other.origin
    }
}

impl<T> ListenerCallback<T> {
    /// Create a new [`ListenerCallback`] from a callback
    ///
    /// This is expected to be called within a runtime scope. Make sure a runtime is current before
    /// calling this method.
    pub fn new<MaybeAsync, Marker>(mut f: impl FnMut(Event<T>) -> MaybeAsync + 'static) -> Self
    where
        T: 'static,
        MaybeAsync: SpawnIfAsync<Marker>,
    {
        Self {
            origin: current_scope_id(),
            callback: Rc::new(RefCell::new(move |event: Event<dyn Any>| {
                let data = event.data.downcast::<T>().unwrap();
                f(Event {
                    metadata: event.metadata.clone(),
                    data,
                })
                .spawn();
            })),
            _marker: PhantomData,
        }
    }

    /// Create a new [`ListenerCallback`] from a raw callback that receives `Event<dyn Any>`.
    ///
    /// This is useful when you need custom downcast logic, such as handling multiple
    /// possible event data types.
    ///
    /// This is expected to be called within a runtime scope.
    pub fn new_raw(f: impl FnMut(Event<dyn Any>) + 'static) -> Self {
        Self {
            origin: current_scope_id(),
            callback: Rc::new(RefCell::new(f)),
            _marker: PhantomData,
        }
    }

    /// Call the callback with an event
    ///
    /// This is expected to be called within a runtime scope. Make sure a runtime is current before
    /// calling this method.
    pub fn call(&self, event: Event<dyn Any>) {
        Runtime::current().with_scope_on_stack(self.origin, || {
            (self.callback.borrow_mut())(event);
        });
    }

    /// Erase the type of the callback, allowing it to be used with any type of event
    pub fn erase(self) -> ListenerCallback {
        ListenerCallback {
            origin: self.origin,
            callback: self.callback,
            _marker: PhantomData,
        }
    }
}
