use crate::{
    Matcher,
    element::ResolvedElement,
    matcher::{query_selector, query_selector_all},
    result::TesterError,
};
use blitz_dom::{Document as _, SelectorList};
use dioxus_core::{Element, VirtualDom};
use dioxus_native_dom::{DioxusDocument, DocumentConfig};
use std::{ops::ControlFlow, time::Duration};
use tokio::time::{error::Elapsed, timeout};

/// The maximum time [DocumentTester] will wait for new events when running [DocumentTester::pump]
/// before concluding that no new events are forthcoming.
// TODO: Make this configurable.
const PUMP_TIMEOUT: Duration = Duration::from_millis(1000);

/// Returns a new [Tester] resulting from rendering the given [Element].
pub fn render(element: fn() -> Element) -> DocumentTester {
    DocumentTester::from_element(element)
}

/// A wrapper which allows querying and interacting with a DOM in Dioxus tests.
pub struct DocumentTester {
    document: DioxusDocument,
    now: f64,
    window_size: Option<(u32, u32)>,
}

impl DocumentTester {
    /// Constructs a new instance by rendering the given `element`.
    pub fn from_element(element: fn() -> Element) -> Self {
        let virtual_dom = VirtualDom::new(element);
        let document = DioxusDocument::new(virtual_dom, DocumentConfig::default());
        Self {
            document,
            now: 0.0,
            window_size: None,
        }
    }

    /// Constructs a new instance from the given [VirtualDom].
    pub fn from_virtual_dom(virtual_dom: VirtualDom) -> Self {
        let document = DioxusDocument::new(virtual_dom, DocumentConfig::default());
        Self {
            document,
            now: 0.0,
            window_size: None,
        }
    }

    /// Adds the given context to the root of this tester's virtual DOM.
    ///
    /// The context is available to all elements within the DOM.
    ///
    /// See [Dioxus documentation](https://dioxuslabs.com/learn/0.7/essentials/basics/context) for
    /// more information on context.
    pub fn with_root_context<T: Clone + 'static>(self, context: T) -> Self {
        self.document.vdom.provide_root_context(context);
        self
    }

    /// Sets the size of the window in pixels to which this DOM will virtually render.
    pub fn with_window_size(mut self, width: u32, height: u32) -> Self {
        self.window_size = Some((width, height));
        self
    }

    /// Performs a layout and build for the DOM managed by this tester.
    ///
    /// This method must be invoked before querying any elements.
    pub fn build(mut self) -> Self {
        self.document.inner.viewport_mut().window_size = self.window_size.unwrap_or((500, 800));
        self.document.initial_build();
        self.document.resolve(self.now);
        self
    }

    /// Resolve a single round of asynchronous operations via the async runtime and the Dioxus
    /// runtime.
    ///
    /// This performs a single round of one of the following:
    ///
    /// - Allow the runtime to process any events which have been dispatch, invoking the event
    ///   handlers.
    /// - Resolve a single round of async operations external to the Dioxus runtime, such as
    ///   network requests.
    ///
    /// For example, if you have a button whose event handler initiates a network request, then a
    /// single call to this method will invoke the event handler and run it until it performs the
    /// network request. A second invocation of this method will resolve the network request and
    /// continue the event handler from that point.
    ///
    /// ```no_run
    /// # use dioxus::prelude::*;
    /// # #[component]
    /// # fn AComponent() -> Element { rsx! { } }
    /// # async fn run_test() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut tester = dioxus_test::render(AComponent).build();
    /// tester.find_by_test_id("make-request-button")?.click();
    ///
    /// tester.pump().await?; // React to the click
    /// // Assert on the state of the UI while the network request is in flight.
    ///
    /// tester.pump().await?; // Receive the server response
    /// // Assert on the state of the UI after the response is received and the UI has been
    /// // rerendered.
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// If this method is invoked with no pending asynchronous operations, then it times out after
    /// one second and returns `Err(Elapsed)`.
    pub async fn pump(&mut self) -> Result<(), Elapsed> {
        timeout(PUMP_TIMEOUT, self.document.vdom.wait_for_work()).await?;
        while self.document.poll(None) {}
        Ok(())
    }

    /// Advance the internal clock by the given [Duration].
    ///
    /// This advances any CSS animations which may be in progress and recalculates the layout.
    pub async fn advance_time(&mut self, duration: Duration) {
        self.now += duration.as_secs_f64();
        self.document.resolve(self.now);
    }

    /// Returns a [TestElement] referencing the root DOM node managed by this tester.
    pub fn root<'vdom>(&'vdom self) -> ResolvedElement<'vdom> {
        ResolvedElement {
            document: &self.document,
            node: self.document.root_element(),
        }
    }

    /// Immediately returns the first element in the DOM satisfying the given [Query].
    ///
    /// If no such element already exists on the DOM, then this returns an error.
    ///
    /// Returns an error if the Query contains a syntactically invalid CSS selector.
    pub(crate) fn get_element<'vdom>(&'vdom self, query: &Query) -> Option<ResolvedElement<'vdom>> {
        self.document
            .query_selector_raw(query.list())
            .map(|node_id| self.node_id_to_element(node_id))
    }

    /// Immediately returns all already elements in the DOM satisfying the given [Query].
    ///
    /// Returns an error if the Query contains a syntactically invalid CSS selector.
    pub(crate) fn get_elements<'vdom>(&'vdom self, query: &Query) -> Vec<ResolvedElement<'vdom>> {
        self.document
            .query_selector_all_raw(query.list())
            .into_iter()
            .map(|node_id| self.node_id_to_element(node_id))
            .collect()
    }

    /// Returns the first element in the DOM satisfying the given [Query], waiting as necessary.
    ///
    /// This makes up to [MAX_TRIES] queries, running [Self::pump] between each one. If it reaches
    /// the limit and the element is still not present, it returns an error.
    ///
    /// Returns an error if the Query contains a syntactically invalid CSS selector.
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// # use dioxus_test::*;
    /// #[component]
    /// fn AComponent() -> Element {
    ///    let mut click_count = use_signal(|| 0);
    ///    rsx! {
    ///        button {
    ///            onclick: move |_| click_count += 1,
    ///            "Click me!"
    ///        }
    ///        div {
    ///            id: "click-count",
    ///            "Click count: {click_count}"
    ///        }
    ///    }
    /// }
    /// async fn run_test() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut tester = dioxus_test::render(AComponent).build();
    /// tester.query("#click-count").expect(inner_html(contains_string("Click count: 0"))).await?;
    /// tester.query("button").click().await?;
    /// tester.query("#click-count").expect(inner_html(contains_string("Click count: 1"))).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn query(
        &'_ mut self,
        query: impl TryIntoSelector,
    ) -> QueryPollFuture<'_, impl for<'a> Matcher<&'a DocumentTester, Output = ResolvedElement<'a>>>
    {
        let selector = query
            .try_into_selector(&self.document)
            .expect("Invalid CSS selector");
        let query = Query { list: selector };
        QueryPollFuture {
            data: self,
            query: query_selector(query),
        }
    }

    fn node_id_to_element(&'_ self, node_id: usize) -> ResolvedElement<'_> {
        let node = self
            .document
            .get_node(node_id)
            .expect("Element must be attached");
        ResolvedElement {
            document: &self.document,
            node,
        }
    }

    /// Returns all elements in the DOM satisfying the given [Query], waiting as necessary until the
    /// set is nonempty.
    ///
    /// This makes up to [MAX_TRIES] queries, running [Self::pump] between each one. If it reaches
    /// the limit and no matching elements are present, it returns an empty list.
    ///
    /// Returns an error if the Query contains a syntactically invalid CSS selector.
    pub fn query_all<'vdom>(
        &'vdom mut self,
        query: impl TryIntoSelector,
    ) -> QueryPollFuture<
        'vdom,
        impl for<'a> Matcher<&'a DocumentTester, Output = Vec<ResolvedElement<'a>>>,
    > {
        let selector = query
            .try_into_selector(&self.document)
            .expect("Invalid CSS selector");
        let query = Query { list: selector };
        QueryPollFuture {
            data: self,
            query: query_selector_all(query),
        }
    }
}

pub trait TryIntoSelector {
    fn try_into_selector(self, document: &DioxusDocument) -> Result<SelectorList, TesterError>;
}

impl TryIntoSelector for &str {
    fn try_into_selector(self, document: &DioxusDocument) -> Result<SelectorList, TesterError> {
        document.try_parse_selector_list(self).map_err(|err| {
            TesterError::InvalidCssSelector(format!("Invalid CSS selector '{}'", self))
        })
    }
}

/// Selects one or more elements in a DOM.
///
/// This can be by CSS or by the `data-testid` attribute.
pub struct Query {
    list: SelectorList,
}

impl Query {
    fn list(&self) -> &SelectorList {
        &self.list
    }
}

/// The maximum number of attempts [DocumentTester] will make to find a given element or make a
/// given assertion on the DOM before concluding that the element will not appear.
// TODO: Make this configurable.
const MAX_TRIES: usize = 5;

pub struct QueryPollFuture<'vdom, Q> {
    data: &'vdom mut DocumentTester,
    query: Q,
}

impl<'vdom, Q> QueryPollFuture<'vdom, Q>
where
    Q: for<'a> Matcher<&'a DocumentTester> + 'vdom,
{
    pub fn immediately(self) -> ControlFlow<<Q as Matcher<&'vdom DocumentTester>>::Output> {
        self.query.matches(self.data)
    }
}

impl<'vdom, Q> IntoFuture for QueryPollFuture<'vdom, Q>
where
    Q: for<'a> Matcher<&'a DocumentTester> + 'vdom,
{
    type Output = Result<<Q as Matcher<&'vdom DocumentTester>>::Output, TesterError>;
    type IntoFuture = std::pin::Pin<Box<dyn Future<Output = Self::Output> + 'vdom>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let mut tries = 0;
            loop {
                // this is weird because of lifetimes, could probably clean this up
                if self.query.matches(self.data).is_break() {
                    // Re-match to extract the value. We know it will match because nothing
                    // has changed since the check above.
                    break match self.query.matches(self.data) {
                        ControlFlow::Break(node) => Ok(node),
                        ControlFlow::Continue(_) => unreachable!(),
                    };
                }
                tries += 1;
                if tries >= MAX_TRIES {
                    break Err(TesterError::NoSuchElementWithCssSelector(
                        "TODO placeholder".to_string(),
                    ));
                }
                let _ = self.data.pump().await;
            }
        })
    }
}

impl<'vdom, Q> QueryPollFuture<'vdom, Q>
where
    Q: for<'a> Matcher<&'a DocumentTester, Output = ResolvedElement<'a>> + 'vdom,
{
    pub fn click(self) -> impl Future<Output = Result<(), TesterError>> + 'vdom {
        async move {
            let element = self.into_future().await?;
            element.click();
            Ok(())
        }
    }

    pub fn tap(self) -> impl Future<Output = Result<(), TesterError>> + 'vdom {
        self.click()
    }

    pub fn expect(
        self,
        matcher: impl Matcher<ResolvedElement<'vdom>> + 'vdom,
    ) -> impl Future<Output = Result<(), TesterError>> + 'vdom {
        async move {
            let element = self.into_future().await?;
            match matcher.matches(element) {
                ControlFlow::Continue(_) => Ok(()),
                ControlFlow::Break(_) => Err(TesterError::AssertionFailure(
                    "Expectation not met".to_string(),
                )),
            }
        }
    }
}
