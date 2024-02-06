use std::str::FromStr;

use crate::routable::Routable;

use super::HistoryProvider;

/// A [`HistoryProvider`] that stores all navigation information in memory.
pub struct MemoryHistory<R: Routable> {
    current: R,
    history: Vec<R>,
    future: Vec<R>,
}

impl<R: Routable> MemoryHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    /// Create a [`MemoryHistory`] starting at `path`.
    ///
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// # use dioxus::prelude::*;
    /// # #[component]
    /// # fn Index() -> Element { None }
    /// # #[component]
    /// # fn OtherPage() -> Element { None }
    /// #[derive(Clone, Routable, Debug, PartialEq)]
    /// enum Route {
    ///     #[route("/")]
    ///     Index {},
    ///     #[route("/some-other-page")]
    ///     OtherPage {},
    /// }
    ///
    /// let mut history = MemoryHistory::<Route>::with_initial_path(Route::Index {});
    /// assert_eq!(history.current_route(), Route::Index {});
    /// assert_eq!(history.can_go_back(), false);
    /// ```
    pub fn with_initial_path(path: R) -> Self {
        Self {
            current: path,
            history: Vec::new(),
            future: Vec::new(),
        }
    }
}

impl<R: Routable> Default for MemoryHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self {
            current: "/".parse().unwrap_or_else(|err| {
                panic!("index route does not exist:\n{err}\n use MemoryHistory::with_initial_path to set a custom path")
            }),
            history: Vec::new(),
            future: Vec::new(),
        }
    }
}

impl<R: Routable> HistoryProvider<R> for MemoryHistory<R> {
    fn current_route(&self) -> R {
        self.current.clone()
    }

    fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

    fn go_back(&mut self) {
        if let Some(last) = self.history.pop() {
            let old = std::mem::replace(&mut self.current, last);
            self.future.push(old);
        }
    }

    fn can_go_forward(&self) -> bool {
        !self.future.is_empty()
    }

    fn go_forward(&mut self) {
        if let Some(next) = self.future.pop() {
            let old = std::mem::replace(&mut self.current, next);
            self.history.push(old);
        }
    }

    fn push(&mut self, new: R) {
        // don't push the same route twice
        if self.current.to_string() == new.to_string() {
            return;
        }
        let old = std::mem::replace(&mut self.current, new);
        self.history.push(old);
        self.future.clear();
    }

    fn replace(&mut self, path: R) {
        self.current = path;
    }
}
