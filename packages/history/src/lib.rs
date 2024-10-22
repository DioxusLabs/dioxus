use std::{rc::Rc, sync::Arc};

mod lens;
mod memory;

pub use lens::*;
pub use memory::*;

/// Get the history provider for the current platform if the platform doesn't implement a history functionality.
pub fn history() -> Rc<dyn History> {
    match dioxus_core::prelude::try_consume_context::<Rc<dyn History>>() {
        Some(history) => history,
        None => {
            panic!("Unable to find a history provider in the renderer. Make sure your renderer supports the Router");
        }
    }
}

pub trait History {
    /// Get the global path of the current URL. In nested routes, this will be the path of the parent route.
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
    ///     #[child]
    ///     OtherPage {
    ///         child: OtherPage
    ///     },
    /// }
    /// let mut history = MemoryHistory::<Route>::default();
    /// assert_eq!(history.full_route_path().to_string(), "/");
    ///
    /// history.push(Route::OtherPage {});
    /// assert_eq!(history.full_route_path().to_string(), "/some-other-page");
    /// ```
    #[must_use]
    fn full_route_path(&self) -> String {
        self.current_route()
    }

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
    /// let mut history = MemoryHistory::<Route>::default();
    /// assert_eq!(history.current_route().to_string(), "/");
    ///
    /// history.push(Route::OtherPage {});
    /// assert_eq!(history.current_route().to_string(), "/some-other-page");
    /// ```
    #[must_use]
    fn current_route(&self) -> String;

    /// Get the current path prefix of the URL.
    ///
    /// Not all [`HistoryProvider`]s need a prefix feature. It is meant for environments where a
    /// dioxus-router-core-routed application is not running on `/`. The [`HistoryProvider`] is responsible
    /// for removing the prefix from the dioxus-router-core-internal path, and also for adding it back in
    /// during navigation. This functions value is only used for creating `href`s (e.g. for SSR or
    /// display (but not navigation) in a web app).
    fn current_prefix(&self) -> Option<String> {
        None
    }

    /// Check whether there is a previous page to navigate back to.
    ///
    /// If a [`HistoryProvider`] cannot know this, it should return [`true`].
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
    /// let mut history = MemoryHistory::<Route>::default();
    /// assert_eq!(history.can_go_back(), false);
    ///
    /// history.push(Route::Other {});
    /// assert_eq!(history.can_go_back(), true);
    /// ```
    #[must_use]
    fn can_go_back(&self) -> bool {
        true
    }

    /// Go back to a previous page.
    ///
    /// If a [`HistoryProvider`] cannot go to a previous page, it should do nothing. This method
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
    /// let mut history = MemoryHistory::<Route>::default();
    /// assert_eq!(history.current_route().to_string(), "/");
    ///
    /// history.go_back();
    /// assert_eq!(history.current_route().to_string(), "/");
    ///
    /// history.push(Route::OtherPage {});
    /// assert_eq!(history.current_route().to_string(), "/some-other-page");
    ///
    /// history.go_back();
    /// assert_eq!(history.current_route().to_string(), "/");
    /// ```
    fn go_back(&self);

    /// Check whether there is a future page to navigate forward to.
    ///
    /// If a [`HistoryProvider`] cannot know this, it should return [`true`].
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
    /// let mut history = MemoryHistory::<Route>::default();
    /// assert_eq!(history.can_go_forward(), false);
    ///
    /// history.push(Route::OtherPage {});
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
    /// If a [`HistoryProvider`] cannot go to a previous page, it should do nothing. This method
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
    /// let mut history = MemoryHistory::<Route>::default();
    /// history.push(Route::OtherPage {});
    /// assert_eq!(history.current_route(), Route::OtherPage {});
    ///
    /// history.go_back();
    /// assert_eq!(history.current_route(), Route::Index {});
    ///
    /// history.go_forward();
    /// assert_eq!(history.current_route(), Route::OtherPage {});
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
    /// let mut history = MemoryHistory::<Route>::default();
    /// assert_eq!(history.current_route(), Route::Index {});
    ///
    /// history.push(Route::OtherPage {});
    /// assert_eq!(history.current_route(), Route::OtherPage {});
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
    /// let mut history = MemoryHistory::<Route>::default();
    /// assert_eq!(history.current_route(), Route::Index {});
    ///
    /// history.replace(Route::OtherPage {});
    /// assert_eq!(history.current_route(), Route::OtherPage {});
    /// assert!(!history.can_go_back());
    /// ```
    fn replace(&self, path: String);

    /// Navigate to an external URL.
    ///
    /// This should navigate to an external URL, which isn't controlled by the router. If a
    /// [`HistoryProvider`] cannot do that, it should return [`false`], otherwise [`true`].
    ///
    /// Returning [`false`] will cause the router to handle the external navigation failure.
    #[allow(unused_variables)]
    fn external(&self, url: String) -> bool {
        false
    }

    /// Provide the [`HistoryProvider`] with an update callback.
    ///
    /// Some [`HistoryProvider`]s may receive URL updates from outside the router. When such
    /// updates are received, they should call `callback`, which will cause the router to update.
    #[allow(unused_variables)]
    fn updater(&self, callback: Arc<dyn Fn() + Send + Sync>) {}

    /// Whether the router should include the legacy prevent default attribute instead of the new
    /// prevent default method. This should only be used by liveview.
    fn include_prevent_default(&self) -> bool {
        false
    }
}
