use crate::TestElement;
use blitz_dom::Document as _;
use dioxus_core::{Element, VirtualDom};
use dioxus_native_dom::{DioxusDocument, DocumentConfig};
use std::time::Duration;
use tokio::time::{error::Elapsed, timeout};

const PUMP_TIMEOUT: Duration = Duration::from_millis(1000);

/// Returns a new [Tester] resulting from rendering the given [Element].
pub fn render(element: fn() -> Element) -> Tester {
    Tester::from_element(element)
}

/// A wrapper which allows querying and interacting with a DOM in Dioxus tests.
pub struct Tester {
    document: DioxusDocument,
    now: f64,
    window_size: Option<(u32, u32)>,
}

impl Tester {
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
    pub fn root<'vdom>(&'vdom self) -> TestElement<'vdom> {
        TestElement {
            document: &self.document,
            node: self.document.root_element(),
        }
    }

    /// Returns a [TestElement] referencing the first DOM node with the given test ID.
    ///
    /// By convention, the custom HTML attribute `data-testid` specifies a test ID which can be used
    /// to find elements used in tests. This is supported by multiple testing frameworks. See
    /// [testing library documentation](https://testing-library.com/docs/queries/bytestid/) for more
    /// information.
    ///
    /// Returns an error if the CSS seelctor itself is invalid or if no node has the test ID.
    pub fn find_by_test_id<'vdom>(
        &'vdom self,
        test_id: &str,
    ) -> Result<TestElement<'vdom>, TesterError> {
        let node_id = self
            .document
            .query_selector(&format!("[data-testid=\"{test_id}\"]"))
            .expect("Error parsing selector")
            .ok_or_else(|| TesterError::NoSuchElementWithTestId(test_id.into()))?;
        let node = self
            .document
            .get_node(node_id)
            .expect("Element must be attached");
        Ok(TestElement {
            document: &self.document,
            node,
        })
    }

    /// Returns a `Vec` of  [TestElement] referencing each DOM node matching the given CSS selector.
    ///
    /// Returns an error if the CSS itself is invalid.
    pub fn find_by_css_selector<'vdom>(
        &'vdom self,
        selector: &str,
    ) -> Result<Vec<TestElement<'vdom>>, TesterError> {
        let node_ids = self
            .document
            .query_selector_all(selector)
            .map_err(|_| TesterError::InvalidCssSelector(selector.into()))?;
        Ok(node_ids
            .into_iter()
            .map(|node_id| {
                let node = self
                    .document
                    .get_node(node_id)
                    .expect("Element must be attached");
                TestElement {
                    document: &self.document,
                    node,
                }
            })
            .collect())
    }

    /// Returns a [TestElement] referencing the first DOM node matching the given CSS selector.
    ///
    /// Returns an error if the CSS seelctor itself is invalid or if no node matches the selector.
    pub fn find_first_by_css_selector<'vdom>(
        &'vdom self,
        selector: &str,
    ) -> Result<TestElement<'vdom>, TesterError> {
        let node_id = self
            .document
            .query_selector(selector)
            .map_err(|_| TesterError::InvalidCssSelector(selector.into()))?
            .ok_or_else(|| TesterError::NoSuchElementWithCssSelector(selector.into()))?;
        let node = self
            .document
            .get_node(node_id)
            .expect("Element must be attached");
        Ok(TestElement {
            document: &self.document,
            node,
        })
    }
}

#[derive(Debug)]
pub enum TesterError {
    /// The given CSS selector had invalid syntax.
    InvalidCssSelector(String),

    /// No element with the test ID, as given by the HTML attribute `data-testid`, was found in the
    /// DOM.
    NoSuchElementWithTestId(String),

    /// No element matching the given CSS selector was found in the DOM.
    NoSuchElementWithCssSelector(String),
}

impl std::fmt::Display for TesterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TesterError::InvalidCssSelector(selector) => {
                write!(f, "Invalid CSS selector {selector}")
            }
            TesterError::NoSuchElementWithTestId(id) => {
                write!(f, "No such element with test ID {id}")
            }
            TesterError::NoSuchElementWithCssSelector(selector) => {
                write!(f, "No such element with CSS selector {selector}")
            }
        }
    }
}

impl std::error::Error for TesterError {}
