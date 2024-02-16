//! History Integration
//!
//! dioxus-router-core relies on [`HistoryProvider`]s to store the current Route, and possibly a
//! history (i.e. a browsers back button) and future (i.e. a browsers forward button).
//!
//! To integrate dioxus-router with a any type of history, all you have to do is implement the
//! [`HistoryProvider`] trait.
//!
//! dioxus-router contains two built in history providers:
//! 1) [`MemoryHistory`] for desktop/mobile/ssr platforms
//! 2) [`WebHistory`] for web platforms

use std::{any::Any, rc::Rc, sync::Arc};

mod memory;
pub use memory::*;

#[cfg(feature = "web")]
mod web;
#[cfg(feature = "web")]
pub use web::*;
#[cfg(feature = "web")]
pub(crate) mod web_history;

#[cfg(feature = "liveview")]
mod liveview;
#[cfg(feature = "liveview")]
pub use liveview::*;

// #[cfg(feature = "web")]
// mod web_hash;
// #[cfg(feature = "web")]
// pub use web_hash::*;

use crate::routable::Routable;

#[cfg(feature = "web")]
pub(crate) mod web_scroll;

/// An integration with some kind of navigation history.
///
/// Depending on your use case, your implementation may deviate from the described procedure. This
/// is fine, as long as both `current_route` and `current_query` match the described format.
///
/// However, you should document all deviations. Also, make sure the navigation is user-friendly.
/// The described behaviors are designed to mimic a web browser, which most users should already
/// know. Deviations might confuse them.
pub trait HistoryProvider<R: Routable> {
    /// Get the path of the current URL.
    ///
    /// **Must start** with `/`. **Must _not_ contain** the prefix.
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
    /// let mut history = MemoryHistory::<Route>::default();
    /// assert_eq!(history.current_route().to_string(), "/");
    ///
    /// history.push(Route::OtherPage {});
    /// assert_eq!(history.current_route().to_string(), "/some-other-page");
    /// ```
    #[must_use]
    fn current_route(&self) -> R;

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
    /// # use dioxus_router::prelude::*;
    /// # use dioxus::prelude::*;
    /// # #[component]
    /// # fn Index() -> Element { None }
    /// # fn Other() -> Element { None }
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
    fn go_back(&mut self);

    /// Check whether there is a future page to navigate forward to.
    ///
    /// If a [`HistoryProvider`] cannot know this, it should return [`true`].
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
    fn go_forward(&mut self);

    /// Go to another page.
    ///
    /// This should do three things:
    /// 1. Merge the current URL with the `path` parameter (which may also include a query part).
    /// 2. Remove the previous URL to the navigation history.
    /// 3. Clear the navigation future.
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
    /// let mut history = MemoryHistory::<Route>::default();
    /// assert_eq!(history.current_route(), Route::Index {});
    ///
    /// history.push(Route::OtherPage {});
    /// assert_eq!(history.current_route(), Route::OtherPage {});
    /// assert!(history.can_go_back());
    /// ```
    fn push(&mut self, route: R);

    /// Replace the current page with another one.
    ///
    /// This should merge the current URL with the `path` parameter (which may also include a query
    /// part). In contrast to the `push` function, the navigation history and future should stay
    /// untouched.
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
    /// let mut history = MemoryHistory::<Route>::default();
    /// assert_eq!(history.current_route(), Route::Index {});
    ///
    /// history.replace(Route::OtherPage {});
    /// assert_eq!(history.current_route(), Route::OtherPage {});
    /// assert!(!history.can_go_back());
    /// ```
    fn replace(&mut self, path: R);

    /// Navigate to an external URL.
    ///
    /// This should navigate to an external URL, which isn't controlled by the router. If a
    /// [`HistoryProvider`] cannot do that, it should return [`false`], otherwise [`true`].
    ///
    /// Returning [`false`] will cause the router to handle the external navigation failure.
    #[allow(unused_variables)]
    fn external(&mut self, url: String) -> bool {
        false
    }

    /// Provide the [`HistoryProvider`] with an update callback.
    ///
    /// Some [`HistoryProvider`]s may receive URL updates from outside the router. When such
    /// updates are received, they should call `callback`, which will cause the router to update.
    #[allow(unused_variables)]
    fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {}
}

pub(crate) trait AnyHistoryProvider {
    fn parse_route(&self, route: &str) -> Result<Rc<dyn Any>, String>;

    #[must_use]
    fn current_route(&self) -> Rc<dyn Any>;

    #[must_use]
    fn can_go_back(&self) -> bool {
        true
    }

    fn go_back(&mut self);

    #[must_use]
    fn can_go_forward(&self) -> bool {
        true
    }

    fn go_forward(&mut self);

    fn push(&mut self, route: Rc<dyn Any>);

    fn replace(&mut self, path: Rc<dyn Any>);

    #[allow(unused_variables)]
    fn external(&mut self, url: String) -> bool {
        false
    }

    #[allow(unused_variables)]
    fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {}
}

pub(crate) struct AnyHistoryProviderImplWrapper<R, H> {
    inner: H,
    _marker: std::marker::PhantomData<R>,
}

impl<R, H> AnyHistoryProviderImplWrapper<R, H> {
    pub fn new(inner: H) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<R, H: Default> Default for AnyHistoryProviderImplWrapper<R, H> {
    fn default() -> Self {
        Self::new(H::default())
    }
}

impl<R, H> AnyHistoryProvider for AnyHistoryProviderImplWrapper<R, H>
where
    R: Routable,
    <R as std::str::FromStr>::Err: std::fmt::Display,
    H: HistoryProvider<R>,
{
    fn parse_route(&self, route: &str) -> Result<Rc<dyn Any>, String> {
        R::from_str(route)
            .map_err(|err| err.to_string())
            .map(|route| Rc::new(route) as Rc<dyn Any>)
    }

    fn current_route(&self) -> Rc<dyn Any> {
        let route = self.inner.current_route();
        Rc::new(route)
    }

    fn can_go_back(&self) -> bool {
        self.inner.can_go_back()
    }

    fn go_back(&mut self) {
        self.inner.go_back()
    }

    fn can_go_forward(&self) -> bool {
        self.inner.can_go_forward()
    }

    fn go_forward(&mut self) {
        self.inner.go_forward()
    }

    fn push(&mut self, route: Rc<dyn Any>) {
        self.inner
            .push(route.downcast::<R>().unwrap().as_ref().clone())
    }

    fn replace(&mut self, route: Rc<dyn Any>) {
        self.inner
            .replace(route.downcast::<R>().unwrap().as_ref().clone())
    }

    fn external(&mut self, url: String) -> bool {
        self.inner.external(url)
    }

    fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {
        self.inner.updater(callback)
    }
}
