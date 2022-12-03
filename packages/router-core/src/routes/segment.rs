use std::collections::BTreeMap;

use crate::{
    utils::{gen_parameter_sitemap, gen_sitemap},
    Name,
};

use super::{Matcher, ParameterRoute, Route, RouteContent};

/// A segment, representing a segment of the URLs path part (i.e. the stuff between two slashes).
#[derive(Debug)]
pub struct Segment<T: Clone> {
    pub(crate) index: Option<RouteContent<T>>,

    pub(crate) fallback: Option<RouteContent<T>>,
    pub(crate) clear_fallback: Option<bool>,

    pub(crate) fixed: BTreeMap<String, Route<T>>,
    pub(crate) matching: Vec<(Box<dyn Matcher>, ParameterRoute<T>)>,
    pub(crate) catch_all: Option<Box<ParameterRoute<T>>>,
}

impl<T: Clone> Segment<T> {
    /// Create a new [`Segment`] without index content.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::Segment;
    /// let seg: Segment<&'static str> = Segment::empty();
    /// ```
    pub fn empty() -> Self {
        Default::default()
    }

    /// Create a new [`Segment`] with some index `content`.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, Segment};
    /// let seg = Segment::content(ContentAtom("some content"));
    /// ```
    pub fn content(content: impl Into<RouteContent<T>>) -> Self {
        Self {
            index: Some(content.into()),
            ..Default::default()
        }
    }

    /// Create a new [`Segment`], possibly with some index `content`.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, Segment};
    /// let seg = Segment::new(Some(ContentAtom("some content")));
    /// ```
    pub fn new(content: Option<impl Into<RouteContent<T>>>) -> Self {
        match content {
            Some(content) => Self::content(content),
            None => Self::empty(),
        }
    }

    /// Add fallback content to a [`Segment`].
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, Segment};
    /// Segment::content(ContentAtom("some content")).fallback(ContentAtom("fallback content"));
    /// ```
    ///
    /// The fallback content of the innermost matched [`Segment`] is used, if the router cannot find
    /// a full matching route.
    ///
    /// # Error Handling
    /// This function may only be called once per [`Segment`]. In _debug mode_ the second call will
    /// panic. In _release mode_, all calls after the first will be ignored.
    pub fn fallback(mut self, content: impl Into<RouteContent<T>>) -> Self {
        debug_assert!(
            self.fallback.is_none(),
            "fallback content cannot be changed"
        );
        self.fallback.get_or_insert(content.into());

        self
    }

    /// Set whether to clear matched content when using the fallback.
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, Segment};
    /// Segment::content(ContentAtom("some content"))
    ///     .fallback(ContentAtom("fallback content"))
    ///     .clear_fallback(true);
    /// ```
    ///
    /// When this is [`true`], the router will remove all content it previously found when falling
    /// back to this [`Segment`]s fallback content. If not set, a [`Segment`] will inherit this
    /// value from its parent segment. For the root [`Segment`], this defaults to [`false`].
    ///
    /// # Error Handling
    /// This function may only be called once per [`Segment`]. In _debug mode_ the second call will
    /// panic. In _release mode_, all calls after the first will be ignored.
    pub fn clear_fallback(mut self, clear: bool) -> Self {
        debug_assert!(
            self.clear_fallback.is_none(),
            "fallback clearing cannot be changed"
        );
        self.clear_fallback.get_or_insert(clear);

        self
    }

    /// Add a fixed [`Route`] to the [`Segment`].
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, Segment};
    /// Segment::empty().fixed("path", ContentAtom("fixed route content"));
    /// ```
    ///
    /// A fixed route is active only when the corresponding URL segment is exactly the same as its
    /// path.
    ///
    /// # Error Handling
    /// An error occurs if multiple fixed routes on the same [`Segment`] have the same `path`. In
    /// _debug mode_, the second call with panic. In _release mode_, the later routes will be
    /// ignored and the initial preserved.
    pub fn fixed(mut self, path: impl Into<String>, content: impl Into<Route<T>>) -> Self {
        let path = path.into();

        debug_assert!(
            !self.fixed.contains_key(&path),
            "duplicate fixed route: {path}"
        );
        self.fixed.entry(path).or_insert_with(|| content.into());

        self
    }

    /// Add a matching [`ParameterRoute`] to the [`Segment`].
    ///
    /// ```rust,ignore
    /// # use dioxus_router_core::routes::Segment;
    /// Segment::empty().matching("some matcher", (true, ContentAtom("matching route content")));
    /// ```
    ///
    /// A matching route is active only when the corresponding URL segment is accepted by its
    /// [`Matcher`], and no previously added matching route is.
    ///
    /// The example above is not checked by the compiler. This is because dioxus-router-core doesn't ship any
    /// [`Matcher`]s by default. However, you can implement your own, or turn on the `regex` feature
    /// to enable a regex implementation.
    pub fn matching(
        mut self,
        matcher: impl Matcher + 'static,
        content: impl Into<ParameterRoute<T>>,
    ) -> Self {
        self.matching.push((Box::new(matcher), content.into()));
        self
    }

    /// Add a catch all [`ParameterRoute`] to the [`Segment`].
    ///
    /// ```rust
    /// # use dioxus_router_core::routes::{ContentAtom, Segment};
    /// Segment::empty().catch_all((ContentAtom("catch all route content"), true));
    /// ```
    ///
    /// A catch all route is active only if no fixed or matching route is.
    ///
    /// # Error Handling
    /// This function may only be called once per [`Segment`]. In _debug mode_ the second call will
    /// panic. In _release mode_, all calls after the first will be ignored.
    pub fn catch_all(mut self, content: impl Into<ParameterRoute<T>>) -> Self {
        debug_assert!(self.catch_all.is_none(), "duplicate catch all route");
        self.catch_all.get_or_insert(Box::new(content.into()));
        self
    }

    /// Generate a site map.
    ///
    /// ```rust
    /// # use std::collections::BTreeMap;
    /// # use dioxus_router_core::{Name, routes::Segment};
    /// let seg = Segment::<u8>::empty().fixed("fixed", "").catch_all(("", true));
    /// let sitemap = seg.gen_sitemap();
    /// assert_eq!(sitemap, vec!["/", "/fixed", "/\\bool"]);
    /// ```
    ///
    /// This function returns a [`Vec`] containing all routes the [`Segment`] knows about, as a
    /// path. Fixed routes are passed in as is, while matching and catch all routes are represented
    /// by their key, marked with a leading `\`. Since the otherwise all paths should be valid in
    /// URLs, and `\` is not, this doesn't cause a conflict.
    pub fn gen_sitemap(&self) -> Vec<String> {
        let mut res = Vec::new();
        res.push(String::from("/"));
        gen_sitemap(&self, "", &mut res);
        res
    }

    /// Generate a site map with parameters filled in.
    ///
    /// ```rust
    /// # use std::collections::BTreeMap;
    /// # use dioxus_router_core::{Name, routes::Segment};
    /// let seg = Segment::<u8>::empty().fixed("fixed", "").catch_all(("", true));
    /// let mut parameters = BTreeMap::new();
    /// parameters.insert(Name::of::<bool>(), vec![String::from("1"), String::from("2")]);
    ///
    /// let sitemap = seg.gen_parameter_sitemap(&parameters);
    /// assert_eq!(sitemap, vec!["/", "/fixed", "/1", "/2"]);
    /// ```
    ///
    /// This function returns a [`Vec`] containing all routes the [`Segment`] knows about, as a
    /// path. Fixed routes are passed in as is, while matching and catch all will be represented
    /// with all `parameters` provided for their key. Matching routes will also filter out all
    /// invalid parameters.
    pub fn gen_parameter_sitemap(&self, parameters: &BTreeMap<Name, Vec<String>>) -> Vec<String> {
        let mut res = Vec::new();
        res.push(String::from("/"));
        gen_parameter_sitemap(&self, parameters, "", &mut res);
        res
    }
}

impl<T: Clone> Default for Segment<T> {
    fn default() -> Self {
        Self {
            index: None,
            fallback: None,
            clear_fallback: None,
            fixed: BTreeMap::new(),
            matching: Vec::new(),
            catch_all: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::routes::{content::test_content, ContentAtom};

    use super::*;

    #[test]
    fn default() {
        let seg: Segment<&str> = Default::default();

        assert!(seg.index.is_none());
        assert!(seg.fallback.is_none());
        assert!(seg.clear_fallback.is_none());
        assert!(seg.fixed.is_empty());
        assert!(seg.matching.is_empty());
        assert!(seg.catch_all.is_none());
    }

    #[test]
    fn empty() {
        let seg = Segment::<&str>::empty();

        assert!(seg.index.is_none());
        assert!(seg.fallback.is_none());
        assert!(seg.clear_fallback.is_none());
        assert!(seg.fixed.is_empty());
        assert!(seg.matching.is_empty());
        assert!(seg.catch_all.is_none());
    }

    #[test]
    fn content() {
        let seg = Segment::content(test_content());

        assert_eq!(seg.index, Some(test_content()));
        assert!(seg.fallback.is_none());
        assert!(seg.clear_fallback.is_none());
        assert!(seg.fixed.is_empty());
        assert!(seg.matching.is_empty());
        assert!(seg.catch_all.is_none());
    }

    #[test]
    fn new_empty() {
        let seg = Segment::<&str>::new(None::<String>);

        assert!(seg.index.is_none());
        assert!(seg.fallback.is_none());
        assert!(seg.clear_fallback.is_none());
        assert!(seg.fixed.is_empty());
        assert!(seg.matching.is_empty());
        assert!(seg.catch_all.is_none());
    }

    #[test]
    fn new_content() {
        let seg = Segment::new(Some(test_content()));

        assert_eq!(seg.index, Some(test_content()));
        assert!(seg.fallback.is_none());
        assert!(seg.clear_fallback.is_none());
        assert!(seg.fixed.is_empty());
        assert!(seg.matching.is_empty());
        assert!(seg.catch_all.is_none());
    }

    #[test]
    fn fallback_initial() {
        let seg = Segment::empty().fallback(test_content());

        assert_eq!(seg.fallback, Some(test_content()));
    }

    #[test]
    #[should_panic = "fallback content cannot be changed"]
    #[cfg(debug_assertions)]
    fn fallback_debug() {
        Segment::empty()
            .fallback(test_content())
            .fallback(test_content());
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn fallback_release() {
        let seg = Segment::empty()
            .fallback(test_content())
            .fallback(RouteContent::Content(ContentAtom("invalid")));

        assert_eq!(seg.fallback, Some(test_content()));
    }

    #[test]
    fn clear_fallback() {
        let mut seg = Segment::<&str>::empty();
        assert!(seg.clear_fallback.is_none());

        seg = seg.clear_fallback(true);
        assert_eq!(seg.clear_fallback, Some(true));
    }

    #[test]
    #[should_panic = "fallback clearing cannot be changed"]
    #[cfg(debug_assertions)]
    fn clear_fallback_debug() {
        Segment::<&str>::empty()
            .clear_fallback(true)
            .clear_fallback(false);
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn clear_fallback_release() {
        let seg = Segment::<&str>::empty()
            .clear_fallback(true)
            .clear_fallback(false);
        assert_eq!(seg.clear_fallback, Some(true));
    }

    #[test]
    fn fixed() {
        let test = RouteContent::Content(ContentAtom("test"));
        let other = RouteContent::Content(ContentAtom("other"));
        let seg = Segment::empty()
            .fixed("test", Route::content(test.clone()))
            .fixed("other", Route::content(other.clone()));

        assert_eq!(seg.fixed.len(), 2);
        assert_eq!(seg.fixed["test"].content, Some(test));
        assert_eq!(seg.fixed["other"].content, Some(other));
    }

    #[test]
    #[should_panic = "duplicate fixed route: test"]
    #[cfg(debug_assertions)]
    fn fixed_debug() {
        Segment::empty()
            .fixed(
                "test",
                Route::content(RouteContent::Content(ContentAtom("test"))),
            )
            .fixed(
                "test",
                Route::content(RouteContent::Content(ContentAtom("other"))),
            );
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn fixed_release() {
        let test = RouteContent::Content(ContentAtom("test"));
        let other = RouteContent::Content(ContentAtom("other"));
        let seg = Segment::empty()
            .fixed("test", Route::content(test.clone()))
            .fixed("test", Route::content(other.clone()));

        assert_eq!(seg.fixed.len(), 1);
        assert_eq!(seg.fixed["test"].content, Some(test));
    }

    #[test]
    fn matching() {
        let test = RouteContent::Content(ContentAtom("test"));
        let other = RouteContent::Content(ContentAtom("other"));
        let seg = Segment::empty()
            .matching(
                String::from("test"),
                ParameterRoute::content::<String>(test.clone()),
            )
            .matching(
                String::from("other"),
                ParameterRoute::content::<String>(other.clone()),
            );

        assert_eq!(seg.matching.len(), 2);
        assert_eq!(seg.matching[0].1.content, Some(test));
        assert_eq!(seg.matching[1].1.content, Some(other));
    }

    #[test]
    fn catch_all_initial() {
        let seg = Segment::empty().catch_all(ParameterRoute::content::<String>(test_content()));

        assert!(seg.catch_all.is_some());
        assert_eq!(seg.catch_all.unwrap().content, Some(test_content()));
    }

    #[test]
    #[should_panic = "duplicate catch all route"]
    #[cfg(debug_assertions)]
    fn catch_all_debug() {
        Segment::empty()
            .catch_all(ParameterRoute::content::<String>(test_content()))
            .catch_all(ParameterRoute::content::<String>(test_content()));
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn catch_all_release() {
        let seg = Segment::empty()
            .catch_all(ParameterRoute::content::<String>(test_content()))
            .catch_all(ParameterRoute::empty::<bool>());

        assert!(seg.catch_all.is_some());
        assert_eq!(seg.catch_all.unwrap().content, Some(test_content()));
    }

    // Check whether the returned sitemap includes "/". More elaborate tests are located alongside
    // the internal `gen_sitemap` function.
    #[test]
    fn gen_sitemap() {
        assert_eq!(Segment::<&'static str>::empty().gen_sitemap(), vec!["/"]);
    }

    // Check whether the returned sitemap includes "/". More elaborate tests are located alongside
    // the internal `gen_parameter_sitemap` function.
    #[test]
    fn gen_parameter_sitemap() {
        assert_eq!(
            Segment::<&'static str>::empty().gen_parameter_sitemap(&BTreeMap::new()),
            vec!["/"]
        );
    }
}
