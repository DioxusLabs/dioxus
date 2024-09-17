use crate::routable::Routable;

/// A [`HistoryProvider`] that stores all navigation information in memory.
pub struct MemoryHistory<R: Routable> {
    current: R,
    history: Vec<R>,
    future: Vec<R>,
}

impl<R: Routable> MemoryHistory<R> {
    /// Create a [`MemoryHistory`] starting at `path`.
    ///
    /// ```rust
    /// # use dioxus_router::prelude::*;
    /// # use dioxus::prelude::*;
    /// # #[component]
    /// # fn Index() -> Element { VNode::empty() }
    /// # #[component]
    /// # fn OtherPage() -> Element { VNode::empty() }
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

    pub fn current_route(&self) -> R {
        self.current.clone()
    }

    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn go_back(&mut self) {
        if let Some(last) = self.history.pop() {
            let old = std::mem::replace(&mut self.current, last);
            self.future.push(old);
        }
    }

    pub fn can_go_forward(&self) -> bool {
        !self.future.is_empty()
    }

    pub fn go_forward(&mut self) {
        if let Some(next) = self.future.pop() {
            let old = std::mem::replace(&mut self.current, next);
            self.history.push(old);
        }
    }

    pub fn push(&mut self, new: R) {
        // don't push the same route twice
        if self.current.serialize() == new.serialize() {
            return;
        }
        let old = std::mem::replace(&mut self.current, new);
        self.history.push(old);
        self.future.clear();
    }

    pub fn replace(&mut self, path: R) {
        self.current = path;
    }
}

impl<R: Routable> Default for MemoryHistory<R> {
    fn default() -> Self {
        Self {
            current: R::deserialize("/").unwrap_or_else(|err| {
                panic!("index route does not exist:\n{err}\n use MemoryHistory::with_initial_path to set a custom path")
            }),
            history: Vec::new(),
            future: Vec::new(),
        }
    }
}
