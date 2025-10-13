//! An endpoint represents an entrypoint for a group of server functions.

pub struct Endpoint<T> {
    path: &'static str,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Endpoint<T> {
    /// Create a new endpoint at the given path.
    pub const fn new(path: &'static str, f: fn() -> T) -> Self {
        Self {
            path,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: 'static> std::ops::Deref for Endpoint<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}
