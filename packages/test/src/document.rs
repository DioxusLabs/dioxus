use crate::{
    condition::{AllElementsCondition, ElementCondition},
    element::ResolvedElement,
    result::TesterError,
};
use blitz_dom::{Document as _, SelectorList};
use dioxus_core::{Element, VirtualDom};
use dioxus_native_dom::{DioxusDocument, DocumentConfig};
use std::time::Duration;
use tokio::time::{error::Elapsed, timeout};

/// The maximum time [DocumentTester] will wait for new events when running [DocumentTester::pump]
/// before concluding that no new events are forthcoming.
// TODO: Make this configurable.
const PUMP_TIMEOUT: Duration = Duration::from_millis(1000);

/// Returns a new [DocumentTester] resulting from rendering the given [Element].
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
    /// tester.query("make-request-button").click().await;
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

    /// Returns an element referencing the root DOM node managed by this tester.
    ///
    /// This allows interacting with and asserting on the root element. However, there is no support
    /// for awaiting expectations. If the test must await an expectation on the root element use
    /// [Self::query] with the CSS selector `:root`.
    pub fn root<'vdom>(&'vdom self) -> ResolvedElement<'vdom> {
        ResolvedElement {
            document: &self.document,
            node: self.document.root_element(),
        }
    }

    /// Immediately returns the first element in the DOM satisfying the given query.
    ///
    /// If no such element already exists on the DOM, then this returns an error.
    ///
    /// Returns an error if the Query contains a syntactically invalid CSS selector.
    pub(crate) fn get_element(&self, query: &SelectorList) -> Option<usize> {
        self.document.query_selector_raw(query)
    }

    /// Immediately returns all already elements in the DOM satisfying the given query.
    ///
    /// Returns an error if the Query contains a syntactically invalid CSS selector.
    pub(crate) fn get_elements(&self, query: &SelectorList) -> Vec<usize> {
        self.document.query_selector_all_raw(query).to_vec()
    }

    /// Returns a representation of first element in the DOM satisfying the given query.
    ///
    /// The query can be anything which dereferences to a `str`, including `&str` and `String`. This
    /// method then interprets it as a CSS selector. Alternatively, one can select by testid with
    /// [by_testid].
    ///
    /// The test can:
    ///
    /// - await the matching element by driving the event loop until it appears,
    /// - immediately resolve the element in order to assert on or interact with it, or
    /// - make an assertion and drive the event loop until that assertion to be true.
    ///
    /// See [ElementCondition] for more.
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
    /// # async fn run_test() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut tester = dioxus_test::render(AComponent).build();
    /// tester.query("#click-count").expect(inner_html(contains_string("Click count: 0"))).await?;
    /// tester.query("button").click().await?;
    /// tester.query("#click-count").expect(inner_html(contains_string("Click count: 1"))).await?;
    /// # Ok(())
    /// # }
    /// # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(run_test()).unwrap();
    /// ```
    ///
    /// Panics if the query contains a syntactically invalid CSS selector.
    pub fn query(&mut self, query: impl TryIntoSelector) -> ElementCondition<'_> {
        let error = query.to_tester_error();
        let selector = query
            .try_into_selector(&self.document)
            .expect("Invalid CSS selector");
        ElementCondition::new(self, selector, error)
    }

    /// Returns a representation of elements in the DOM satisfying the given query.
    ///
    /// The query can be anything which dereferences to a `str`, including `&str` and `String`. This
    /// method then interprets it as a CSS selector. Alternatively, one can select by testid with
    /// [by_testid].
    ///
    /// The test can immediately resolve the set of elements in order to assert on or interact with
    /// them, or it can make an assertion and drive the event loop until that assertion to be true.
    /// See [AllElementsCondition] for more.
    ///
    /// Panics if the query contains a syntactically invalid CSS selector.
    pub fn query_all(&mut self, query: impl TryIntoSelector) -> AllElementsCondition<'_> {
        let selector = query
            .try_into_selector(&self.document)
            .expect("Invalid CSS selector");
        AllElementsCondition::new(self, selector)
    }

    pub(crate) fn node_id_to_element<'vdom>(&'vdom self, node_id: usize) -> ResolvedElement<'vdom> {
        let node = self
            .document
            .get_node(node_id)
            .expect("Element must be attached");
        ResolvedElement {
            document: &self.document,
            node,
        }
    }
}

/// A value which can be turned into a CSS selector to query the DOM.
///
/// This is implemented for all types which dereference to `str`, including `&str` and `String`.
///
/// One can also select by [testid](https://testing-library.com/docs/queries/bytestid/) using the
/// function [by_testid].
pub trait TryIntoSelector {
    fn try_into_selector(self, document: &DioxusDocument) -> Result<SelectorList, TesterError>;

    fn to_tester_error(&self) -> TesterError;
}

impl<T: AsRef<str>> TryIntoSelector for T {
    fn try_into_selector(self, document: &DioxusDocument) -> Result<SelectorList, TesterError> {
        document
            .try_parse_selector_list(self.as_ref())
            .map_err(|_| {
                TesterError::InvalidCssSelector(format!("Invalid CSS selector '{}'", self.as_ref()))
            })
    }

    fn to_tester_error(&self) -> TesterError {
        TesterError::NoSuchElementWithCssSelector(self.as_ref().into())
    }
}

struct QueryByTestId(String);

impl TryIntoSelector for QueryByTestId {
    fn try_into_selector(self, document: &DioxusDocument) -> Result<SelectorList, TesterError> {
        Ok(document
            .try_parse_selector_list(&format!(r#"[data-testid="{}"]"#, self.0))
            .expect("Selector with testid should always parse"))
    }

    fn to_tester_error(&self) -> TesterError {
        TesterError::NoSuchElementWithTestId(self.0.clone())
    }
}

/// Returns a query selector matching elements with the given value in the `data-testid` attribute.
///
/// ```
/// use dioxus::prelude::*;
/// use dioxus_test::{by_testid, eq, inner_html, render};
///
/// #[component]
/// fn MyComponent() -> Element {
///     rsx! {
///         div {
///              "data-testid": "the-label",
///              "Label content"
///         }
///     }
/// }
///
/// let mut tester = render(MyComponent).build();
/// tester
///     .query(by_testid("the-label"))
///     .expect(inner_html(eq("Label content")))
///     .immediately()
///     .unwrap();
/// ```
///
/// This attribute is a common convention for marking DOM components with which tests interact. Find
/// more information [here](https://testing-library.com/docs/queries/bytestid/).
pub fn by_testid(testid: impl AsRef<str>) -> impl TryIntoSelector {
    QueryByTestId(testid.as_ref().to_string())
}
