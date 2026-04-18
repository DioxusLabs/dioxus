use crate::{DocumentTester, Matcher, TesterError, element::ResolvedElement};
use blitz_dom::SelectorList;
use std::{marker::PhantomData, ops::ControlFlow, pin::Pin};

/// The maximum number of attempts [DocumentTester] will make to find a given element or make a
/// given assertion on the DOM before concluding that the element will not appear.
// TODO: Make this configurable.
const MAX_TRIES: usize = 5;

trait EventLoopDriver {
    fn pump(&mut self) -> impl Future<Output = ()>;
}

trait Waitable: EventLoopDriver {
    type Output;
    fn check(&self) -> ControlFlow<Self::Output>;
    fn describe_failure(&self) -> TesterError;

    fn to_waitable_future<'vdom>(
        &'vdom mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, TesterError>> + 'vdom>>
    where
        Self: Sized,
    {
        Box::pin(async move {
            let mut tries = 0;
            loop {
                match self.check() {
                    ControlFlow::Break(data) => break Ok(data),
                    ControlFlow::Continue(_) => {
                        tries += 1;
                        if tries >= MAX_TRIES {
                            break Err(self.describe_failure());
                        }
                    }
                }
                self.pump().await;
            }
        })
    }
}

/// A represenation of a single element on the DOM which may already exist or may exist in the
/// future.
///
/// A test can make assertions on the element with [ElementCondition::expect]. The test decides
/// whether to make the assertion immediately or await it.
///
/// ```
/// use dioxus::prelude::*;
/// use dioxus_test::{eq, inner_html, render};
///
/// #[component]
/// fn MyComponent() -> Element {
///     rsx! {
///         div {
///              class: "test-component",
///              "Hello, world!"
///         }
///     }
/// }
///
/// # /* Make sure this also compiles as a doctest.
/// #[tokio::test]
/// # */
/// async fn my_component_renders_correctly() {
///     let mut tester = render(MyComponent).build();
///
///     // This works only if the element has already been rendered.
///     tester.query(".test-component").expect(inner_html(eq("Hello, world!"))).immediately().unwrap();
///     // This waits for the element to appear
///     tester.query(".test-component").expect(inner_html(eq("Hello, world!"))).await.unwrap();
/// }
/// # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(my_component_renders_correctly());
/// ```
///
/// A test can interact with the element once it appears, such as with [ElementCondition::click].
///
/// ```
/// use dioxus::prelude::*;
/// use dioxus_test::{eq, inner_html, render};
///
/// #[component]
/// fn MyComponent() -> Element {
///     rsx! {
///         button {
///              class: "test-button",
///              onclick: move |_| {},
///              "Click me"
///         }
///     }
/// }
///
/// # /* Make sure this also compiles as a doctest.
/// #[tokio::test]
/// # */
/// async fn my_component_has_a_button() {
///     let mut tester = render(MyComponent).build();
///     tester.query(".test-button").click().await.unwrap();
/// }
/// # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(my_component_has_a_button());
/// ```
///
/// A test can also fetch or await an `ElementCondition` directly to produce a [ResolvedElement]
/// for further assertions.
///
/// ```
/// use dioxus::prelude::*;
/// use dioxus_test::{eq, inner_html, render};
///
/// #[component]
/// fn MyComponent() -> Element {
///     rsx! {
///         div {
///              class: "test-component",
///              "Hello, world!"
///         }
///     }
/// }
///
/// # /* Make sure this also compiles as a doctest.
/// #[tokio::test]
/// # */
/// async fn my_component_renders_correctly() {
///     let mut tester = render(MyComponent).build();
///     let element = tester.query(".test-component");
///
///     // This works only if the element has already been rendered.
///     let content = element.immediately().unwrap().inner_html();
///     // This waits for the element to appear
///     let content = element.await.unwrap().inner_html();
///     assert_eq!(content, "Hello, world!");
/// }
/// # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(my_component_renders_correctly());
/// ```
pub struct ElementCondition<'vdom> {
    data: &'vdom mut DocumentTester,
    query: SelectorList,
    error: TesterError,
}

impl<'vdom> ElementCondition<'vdom> {
    pub(crate) fn new(
        data: &'vdom mut DocumentTester,
        query: SelectorList,
        error: TesterError,
    ) -> Self {
        Self { data, query, error }
    }

    /// Simulates the user clicking on the element this instance represents.
    ///
    /// This runs the event loop until the element appears, if necessary, up to [MAX_TRIES] times.
    /// It returns `Err` if the element does not appear.
    pub async fn click(self) -> Result<(), TesterError> {
        let element = self.into_future().await?;
        element.click();
        Ok(())
    }

    /// Synonym for [ElementCondition::click].
    pub fn tap(self) -> impl Future<Output = Result<(), TesterError>> + 'vdom {
        self.click()
    }

    /// Asserts that the given [Matcher] matches this element, either immediately or in the future.
    ///
    /// The test can require that the element already be present and matched:
    ///
    /// ```
    /// use dioxus::prelude::*;
    /// use dioxus_test::{eq, inner_html, render};
    ///
    /// #[component]
    /// fn MyComponent() -> Element {
    ///     rsx! {
    ///         div {
    ///              class: "test-component",
    ///              "Hello, world!"
    ///         }
    ///     }
    /// }
    ///
    /// # /* Make sure this also compiles as a doctest.
    /// #[test]
    /// # */
    /// fn my_component_renders_correctly() {
    ///     let mut tester = render(MyComponent).build();
    ///     tester
    ///         .query(".test-component")
    ///         .expect(inner_html(eq("Hello, world!")))
    ///         .immediately()
    ///         .unwrap();
    /// }
    /// # my_component_renders_correctly();
    /// ```
    ///
    /// Or the test can wait for the element to exist (if necessary) and the condition to be
    /// matched using `await`:
    ///
    /// ```
    /// use dioxus::prelude::*;
    /// use dioxus_test::{eq, inner_html, render};
    ///
    /// #[component]
    /// fn MyComponent() -> Element {
    ///     rsx! {
    ///         div {
    ///              class: "test-component",
    ///              "Hello, world!"
    ///         }
    ///     }
    /// }
    ///
    /// # /* Make sure this also compiles as a doctest.
    /// #[tokio::test]
    /// # */
    /// async fn my_component_renders_correctly() {
    ///     let mut tester = render(MyComponent).build();
    ///     tester
    ///         .query(".test-component")
    ///         .expect(inner_html(eq("Hello, world!")))
    ///         .await
    ///         .unwrap();
    /// }
    /// # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(my_component_renders_correctly());
    /// ```
    pub fn expect<M>(self, matcher: M) -> MatcherCondition<'vdom, M, ElementCondition<'vdom>>
    where
        M: for<'a> Matcher<ResolvedElement<'a>>,
    {
        MatcherCondition {
            element: self,
            matcher,
            phantom: Default::default(),
        }
    }

    /// Resolves the element represented by this instance without running the event loop.
    ///
    /// This can be used to obtain a [ResolvedElement] on which the test can operate when one knows
    /// that the element must already exist.
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// # use dioxus_test::*;
    /// #[component]
    /// fn AComponent() -> Element {
    ///    rsx! {
    ///        button {
    ///            onclick: move |_| {},
    ///            "Click me!"
    ///        }
    ///    }
    /// }
    /// # async fn run_test() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut tester = dioxus_test::render(AComponent).build();
    /// let query = tester.query("button");
    /// query.immediately()?.click();
    /// # Ok(())
    /// # }
    /// # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(run_test()).unwrap();
    /// ```
    pub fn immediately(&'vdom self) -> Result<ResolvedElement<'vdom>, TesterError> {
        match self.check() {
            ControlFlow::Continue(_) => Err(self.error.clone()),
            ControlFlow::Break(b) => {
                let node = self.data.node_id_to_element(b);
                Ok(node)
            }
        }
    }
}

impl<'vdom> EventLoopDriver for ElementCondition<'vdom> {
    async fn pump(&mut self) {
        let _ = self.data.pump().await;
    }
}

impl<'vdom> Waitable for ElementCondition<'vdom> {
    type Output = usize;

    fn check(&self) -> ControlFlow<Self::Output> {
        if let Some(element) = self.data.get_element(&self.query) {
            ControlFlow::Break(element)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn describe_failure(&self) -> TesterError {
        self.error.clone()
    }
}

impl<'vdom, M> Matchable<M> for ElementCondition<'vdom>
where
    M: for<'a> Matcher<ResolvedElement<'a>>,
{
    fn matches(&self, matcher: &M) -> ControlFlow<()> {
        match Waitable::check(self) {
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
            ControlFlow::Break(n) => {
                let node = self.data.node_id_to_element(n);
                matcher.matches(node)
            }
        }
    }

    fn explain_match_failure(&self, matcher: &M) -> String {
        match Waitable::check(self) {
            ControlFlow::Continue(_) => self.error.to_string(),
            ControlFlow::Break(n) => {
                let node = self.data.node_id_to_element(n);
                matcher.explain_failure(node)
            }
        }
    }
}

impl<'vdom> IntoFuture for ElementCondition<'vdom> {
    type Output = Result<ResolvedElement<'vdom>, TesterError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + 'vdom>>;

    fn into_future(mut self) -> Self::IntoFuture {
        Box::pin(async move {
            let node_id = self.to_waitable_future().await?;
            Ok(self.data.node_id_to_element(node_id))
        })
    }
}

/// A represenation of a set of elements on the DOM matching a query, currently or in the future.
///
/// A test can make assertions on the elements with [AllElementsCondition::expect]. The test decides
/// whether to make the assertion immediately or await it.
///
/// ```
/// use dioxus::prelude::*;
/// use dioxus_test::{empty, eq, inner_html, not, render};
///
/// #[component]
/// fn MyComponent() -> Element {
///     rsx! {
///         div {
///              class: "test-component",
///              "Hello, world!"
///         }
///     }
/// }
///
/// # /* Make sure this also compiles as a doctest.
/// #[tokio::test]
/// # */
/// async fn my_component_renders_correctly() {
///     let mut tester = render(MyComponent).build();
///
///     tester.query_all(".test-component").expect(not(empty())).immediately().unwrap();
///
///     tester.query_all(".this-selector-does-not-exist").expect(empty()).await.unwrap();
/// }
/// # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(my_component_renders_correctly());
/// ```
///
/// The test can also resolve the elements into a `Vec` of [ResolvedElement] with
/// [AllElementsCondition::immediately].
///
/// ```
/// use dioxus::prelude::*;
/// use dioxus_test::{empty, eq, inner_html, not, render};
///
/// #[component]
/// fn MyComponent() -> Element {
///     rsx! {
///         div {
///              class: "test-component",
///              "Hello, world!"
///         }
///     }
/// }
///
/// # /* Make sure this also compiles as a doctest.
/// #[test]
/// # */
/// fn my_component_renders_correctly() {
///     let mut tester = render(MyComponent).build();
///     let elements = tester.query_all(".test-component");
///     assert!(!elements.immediately().is_empty());
/// }
/// # my_component_renders_correctly();
/// ```
pub struct AllElementsCondition<'vdom> {
    data: &'vdom mut DocumentTester,
    query: SelectorList,
}

impl<'vdom> AllElementsCondition<'vdom> {
    pub(crate) fn new(data: &'vdom mut DocumentTester, query: SelectorList) -> Self {
        Self { data, query }
    }

    /// Asserts that the given [Matcher] matches this element collection, either immediately or in
    /// the future.
    ///
    /// The test can require that the element already be present and matched:
    ///
    /// ```
    /// use dioxus::prelude::*;
    /// use dioxus_test::{empty, eq, inner_html, not, render};
    ///
    /// #[component]
    /// fn MyComponent() -> Element {
    ///     rsx! {
    ///         div {
    ///              class: "test-component",
    ///              "Hello, world!"
    ///         }
    ///     }
    /// }
    ///
    /// # /* Make sure this also compiles as a doctest.
    /// #[test]
    /// # */
    /// fn my_component_renders_correctly() {
    ///     let mut tester = render(MyComponent).build();
    ///     tester
    ///         .query_all(".test-component")
    ///         .expect(not(empty()))
    ///         .immediately()
    ///         .unwrap();
    /// }
    /// # my_component_renders_correctly();
    /// ```
    ///
    /// Or the test can wait for the condition to be matched using `await`:
    ///
    /// ```
    /// use dioxus::prelude::*;
    /// use dioxus_test::{empty, eq, inner_html, not, render};
    ///
    /// #[component]
    /// fn MyComponent() -> Element {
    ///     rsx! {
    ///         div {
    ///              class: "test-component",
    ///              "Hello, world!"
    ///         }
    ///     }
    /// }
    ///
    /// # /* Make sure this also compiles as a doctest.
    /// #[tokio::test]
    /// # */
    /// async fn my_component_renders_correctly() {
    ///     let mut tester = render(MyComponent).build();
    ///     tester
    ///         .query_all(".test-component")
    ///         .expect(not(empty()))
    ///         .await
    ///         .unwrap();
    /// }
    /// # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(my_component_renders_correctly());
    /// ```
    pub fn expect<M>(self, matcher: M) -> MatcherCondition<'vdom, M, AllElementsCondition<'vdom>>
    where
        M: for<'a> Matcher<Vec<ResolvedElement<'a>>>,
    {
        MatcherCondition {
            element: self,
            matcher,
            phantom: Default::default(),
        }
    }

    pub fn immediately(&'vdom self) -> Vec<ResolvedElement<'vdom>> {
        let node_ids = self.data.get_elements(&self.query);
        node_ids
            .into_iter()
            .map(|node_id| self.data.node_id_to_element(node_id))
            .collect()
    }
}

impl<'vdom> EventLoopDriver for AllElementsCondition<'vdom> {
    async fn pump(&mut self) {
        let _ = self.data.pump().await;
    }
}

impl<'vdom, M> Matchable<M> for AllElementsCondition<'vdom>
where
    M: for<'a> Matcher<Vec<ResolvedElement<'a>>>,
{
    fn matches(&self, matcher: &M) -> ControlFlow<()> {
        let elements = self.data.get_elements(&self.query);
        let resolved: Vec<ResolvedElement<'_>> = elements
            .into_iter()
            .map(|node_id| self.data.node_id_to_element(node_id))
            .collect();
        matcher.matches(resolved)
    }

    fn explain_match_failure(&self, matcher: &M) -> String {
        let elements = self.data.get_elements(&self.query);
        let resolved: Vec<ResolvedElement<'_>> = elements
            .into_iter()
            .map(|node_id| self.data.node_id_to_element(node_id))
            .collect();
        matcher.explain_failure(resolved)
    }
}

/// A representation of a concrete assertion on an element or set of elements using a [Matcher].
///
/// The test can decide whether to make this assertion immediately with
/// [MatcherCondition::immediately] or to await it.
pub struct MatcherCondition<'vdom, M, W> {
    element: W,
    matcher: M,
    phantom: PhantomData<&'vdom ()>,
}

impl<'vdom, M, W> MatcherCondition<'vdom, M, W>
where
    W: Matchable<M>,
{
    /// Asserts that the matcher in this instance matches the element or set of elements
    /// immediately.
    ///
    /// This can be used, for example, to assert on the state of the DOM immediately after its
    /// initial render.
    ///
    /// ```
    /// use dioxus::prelude::*;
    /// use dioxus_test::{eq, inner_html, render};
    ///
    /// #[component]
    /// fn MyComponent() -> Element {
    ///     rsx! {
    ///         div {
    ///              class: "test-component",
    ///              "Hello, world!"
    ///         }
    ///     }
    /// }
    ///
    /// # /* Make sure this also compiles as a doctest.
    /// #[test]
    /// # */
    /// fn my_component_renders_correctly() {
    ///     let mut tester = render(MyComponent).build();
    ///     tester
    ///         .query(".test-component")
    ///         .expect(inner_html(eq("Hello, world!")))
    ///         .immediately()
    ///         .unwrap();
    /// }
    /// # my_component_renders_correctly();
    /// ```
    ///
    /// This can also be used if the state of the DOM has already advanced to a point where the
    /// assertion should pass because the test has already used an asynchronous assertion on it.
    ///
    /// ```
    /// use dioxus::prelude::*;
    /// use dioxus_test::{by_testid, eq, inner_html, render};
    ///
    /// #[component]
    /// fn MyComponent() -> Element {
    ///     let mut text = use_signal(|| "Click me!");
    ///     let mut label = use_signal(|| "Not clicked yet");
    ///     rsx! {
    ///         div {
    ///              "data-testid": "the-label",
    ///              {label}
    ///         }
    ///         button {
    ///              class: "test-button",
    ///              onclick: move |_| {
    ///                  *text.write() = "Clicked";
    ///                  *label.write() = "Now clicked";
    ///              },
    ///              {text}
    ///         }
    ///     }
    /// }
    ///
    /// # /* Make sure this also compiles as a doctest.
    /// #[tokio::test]
    /// # */
    /// async fn my_component_changes_button_text_on_click() {
    ///     let mut tester = render(MyComponent).build();
    ///     tester.query(".test-button").click().await;
    ///     tester.query(".test-button").expect(inner_html(eq("Clicked"))).await.unwrap();
    ///     tester
    ///         .query(by_testid("the-label"))
    ///         .expect(inner_html(eq("Now clicked")))
    ///         .immediately()
    ///         .unwrap();
    /// }
    /// # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(my_component_changes_button_text_on_click());
    /// ```
    ///
    /// This does not await any asynchronous actions, such as network activity to responding to user
    /// interaction. For example, the following test will fail:
    ///
    /// ```
    /// use dioxus::prelude::*;
    /// use dioxus_test::{eq, inner_html, render};
    ///
    /// #[component]
    /// fn MyComponent() -> Element {
    ///     let mut text = use_signal(|| "Click me!");
    ///     rsx! {
    ///         button {
    ///              class: "test-button",
    ///              onclick: move |_| {
    ///                  *text.write() = "Don't click any more!";
    ///              },
    ///              {text}
    ///         }
    ///     }
    /// }
    ///
    /// # /* Make sure this also compiles as a doctest.
    /// #[tokio::test]
    /// # */
    /// async fn my_component_changes_button_text_on_click() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut tester = render(MyComponent).build();
    ///     tester.query(".test-button").click().await;
    ///     tester
    ///         .query(".test-button")
    ///         .expect(inner_html(eq("Don't click any more!")))
    ///         .immediately()?;
    ///     Ok(())
    /// }
    /// # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(my_component_changes_button_text_on_click()).err().unwrap();
    /// ```
    pub fn immediately(&'vdom self) -> Result<(), TesterError> {
        match self.element.matches(&self.matcher) {
            ControlFlow::Continue(_) => Err(TesterError::AssertionFailure(
                self.element.explain_match_failure(&self.matcher),
            )),
            ControlFlow::Break(_) => Ok(()),
        }
    }
}

impl<'vdom, M, W> EventLoopDriver for MatcherCondition<'vdom, M, W>
where
    W: EventLoopDriver,
{
    fn pump(&mut self) -> impl Future<Output = ()> {
        self.element.pump()
    }
}

impl<'vdom, M, W> Waitable for MatcherCondition<'vdom, M, W>
where
    W: EventLoopDriver + Matchable<M>,
{
    type Output = ();

    fn check(&self) -> ControlFlow<Self::Output> {
        self.element.matches(&self.matcher)
    }

    fn describe_failure(&self) -> TesterError {
        TesterError::AssertionFailure(self.element.explain_match_failure(&self.matcher))
    }
}

impl<'vdom, M, W> IntoFuture for MatcherCondition<'vdom, M, W>
where
    Self: Waitable<Output = ()> + 'vdom,
{
    type Output = Result<(), TesterError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + 'vdom>>;

    fn into_future(mut self) -> Self::IntoFuture {
        Box::pin(async move { self.to_waitable_future().await })
    }
}

/// A datum on which one can invoke a [Matcher].
///
/// This allows [MatchableCondition] to work with both [ElementCondition] and
/// [AllElementsCondition].
pub trait Matchable<M> {
    fn matches(&self, matcher: &M) -> ControlFlow<()>;

    fn explain_match_failure(&self, matcher: &M) -> String;
}
