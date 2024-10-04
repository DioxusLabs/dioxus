use std::sync::Arc;

use super::*;

/// A context for the document
pub type DocumentContext = Arc<dyn Document>;

fn format_string_for_js(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn format_attributes(attributes: &[(&str, String)]) -> String {
    let mut formatted = String::from("[");
    for (key, value) in attributes {
        formatted.push_str(&format!(
            "[{}, {}],",
            format_string_for_js(key),
            format_string_for_js(value)
        ));
    }
    if formatted.ends_with(',') {
        formatted.pop();
    }
    formatted.push(']');
    formatted
}

fn create_element_in_head(
    tag: &str,
    attributes: &[(&str, String)],
    children: Option<String>,
) -> String {
    let helpers = include_str!("./js/head.js");
    let attributes = format_attributes(attributes);
    let children = children
        .as_deref()
        .map(format_string_for_js)
        .unwrap_or("null".to_string());
    let tag = format_string_for_js(tag);
    format!(r#"{helpers};window.createElementInHead({tag}, {attributes}, {children});"#)
}

/// A provider for document-related functionality.
///
/// Provides things like a history API, a title, a way to run JS, and some other basics/essentials used
/// by nearly every platform.
///
/// An integration with some kind of navigation history.
///
/// Depending on your use case, your implementation may deviate from the described procedure. This
/// is fine, as long as both `current_route` and `current_query` match the described format.
///
/// However, you should document all deviations. Also, make sure the navigation is user-friendly.
/// The described behaviors are designed to mimic a web browser, which most users should already
/// know. Deviations might confuse them.
pub trait Document: 'static {
    /// Run `eval` against this document, returning an [`Eval`] that can be used to await the result.
    fn eval(&self, js: String) -> Eval;

    /// Set the title of the document
    fn set_title(&self, title: String) {
        self.eval(format!("document.title = {title:?};"));
    }

    /// Create a new element in the head
    fn create_head_element(
        &self,
        name: &str,
        attributes: &[(&str, String)],
        contents: Option<String>,
    ) {
        self.eval(create_element_in_head(name, attributes, contents));
    }

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        let attributes = props.attributes();
        self.create_head_element("meta", &attributes, None);
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        let attributes = props.attributes();
        match (&props.src, props.script_contents()) {
            // The script has inline contents, render it as a script tag
            (_, Ok(contents)) => self.create_head_element("script", &attributes, Some(contents)),
            // The script has a src, render it as a script tag without a body
            (Some(_), _) => self.create_head_element("script", &attributes, None),
            // The script has neither contents nor src, log an error
            (None, Err(err)) => {
                err.log("Script");
                return;
            }
        }
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        let mut attributes = props.attributes();
        match (&props.href, props.style_contents()) {
            // The style has inline contents, render it as a style tag
            (_, Ok(contents)) => self.create_head_element("style", &attributes, Some(contents)),
            // The style has a src, render it as a link tag
            (Some(_), _) => {
                attributes.push(("type", "text/css".into()));
                self.create_head_element("link", &attributes, None)
            }
            // The style has neither contents nor src, log an error
            (None, Err(err)) => {
                err.log("Style");
                return;
            }
        };
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        let attributes = props.attributes();
        self.create_head_element("link", &attributes, None);
    }

    /// Get the path of the current URL.
    ///
    /// **Must start** with `/`. **Must _not_ contain** the prefix.
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
    fn base_route(&self) -> Option<String> {
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
    /// let mut history = MemoryHistory::<Route>::default();
    /// assert_eq!(history.current_route(), Route::Index {});
    ///
    /// history.push(Route::OtherPage {});
    /// assert_eq!(history.current_route(), Route::OtherPage {});
    /// assert!(history.can_go_back());
    /// ```
    fn push_route(&self, route: String);

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
    fn replace_route(&self, path: String);

    /// Navigate to an external URL.
    ///
    /// This should navigate to an external URL, which isn't controlled by the router. If a
    /// [`HistoryProvider`] cannot do that, it should return [`false`], otherwise [`true`].
    ///
    /// Returning [`false`] will cause the router to handle the external navigation failure.
    #[allow(unused_variables)]
    fn navigate_external(&self, url: String) -> bool {
        false
    }

    /// Provide the [`HistoryProvider`] with an update callback.
    ///
    /// Some [`HistoryProvider`]s may receive URL updates from outside the router. When such
    /// updates are received, they should call `callback`, which will cause the router to update.
    #[allow(unused_variables)]
    fn updater(&self, callback: Arc<dyn Fn()>) {}

    /// Whether the history provider is synchronous or asynchronous.
    ///
    /// Generally we only support synchronous history providers, but some platforms like liveview
    /// may need to be asynchronous.
    fn is_synchronous(&self) -> bool {
        true
    }
}
