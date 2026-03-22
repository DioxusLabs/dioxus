use std::marker::PhantomData;

use crate::element::ResolvedElement;

/// A representation of a condition to be expected on the DOM.
pub trait Matcher<T> {
    fn matches(&self, actual: &T) -> bool;
}

/// Returns a [Matcher] which matches an element whose inner HTML is matched by the [Matcher]
/// `inner`.
pub fn inner_html(inner: impl Matcher<String>) -> impl for<'vdom> Matcher<ResolvedElement<'vdom>> {
    struct InnerHtmlMatcher<InnerMatcher: Matcher<String>>(InnerMatcher);

    impl<'vdom, InnerMatcher: Matcher<String>> Matcher<ResolvedElement<'vdom>>
        for InnerHtmlMatcher<InnerMatcher>
    {
        fn matches(&self, element: &ResolvedElement<'vdom>) -> bool {
            self.0.matches(&element.inner_html())
        }
    }

    InnerHtmlMatcher(inner)
}

/// Returns a [Matcher] which matches a `String` containing the given `substring`.
pub fn contains_string<'a>(substring: impl AsRef<str> + 'a) -> impl Matcher<String> + 'a {
    struct ContainingStringMatcher<Expected: AsRef<str>>(Expected);

    impl<Expected: AsRef<str>> Matcher<String> for ContainingStringMatcher<Expected> {
        fn matches(&self, actual: &String) -> bool {
            actual.contains(self.0.as_ref())
        }
    }

    ContainingStringMatcher(substring)
}

/// Returns a [Matcher] which matches any data not matched by the given [Matcher] `inner`.
pub fn not<T>(inner: impl Matcher<T>) -> impl Matcher<T> {
    struct NotMatcher<T, InnerMatcher: Matcher<T>>(InnerMatcher, PhantomData<T>);

    impl<T, InnerMatcher: Matcher<T>> Matcher<T> for NotMatcher<T, InnerMatcher> {
        fn matches(&self, actual: &T) -> bool {
            !self.0.matches(actual)
        }
    }

    NotMatcher(inner, Default::default())
}
