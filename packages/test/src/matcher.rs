use crate::element::ResolvedElement;

pub trait Matcher<T> {
    fn matches(&self, actual: &T) -> bool;
}

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

pub fn contains_string<'a>(substring: impl AsRef<str> + 'a) -> impl Matcher<String> + 'a {
    struct ContainingStringMatcher<Expected: AsRef<str>>(Expected);

    impl<Expected: AsRef<str>> Matcher<String> for ContainingStringMatcher<Expected> {
        fn matches(&self, actual: &String) -> bool {
            actual.contains(self.0.as_ref())
        }
    }

    ContainingStringMatcher(substring)
}
