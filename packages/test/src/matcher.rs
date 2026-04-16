use crate::element::ResolvedElement;
use std::ops::ControlFlow;

/// A representation of a condition to be expected on the DOM.
pub trait Matcher<T> {
    fn matches(&self, actual: T) -> ControlFlow<()>;
}

/// Returns a [Matcher] which matches an element whose inner HTML is matched by the [Matcher]
/// `inner`.
pub fn inner_html(inner: impl Matcher<String>) -> impl for<'vdom> Matcher<ResolvedElement<'vdom>> {
    struct InnerHtmlMatcher<InnerMatcher: Matcher<String>>(InnerMatcher);

    impl<'vdom, InnerMatcher: Matcher<String>> Matcher<ResolvedElement<'vdom>>
        for InnerHtmlMatcher<InnerMatcher>
    {
        fn matches(&self, element: ResolvedElement<'vdom>) -> ControlFlow<()> {
            let inner_html = element.inner_html();
            self.0.matches(inner_html)
        }
    }

    InnerHtmlMatcher(inner)
}

/// Returns a [Matcher] which matches a value which equals the given value in the sense of
/// [`PartialEq`].
pub fn eq<T, A: PartialEq<T>>(value: T) -> impl Matcher<A> {
    struct EqualsMatcher<T>(T);

    impl<T, A: PartialEq<T>> Matcher<A> for EqualsMatcher<T> {
        fn matches(&self, actual: A) -> ControlFlow<()> {
            if actual == self.0 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }
    }

    EqualsMatcher(value)
}

/// Returns a [Matcher] which matches a `String` containing the given `substring`.
pub fn contains_string<'a>(substring: impl AsRef<str> + 'a) -> impl Matcher<String> + 'a {
    struct ContainingStringMatcher<Expected: AsRef<str>>(Expected);

    impl<Expected: AsRef<str>> Matcher<String> for ContainingStringMatcher<Expected> {
        fn matches(&self, actual: String) -> ControlFlow<()> {
            if actual.contains(self.0.as_ref()) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }
    }

    ContainingStringMatcher(substring)
}

/// Returns a [Matcher] which matches any data not matched by the given [Matcher] `inner`.
pub fn not<T>(inner: impl Matcher<T>) -> impl Matcher<T> {
    struct NotMatcher<InnerMatcher>(InnerMatcher);

    impl<T, InnerMatcher: Matcher<T>> Matcher<T> for NotMatcher<InnerMatcher> {
        fn matches(&self, actual: T) -> ControlFlow<()> {
            match self.0.matches(actual) {
                ControlFlow::Continue(_) => ControlFlow::Break(()),
                ControlFlow::Break(_) => ControlFlow::Continue(()),
            }
        }
    }

    NotMatcher(inner)
}

/// A [Matcher] which matches a `Vec` with no elements.
///
/// Returned by [empty].
pub struct EmptyMatcher;

impl<T> Matcher<Vec<T>> for EmptyMatcher {
    fn matches(&self, actual: Vec<T>) -> ControlFlow<()> {
        if actual.is_empty() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

/// Returns a [Matcher] which matches a `Vec` with no elements.
pub fn empty() -> EmptyMatcher {
    EmptyMatcher
}
