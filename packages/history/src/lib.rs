use dioxus_core::{provide_context, provide_root_context};
use std::{rc::Rc, sync::Arc};

mod memory;
pub use memory::*;

/// Get the history provider for the current platform if the platform doesn't implement a history functionality.
pub fn history() -> Rc<dyn History> {
    match dioxus_core::try_consume_context::<Rc<dyn History>>() {
        Some(history) => history,
        None => {
            tracing::error!("Unable to find a history provider in the renderer. Make sure your renderer supports the Router. Falling back to the in-memory history provider.");
            provide_root_context(Rc::new(MemoryHistory::default()))
        }
    }
}

/// Provide a history context to the current component.
pub fn provide_history_context(history: Rc<dyn History>) {
    provide_context(history);
}

pub trait History {
    /// Get the path of the current URL.
    ///
    /// **Must start** with `/`. **Must _not_ contain** the prefix.
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
    /// let mut history = dioxus::history::MemoryHistory::default();
    /// assert_eq!(history.current_route(), "/");
    ///
    /// history.push(Route::OtherPage {}.to_string());
    /// assert_eq!(history.current_route(), "/some-other-page");
    /// ```
    #[must_use]
    fn current_route(&self) -> String;

    /// Get the current path prefix of the URL.
    ///
    /// Not all [`History`]s need a prefix feature. It is meant for environments where a
    /// dioxus-router-core-routed application is not running on `/`. The [`History`] is responsible
    /// for removing the prefix from the dioxus-router-core-internal path, and also for adding it back in
    /// during navigation. This functions value is only used for creating `href`s (e.g. for SSR or
    /// display (but not navigation) in a web app).
    fn current_prefix(&self) -> Option<String> {
        None
    }

    /// Check whether there is a previous page to navigate back to.
    ///
    /// If a [`History`] cannot know this, it should return [`true`].
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// # #[component]
    /// # fn Index() -> Element { VNode::empty() }
    /// # fn Other() -> Element { VNode::empty() }
    /// #[derive(Clone, Routable, Debug, PartialEq)]
    /// enum Route {
    ///     #[route("/")]
    ///     Index {},
    ///     #[route("/other")]
    ///     Other {},
    /// }
    /// let mut history = dioxus::history::MemoryHistory::default();
    /// assert_eq!(history.can_go_back(), false);
    ///
    /// history.push(Route::Other {}.to_string());
    /// assert_eq!(history.can_go_back(), true);
    /// ```
    #[must_use]
    fn can_go_back(&self) -> bool {
        true
    }

    /// Go back to a previous page.
    ///
    /// If a [`History`] cannot go to a previous page, it should do nothing. This method
    /// might be called, even if `can_go_back` returns [`false`].
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
    /// let mut history = dioxus::history::MemoryHistory::default();
    /// assert_eq!(history.current_route(), "/");
    ///
    /// history.go_back();
    /// assert_eq!(history.current_route(), "/");
    ///
    /// history.push(Route::OtherPage {}.to_string());
    /// assert_eq!(history.current_route(), "/some-other-page");
    ///
    /// history.go_back();
    /// assert_eq!(history.current_route(), "/");
    /// ```
    fn go_back(&self);

    /// Check whether there is a future page to navigate forward to.
    ///
    /// If a [`History`] cannot know this, it should return [`true`].
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
    /// let mut history = dioxus::history::MemoryHistory::default();
    /// assert_eq!(history.can_go_forward(), false);
    ///
    /// history.push(Route::OtherPage {}.to_string());
    /// assert_eq!(history.can_go_forward(), false);
    ///
    /// history.go_back();
    /// assert_eq!(history.can_go_forward(), true);
    /// ```
    #[must_use]
    fn can_go_forward(&self) -> bool {
        true
    }

    /// Go forward to a future page.
    ///
    /// If a [`History`] cannot go to a previous page, it should do nothing. This method
    /// might be called, even if `can_go_forward` returns [`false`].
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
    /// let mut history = dioxus::history::MemoryHistory::default();
    /// history.push(Route::OtherPage {}.to_string());
    /// assert_eq!(history.current_route(), Route::OtherPage {}.to_string());
    ///
    /// history.go_back();
    /// assert_eq!(history.current_route(), Route::Index {}.to_string());
    ///
    /// history.go_forward();
    /// assert_eq!(history.current_route(), Route::OtherPage {}.to_string());
    /// ```
    fn go_forward(&self);

    /// Go to another page.
    ///
    /// This should do three things:
    /// 1. Merge the current URL with the `path` parameter (which may also include a query part).
    /// 2. Remove the previous URL to the navigation history.
    /// 3. Clear the navigation future.
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
    /// let mut history = dioxus::history::MemoryHistory::default();
    /// assert_eq!(history.current_route(), Route::Index {}.to_string());
    ///
    /// history.push(Route::OtherPage {}.to_string());
    /// assert_eq!(history.current_route(), Route::OtherPage {}.to_string());
    /// assert!(history.can_go_back());
    /// ```
    fn push(&self, route: String);

    /// Replace the current page with another one.
    ///
    /// This should merge the current URL with the `path` parameter (which may also include a query
    /// part). In contrast to the `push` function, the navigation history and future should stay
    /// untouched.
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
    /// let mut history = dioxus::history::MemoryHistory::default();
    /// assert_eq!(history.current_route(), Route::Index {}.to_string());
    ///
    /// history.replace(Route::OtherPage {}.to_string());
    /// assert_eq!(history.current_route(), Route::OtherPage {}.to_string());
    /// assert!(!history.can_go_back());
    /// ```
    fn replace(&self, path: String);

    /// Navigate to an external URL.
    ///
    /// This should navigate to an external URL, which isn't controlled by the router. If a
    /// [`History`] cannot do that, it should return [`false`], otherwise [`true`].
    ///
    /// Returning [`false`] will cause the router to handle the external navigation failure.
    #[allow(unused_variables)]
    fn external(&self, url: String) -> bool {
        false
    }

    /// Provide the [`History`] with an update callback.
    ///
    /// Some [`History`]s may receive URL updates from outside the router. When such
    /// updates are received, they should call `callback`, which will cause the router to update.
    #[allow(unused_variables)]
    fn updater(&self, callback: Arc<dyn Fn() + Send + Sync>) {}

    /// Whether the router should include the legacy prevent default attribute instead of the new
    /// prevent default method. This should only be used by liveview.
    fn include_prevent_default(&self) -> bool {
        false
    }
}
