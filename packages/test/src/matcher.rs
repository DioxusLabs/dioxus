use std::{marker::PhantomData, ops::ControlFlow};

use crate::{DocumentTester, Query, element::ResolvedElement};

/// A representation of a condition to be expected on the DOM.
pub trait Matcher<T> {
    type Output;
    fn matches(&self, actual: T) -> ControlFlow<Self::Output>;
}

/// Returns a [Matcher] which matches an element whose inner HTML is matched by the [Matcher]
/// `inner`.
pub fn inner_html(inner: impl Matcher<String>) -> impl for<'vdom> Matcher<ResolvedElement<'vdom>> {
    struct InnerHtmlMatcher<InnerMatcher: Matcher<String>>(InnerMatcher);

    impl<'vdom, InnerMatcher: Matcher<String>> Matcher<ResolvedElement<'vdom>>
        for InnerHtmlMatcher<InnerMatcher>
    {
        type Output = InnerMatcher::Output;

        fn matches(&self, element: ResolvedElement<'vdom>) -> ControlFlow<InnerMatcher::Output> {
            let inner_html = element.inner_html();
            self.0.matches(inner_html)
        }
    }

    InnerHtmlMatcher(inner)
}

/// Returns a [Matcher] which matches a `String` containing the given `substring`.
pub fn contains_string<'a>(substring: impl AsRef<str> + 'a) -> impl Matcher<String> + 'a {
    struct ContainingStringMatcher<Expected: AsRef<str>>(Expected);

    impl<Expected: AsRef<str>> Matcher<String> for ContainingStringMatcher<Expected> {
        type Output = ();
        fn matches(&self, actual: String) -> ControlFlow<Self::Output> {
            if actual.contains(self.0.as_ref()) {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        }
    }

    ContainingStringMatcher(substring)
}

/// Returns a [Matcher] which matches any data not matched by the given [Matcher] `inner`.
pub fn not<T>(inner: impl Matcher<T, Output = ()>) -> impl Matcher<T, Output = ()> {
    struct NotMatcher<T, InnerMatcher: Matcher<T, Output = ()>>(InnerMatcher, PhantomData<T>);

    impl<T, InnerMatcher: Matcher<T, Output = ()>> Matcher<T> for NotMatcher<T, InnerMatcher> {
        type Output = ();

        fn matches(&self, actual: T) -> ControlFlow<Self::Output> {
            match self.0.matches(actual) {
                ControlFlow::Continue(_) => ControlFlow::Break(()),
                ControlFlow::Break(_) => ControlFlow::Continue(()),
            }
        }
    }

    NotMatcher(inner, Default::default())
}

/// Returns a [Matcher] which matches a query selector matching the given `selector`.
pub fn query_selector(
    query: Query,
) -> impl for<'a> Matcher<&'a DocumentTester, Output = ResolvedElement<'a>> {
    struct QuerySelectorMatcher(Query);

    impl<'a> Matcher<&'a DocumentTester> for QuerySelectorMatcher {
        type Output = ResolvedElement<'a>;
        fn matches(&self, tester: &'a DocumentTester) -> ControlFlow<Self::Output> {
            if let Some(element) = tester.get_element(&self.0) {
                ControlFlow::Break(element)
            } else {
                ControlFlow::Continue(())
            }
        }
    }

    QuerySelectorMatcher(query)
}

/// Returns a [Matcher] which matches a query selector matching the given `selector`.
pub fn query_selector_all(
    query: Query,
) -> impl for<'a> Matcher<&'a DocumentTester, Output = Vec<ResolvedElement<'a>>> {
    struct QuerySelectorMatcher(Query);

    impl<'a> Matcher<&'a DocumentTester> for QuerySelectorMatcher {
        type Output = Vec<ResolvedElement<'a>>;
        fn matches(&self, tester: &'a DocumentTester) -> ControlFlow<Self::Output> {
            let elements = tester.get_elements(&self.0);
            if elements.is_empty() {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(elements)
            }
        }
    }

    QuerySelectorMatcher(query)
}
