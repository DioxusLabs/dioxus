/// `Lazy` is a thread-safe, lazily-initialized global variable.
///
/// It uses `std::sync::OnceLock`` internally to ensure that the value is only initialized once.
pub struct Lazy<T> {
    value: std::sync::OnceLock<T>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static> Lazy<T> {
    /// Create a new `Lazy` instance.
    ///
    /// This internally calls `std::sync::OnceLock::new()` under the hood.
    pub const fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            value: std::sync::OnceLock::new(),
        }
    }

    /// Set the value of the `Lazy` instance.
    ///
    /// This should only be called once during the server setup phase, typically inside `dioxus::serve`.
    /// Future calls to this method will return an error containing the provided value.
    pub fn set(&self, pool: T) -> Result<(), T> {
        self.value.set(pool)
    }
}

impl<T> std::ops::Deref for Lazy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.get().expect("Lazy value must be initialized before use. Make sure to call `.set()` in `dioxus::serve` before using the value.")
    }
}

impl<T: Send + Sync + 'static> Default for Lazy<T> {
    fn default() -> Self {
        Self::new()
    }
}
