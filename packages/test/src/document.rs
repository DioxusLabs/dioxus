use crate::{Matcher, element::ResolvedElement, result::TesterError};
use blitz_dom::Document as _;
use dioxus_core::{Element, VirtualDom};
use dioxus_native_dom::{DioxusDocument, DocumentConfig};
use std::time::Duration;
use tokio::time::{error::Elapsed, timeout};

const PUMP_TIMEOUT: Duration = Duration::from_millis(1000);
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

    pub fn get_element<'vdom>(
        &'vdom self,
        query: Query,
    ) -> Result<ResolvedElement<'vdom>, TesterError> {
        let node_id = self.query_element(&query)?.ok_or(query.into_error())?;
        Ok(self.node_id_to_element(node_id))
    }

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

    pub async fn click(&mut self, query: Query) -> crate::Result<()> {
        Ok(self.wait_for_element(query).await?.click())
    }

    pub async fn tap(&mut self, query: Query) -> crate::Result<()> {
        self.click(query).await
    }

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

pub enum Query {
    ByCss(String),
    ByTestId(String),
}

impl Query {
    pub fn by_css(selector: impl AsRef<str>) -> Self {
        Self::ByCss(selector.as_ref().into())
    }

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
