use crate::element::ResolvedElement;
use test_that::{description::Description, matcher::Describable, prelude::Matcher};

pub use test_that::matchers::{containers::*, *};

/// Returns a [Matcher] which matches an element whose inner HTML is matched by the [Matcher]
/// `inner`.
pub fn inner_html(inner: impl Matcher<String>) -> impl for<'vdom> Matcher<ResolvedElement<'vdom>> {
    struct InnerHtmlMatcher<InnerMatcher>(InnerMatcher);

    impl<'vdom, InnerMatcher: Matcher<String>> Matcher<ResolvedElement<'vdom>>
        for InnerHtmlMatcher<InnerMatcher>
    {
        fn matches(&self, actual: &ResolvedElement<'vdom>) -> test_that::matcher::MatcherResult {
            let inner_html = actual.inner_html();
            self.0.matches(&inner_html)
        }
    }

    impl<'vdom, InnerMatcher: Matcher<String>> Describable for InnerHtmlMatcher<InnerMatcher> {
        fn describe(
            &self,
            matcher_result: test_that::matcher::MatcherResult,
        ) -> test_that::description::Description {
            Description::new()
                .text("Has inner HTML matching")
                .nested(self.0.describe(matcher_result))
        }
    }

    InnerHtmlMatcher(inner)
}
