use std::fmt::Debug;

/// The basic content type of dioxus-router-core.
///
/// For actual route definitions this type is basically useless. However, it allows the router to
/// support convenience [`From`] implementations which can tell content and redirects apart.
///
/// ```rust
/// # use dioxus_router_core::routes::ContentAtom;
/// let content = ContentAtom("some content");
/// ```
#[derive(Clone)]
pub struct ContentAtom<T>(pub T)
where
    T: Clone;

impl<T: Clone + Debug> Debug for ContentAtom<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ContentAtom").field(&self.0).finish()
    }
}

impl<T: Clone + PartialEq> PartialEq for ContentAtom<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Clone + Eq> Eq for ContentAtom<T> {}
