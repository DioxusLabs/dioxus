use crate::element::ResolvedElement;
use std::ops::ControlFlow;

/// A representation of a condition to be expected on the DOM.
pub trait Matcher<T: std::fmt::Debug> {
    fn matches(&self, actual: T) -> ControlFlow<()>;

    fn describe(&self) -> String;

    fn explain_failure(&self, actual: T) -> String {
        format!("\nExpected: {}\n  but was: {actual:?}\n", self.describe())
    }
}

/// Returns a [Matcher] which matches an element whose inner HTML is matched by the [Matcher]
/// `inner`.
pub fn inner_html(inner: impl Matcher<String>) -> impl for<'vdom> Matcher<ResolvedElement<'vdom>> {
    struct InnerHtmlMatcher<InnerMatcher>(InnerMatcher);

    impl<'vdom, InnerMatcher: Matcher<String>> Matcher<ResolvedElement<'vdom>>
        for InnerHtmlMatcher<InnerMatcher>
    {
        fn matches(&self, element: ResolvedElement<'vdom>) -> ControlFlow<()> {
            let inner_html = element.inner_html();
            self.0.matches(inner_html)
        }

        fn describe(&self) -> String {
            format!("inner HTML {}", self.0.describe())
        }
    }

    InnerHtmlMatcher(inner)
}

/// Returns a [Matcher] which matches a value which equals the given value in the sense of
/// [`PartialEq`].
pub fn eq<T: std::fmt::Debug, A: PartialEq<T> + std::fmt::Debug>(value: T) -> impl Matcher<A> {
    struct EqualsMatcher<T>(T);

    impl<T: std::fmt::Debug, A: PartialEq<T> + std::fmt::Debug> Matcher<A> for EqualsMatcher<T> {
        fn matches(&self, actual: A) -> ControlFlow<()> {
            if actual == self.0 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }

        fn describe(&self) -> String {
            format!("equal to {:?}", self.0)
        }
    }

    EqualsMatcher(value)
}

/// Returns a [Matcher] which matches a `String` containing the given `substring`.
pub fn contains_string<'a>(substring: impl AsRef<str> + 'a) -> impl Matcher<String> + 'a {
    struct ContainingStringMatcher<Expected>(Expected);

    impl<Expected: AsRef<str>> Matcher<String> for ContainingStringMatcher<Expected> {
        fn matches(&self, actual: String) -> ControlFlow<()> {
            if actual.contains(self.0.as_ref()) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }

        fn describe(&self) -> String {
            format!("contains string {}", self.0.as_ref())
        }
    }

    ContainingStringMatcher(substring)
}

pub struct NotMatcher<InnerMatcher>(InnerMatcher);

impl<T: std::fmt::Debug, InnerMatcher: Matcher<T>> Matcher<T> for NotMatcher<InnerMatcher> {
    fn matches(&self, actual: T) -> ControlFlow<()> {
        match self.0.matches(actual) {
            ControlFlow::Continue(_) => ControlFlow::Break(()),
            ControlFlow::Break(_) => ControlFlow::Continue(()),
        }
    }

    fn describe(&self) -> String {
        format!("not {}", self.0.describe())
    }
}

/// Returns a [Matcher] which matches any data not matched by the given [Matcher] `inner`.
pub fn not<M>(inner: M) -> NotMatcher<M> {
    NotMatcher(inner)
}

/// A [Matcher] which matches a `Vec` with no elements.
///
/// Returned by [empty].
pub struct EmptyMatcher;

impl<T: std::fmt::Debug> Matcher<Vec<T>> for EmptyMatcher {
    fn matches(&self, actual: Vec<T>) -> ControlFlow<()> {
        if actual.is_empty() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    fn describe(&self) -> String {
        "an empty collection".into()
    }
}

/// Returns a [Matcher] which matches a `Vec` with no elements.
pub fn empty() -> EmptyMatcher {
    EmptyMatcher
}
