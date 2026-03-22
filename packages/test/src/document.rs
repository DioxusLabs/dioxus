use crate::{Matcher, element::ResolvedElement, result::TesterError};
use blitz_dom::Document as _;
use dioxus_core::{Element, VirtualDom};
use dioxus_native_dom::{DioxusDocument, DocumentConfig};
use std::time::Duration;
use tokio::time::{error::Elapsed, timeout};

/// The maximum time [DocumentTester] will wait for new events when running [DocumentTester::pump]
/// before concluding that no new events are forthcoming.
// TODO: Make this configurable.
const PUMP_TIMEOUT: Duration = Duration::from_millis(1000);

/// The maximum number of attempts [DocumentTester] will make to find a given element or make a
/// given assertion on the DOM before concluding that the element will not appear.
// TODO: Make this configurable.
const MAX_TRIES: usize = 5;

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
    pub fn get_element<'vdom>(
        &'vdom self,
        query: Query,
    ) -> Result<ResolvedElement<'vdom>, TesterError> {
        let node_id = self.query_element(&query)?.ok_or(query.into_error())?;
        Ok(self.node_id_to_element(node_id))
    }

    /// Returns the first element in the DOM satisfying the given [Query], waiting as necessary.
    ///
    /// This makes up to [MAX_TRIES] queries, running [Self::pump] between each one. If it reaches
    /// the limit and the element is still not present, it returns an error.
    ///
    /// Returns an error if the Query contains a syntactically invalid CSS selector.
    pub async fn wait_for_element(
        &'_ mut self,
        query: Query,
    ) -> crate::Result<ResolvedElement<'_>> {
        let mut tries = 0;
        let node_id = loop {
            if let Some(node_id) = self.query_element(&query)? {
                break Ok(node_id);
            }
            tries += 1;
            if tries > MAX_TRIES {
                break Err(query.into_error());
            }
            let _ = self.pump().await;
        }?;
        Ok(self.node_id_to_element(node_id))
    }

    fn query_element(&self, query: &Query) -> Result<Option<usize>, TesterError> {
        Ok(self
            .document
            .query_selector(query.as_css())
            .map_err(|_| TesterError::InvalidCssSelector(query.as_css().to_string()))?)
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

    /// Immediately returns all already elements in the DOM satisfying the given [Query].
    ///
    /// Returns an error if the Query contains a syntactically invalid CSS selector.
    pub fn get_elements<'vdom>(
        &'vdom self,
        query: &Query,
    ) -> Result<Vec<ResolvedElement<'vdom>>, TesterError> {
        let node_ids = self.query_elements(&query)?;
        Ok(node_ids
            .into_iter()
            .map(|node_id| self.node_id_to_element(node_id))
            .collect())
    }

    /// Returns all elements in the DOM satisfying the given [Query], waiting as necessary until the
    /// set is nonempty.
    ///
    /// This makes up to [MAX_TRIES] queries, running [Self::pump] between each one. If it reaches
    /// the limit and no matching elements are present, it returns an empty list.
    ///
    /// Returns an error if the Query contains a syntactically invalid CSS selector.
    pub async fn wait_for_elements<'vdom>(
        &'vdom mut self,
        query: Query,
    ) -> crate::Result<Vec<ResolvedElement<'vdom>>> {
        let mut tries = 0;
        let node_ids = loop {
            let node_ids = self.query_elements(&query)?;
            if node_ids.is_empty() {
                tries += 1;
                if tries > MAX_TRIES {
                    break Ok(vec![]);
                }
            } else {
                break Ok(node_ids);
            }
            let _ = self.pump().await;
        }?;
        Ok(node_ids
            .into_iter()
            .map(|node_id| self.node_id_to_element(node_id))
            .collect())
    }

    fn query_elements(&self, query: &Query) -> Result<Vec<usize>, TesterError> {
        Ok(self
            .document
            .query_selector_all(query.as_css())
            .map_err(|_| TesterError::InvalidCssSelector(query.as_css().to_string()))?
            .to_vec())
    }

    /// Simulates a click on the first element in the DOM matching the given [Query].
    ///
    /// This waits as necessary for the element to appear if it has not already. It uses the same
    /// logic as [Self::wait_for_element].
    pub async fn click(&mut self, query: Query) -> crate::Result<()> {
        Ok(self.wait_for_element(query).await?.click())
    }

    /// Synonym of [Self::click].
    pub async fn tap(&mut self, query: Query) -> crate::Result<()> {
        self.click(query).await
    }

    /// Asserts that the given [Matcher] matches the first element on the DOM matching the given
    /// [Query].
    ///
    /// This requires that the condition specified in `expectation` already be true at the time the
    /// method is invoked. It does not run the event loop.
    ///
    /// This returns `Ok(())` if the condition in `expectation` is true and an error otherwise.
    pub fn expect_immediately(
        &mut self,
        query: Query,
        expectation: impl for<'a> Matcher<ResolvedElement<'a>>,
    ) -> crate::Result<()> {
        if let Some(node_id) = self.query_element(&query)? {
            let element = self.node_id_to_element(node_id);
            if !expectation.matches(&element) {
                return Err(TesterError::AssertionFailure("TODO".into()));
            }
        }
        Ok(())
    }

    /// Asserts that the given [Matcher] eventually matches the first element on the DOM matching
    /// the given [Query] after the event loop runs.
    ///
    /// This runs up to [MAX_TRIES] checks of `expectation`, invoking [Self::pump] between each
    /// check. If the expectation is still not met after that, it returns an error.
    ///
    /// This returns `Ok(())` if the condition in `expectation` is eventually met.
    pub async fn expect_eventually(
        &mut self,
        query: Query,
        expectation: impl for<'a> Matcher<ResolvedElement<'a>>,
    ) -> crate::Result<()> {
        let mut tries = 0;
        loop {
            if let Some(node_id) = self.query_element(&query)? {
                let element = self.node_id_to_element(node_id);
                if expectation.matches(&element) {
                    break Ok(());
                } else {
                    tries += 1;
                    if tries > MAX_TRIES {
                        break Err(TesterError::AssertionFailure("TODO".into()));
                    }
                }
                drop(element);
            } else {
                tries += 1;
                if tries > MAX_TRIES {
                    break Err(query.into_error());
                }
            }
            let _ = self.pump().await;
        }
    }
}

/// Selects one or more elements in a DOM.
///
/// This can be by CSS or by the `data-testid` attribute.
pub enum Query {
    ByCss(String),
    ByTestId(String),
}

impl Query {
    /// Returns a [Query] which selects elements by the given CSS selector.
    pub fn by_css(selector: impl AsRef<str>) -> Self {
        Self::ByCss(selector.as_ref().into())
    }

    /// Returns a [Query] which selects elements by the value of its `data-testid` attribute.
    pub fn by_test_id(test_id: impl AsRef<str>) -> Self {
        Self::ByTestId(format!("[data-testid=\"{}\"]", test_id.as_ref()))
    }

    fn as_css(&self) -> &str {
        match self {
            Query::ByCss(s) => &s,
            Query::ByTestId(s) => &s,
        }
    }

    fn into_error(self) -> TesterError {
        match self {
            Query::ByCss(s) => TesterError::NoSuchElementWithCssSelector(s),
            Query::ByTestId(s) => TesterError::NoSuchElementWithTestId(s), // TODO
        }
    }
}
