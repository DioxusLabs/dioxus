use std::cell::RefCell;

use crate::History;

struct MemoryHistoryState {
    current: String,
    history: Vec<String>,
    future: Vec<String>,
}

/// A [`History`] provider that stores all navigation information in memory.
pub struct MemoryHistory {
    state: RefCell<MemoryHistoryState>,
    base_path: Option<String>,
}

impl Default for MemoryHistory {
    fn default() -> Self {
        Self::with_initial_path("/")
    }
}

impl MemoryHistory {
    /// Create a [`MemoryHistory`] starting at `path`.
    ///
    /// ```rust
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
    /// let mut history = dioxus_history::MemoryHistory::with_initial_path(Route::Index {});
    /// assert_eq!(history.current_route(), Route::Index {}.to_string());
    /// assert_eq!(history.can_go_back(), false);
    /// ```
    pub fn with_initial_path(path: impl ToString) -> Self {
        Self {
            state: MemoryHistoryState{
                current: path.to_string().parse().unwrap_or_else(|err| {
                    panic!("index route does not exist:\n{err}\n use MemoryHistory::with_initial_path to set a custom path")
                }),
                history: Vec::new(),
                future: Vec::new(),
            }.into(),
            base_path: None,
        }
    }

    /// Set the base path for the history. All routes will be prefixed with this path when rendered.
    ///
    /// ```rust
    /// # use dioxus_history::*;
    /// let mut history = MemoryHistory::default().with_prefix("/my-app");
    ///
    /// // The base path is set to "/my-app"
    /// assert_eq!(history.current_prefix(), Some("/my-app".to_string()));
    /// ```
    pub fn with_prefix(mut self, prefix: impl ToString) -> Self {
        self.base_path = Some(prefix.to_string());
        self
    }
}

impl History for MemoryHistory {
    fn current_prefix(&self) -> Option<String> {
        self.base_path.clone()
    }

    fn current_route(&self) -> String {
        self.state.borrow().current.clone()
    }

    fn can_go_back(&self) -> bool {
        !self.state.borrow().history.is_empty()
    }

    fn go_back(&self) {
        let mut write = self.state.borrow_mut();
        if let Some(last) = write.history.pop() {
            let old = std::mem::replace(&mut write.current, last);
            write.future.push(old);
        }
    }

    fn can_go_forward(&self) -> bool {
        !self.state.borrow().future.is_empty()
    }

    fn go_forward(&self) {
        let mut write = self.state.borrow_mut();
        if let Some(next) = write.future.pop() {
            let old = std::mem::replace(&mut write.current, next);
            write.history.push(old);
        }
    }

    fn push(&self, new: String) {
        let mut write = self.state.borrow_mut();
        // don't push the same route twice
        if write.current == new {
            return;
        }
        let old = std::mem::replace(&mut write.current, new);
        write.history.push(old);
        write.future.clear();
    }

    fn replace(&self, path: String) {
        let mut write = self.state.borrow_mut();
        write.current = path;
    }
}
