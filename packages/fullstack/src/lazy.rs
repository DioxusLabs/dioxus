#![allow(clippy::needless_return)]

use std::{hint::black_box, prelude::rust_2024::Future, sync::atomic::AtomicBool};

/// `Lazy` is a thread-safe, lazily-initialized global variable.
///
/// Unlike other async once-cell implementations, accessing the value of a `Lazy` instance is synchronous
/// and done on `deref`.
///
/// This is done by offloading the async initialization to a blocking thread during the first access,
/// and then using the initialized value for all subsequent accesses.
///
/// It uses `std::sync::OnceLock` internally to ensure that the value is only initialized once.
pub struct Lazy<T> {
    value: std::sync::OnceLock<T>,
    started_initialization: AtomicBool,
    constructor: Option<fn() -> Result<T, dioxus_core::Error>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static> Lazy<T> {
    /// Create a new `Lazy` instance.
    ///
    /// This internally calls `std::sync::OnceLock::new()` under the hood.
    #[allow(clippy::self_named_constructors)]
    pub const fn lazy() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            constructor: None,
            started_initialization: AtomicBool::new(false),
            value: std::sync::OnceLock::new(),
        }
    }

    pub const fn new<F, G, E>(constructor: F) -> Self
    where
        F: Fn() -> G + Copy,
        G: Future<Output = Result<T, E>> + Send + 'static,
        E: Into<dioxus_core::Error>,
    {
        if std::mem::size_of::<F>() != 0 {
            panic!("The constructor function must be a zero-sized type (ZST). Consider using a function pointer or a closure without captured variables.");
        }

        // Prevent the constructor from being optimized out
        black_box(constructor);

        Self {
            _phantom: std::marker::PhantomData,
            value: std::sync::OnceLock::new(),
            started_initialization: AtomicBool::new(false),
            constructor: Some(blocking_initialize::<T, F, G, E>),
        }
    }

    /// Set the value of the `Lazy` instance.
    ///
    /// This should only be called once during the server setup phase, typically inside `dioxus::serve`.
    /// Future calls to this method will return an error containing the provided value.
    pub fn set(&self, pool: T) -> Result<(), dioxus_core::Error> {
        let res = self.value.set(pool);
        if res.is_err() {
            return Err(anyhow::anyhow!("Lazy value is already initialized."));
        }

        Ok(())
    }

    pub fn try_set(&self, pool: T) -> Result<(), T> {
        self.value.set(pool)
    }

    /// Initialize the value of the `Lazy` instance if it hasn't been initialized yet.
    pub fn initialize(&self) -> Result<(), dioxus_core::Error> {
        if let Some(constructor) = self.constructor {
            // If we're already initializing this value, wait on the receiver.
            if self
                .started_initialization
                .swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                self.value.wait();
                return Ok(());
            }

            // Otherwise, we need to initialize the value
            self.set(constructor().unwrap())?;
        }
        Ok(())
    }
}

impl<T: Send + Sync + 'static> Default for Lazy<T> {
    fn default() -> Self {
        Self::lazy()
    }
}

impl<T: Send + Sync + 'static> std::ops::Deref for Lazy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        if self.constructor.is_none() {
            return self.value.get().expect("Lazy value is not initialized. Make sure to call `initialize` before dereferencing.");
        };

        if self.value.get().is_none() {
            self.initialize().expect("Failed to initialize lazy value");
        }

        self.value.get().unwrap()
    }
}

/// This is a small hack that allows us to staple the async initialization into a blocking context.
///
/// We call the `rust-call` method of the zero-sized constructor function. This is safe because we're
/// not actually dereferencing any unsafe data, just calling its vtable entry to get the future.
fn blocking_initialize<T, F, G, E>() -> Result<T, dioxus_core::Error>
where
    T: Send + Sync + 'static,
    F: Fn() -> G + Copy,
    G: Future<Output = Result<T, E>> + Send + 'static,
    E: Into<dioxus_core::Error>,
{
    assert_eq!(std::mem::size_of::<F>(), 0, "The constructor function must be a zero-sized type (ZST). Consider using a function pointer or a closure without captured variables.");

    #[cfg(feature = "server")]
    {
        let ptr: F = unsafe { std::mem::zeroed() };
        let fut = ptr();
        let rt = tokio::runtime::Handle::current();
        return std::thread::spawn(move || rt.block_on(fut).map_err(|e| e.into()))
            .join()
            .unwrap();
    }

    // todo: technically we can support constructors in wasm with the same tricks inventory uses with `__wasm_call_ctors`
    // the host would need to decide when to cal the ctors and when to block them.
    #[cfg(not(feature = "server"))]
    unimplemented!("Lazy initialization is only supported with tokio and threads enabled.")
}
