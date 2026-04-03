use crate::{Matcher, element::ResolvedElement, result::TesterError};
use blitz_dom::{Document as _, SelectorList};
use dioxus_core::{Element, VirtualDom};
use dioxus_native_dom::{DioxusDocument, DocumentConfig};
use std::{
    marker::PhantomData,
    ops::ControlFlow,
    pin::{Pin, pin},
    task::{Context, Poll},
    time::Duration,
};
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
    pub(crate) fn get_element<'vdom>(
        &'vdom self,
        query: &SelectorList,
    ) -> Option<ResolvedElement<'vdom>> {
        self.document
            .query_selector_raw(query)
            .map(|node_id| self.node_id_to_element(node_id))
    }

    /// Immediately returns all already elements in the DOM satisfying the given [Query].
    ///
    /// Returns an error if the Query contains a syntactically invalid CSS selector.
    pub(crate) fn get_elements<'vdom>(
        &'vdom self,
        query: &SelectorList,
    ) -> Vec<ResolvedElement<'vdom>> {
        self.document
            .query_selector_all_raw(query)
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
    pub fn query(&'_ mut self, query: impl TryIntoSelector) -> impl Waitable {
        let selector = query
            .try_into_selector(&self.document)
            .expect("Invalid CSS selector");
        ElementCondition {
            data: self,
            query: selector,
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
    ) -> AllElementsCondition<'vdom> {
        let selector = query
            .try_into_selector(&self.document)
            .expect("Invalid CSS selector");
        AllElementsCondition {
            data: self,
            query: selector,
        }
    }
}

pub trait TryIntoSelector {
    fn try_into_selector(self, document: &DioxusDocument) -> Result<SelectorList, TesterError>;
}

impl TryIntoSelector for &str {
    fn try_into_selector(self, document: &DioxusDocument) -> Result<SelectorList, TesterError> {
        document.try_parse_selector_list(self).map_err(|_| {
            TesterError::InvalidCssSelector(format!("Invalid CSS selector '{}'", self))
        })
    }
}

struct QueryByTestId(String);

impl TryIntoSelector for QueryByTestId {
    fn try_into_selector(self, document: &DioxusDocument) -> Result<SelectorList, TesterError> {
        Ok(document
            .try_parse_selector_list(&format!(r#"[data-testid="{}"]"#, self.0))
            .expect("Selector with testid should always parse"))
    }
}

pub fn by_testid(testid: impl AsRef<str>) -> impl TryIntoSelector {
    QueryByTestId(testid.as_ref().to_string())
}

/// The maximum number of attempts [DocumentTester] will make to find a given element or make a
/// given assertion on the DOM before concluding that the element will not appear.
// TODO: Make this configurable.
const MAX_TRIES: usize = 5;

trait Waitable<'output> {
    type Output;
    async fn pump(&mut self);
    fn check(&'output self) -> ControlFlow<Self::Output>;
}

struct WaitableFuture<'output, W: Waitable<'output>> {
    waitable: W,
    state: WaitableFutureState,
    phantom: PhantomData<&'output ()>,
}

impl<'output, W: Waitable<'output>> WaitableFuture<'output, W> {
    fn new(waitable: W) -> Self {
        Self {
            waitable,
            state: WaitableFutureState::Init,
            phantom: Default::default(),
        }
    }
}

enum WaitableFutureState {
    Init,
    Pump(usize, Box<dyn Future<Output = ()>>),
}

impl<'output, W: Waitable<'output> + Unpin> Future for WaitableFuture<'output, W> {
    type Output = Result<W::Output, TesterError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state {
            WaitableFutureState::Init => match self.waitable.check() {
                ControlFlow::Break(data) => Poll::Ready(Ok(data)),
                ControlFlow::Continue(_) => {
                    self.state = WaitableFutureState::Pump(0, Box::new(self.waitable.pump()));
                    Poll::Pending
                }
            },
            WaitableFutureState::Pump(tries, mut pump_future) => {
                let pump_future_pin = pin!(pump_future);
                match pump_future_pin.poll(cx) {
                    Poll::Ready(_) => match self.waitable.check() {
                        ControlFlow::Break(data) => Poll::Ready(Ok(data)),
                        ControlFlow::Continue(_) => {
                            if tries >= MAX_TRIES {
                                Poll::Ready(Err(TesterError::NoSuchElementWithCssSelector(
                                    "TODO placeholder".to_string(),
                                )))
                            } else {
                                self.state = WaitableFutureState::Pump(
                                    tries + 1,
                                    Box::new(self.waitable.pump()),
                                );
                                Poll::Pending
                            }
                        }
                    },
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

pub trait ImmediateCondition<'output> {
    type Output: 'output;
    fn immediately(&'output self) -> Result<Self::Output, TesterError>;
}

impl<'output, T: Waitable<'output> + 'output> ImmediateCondition<'output> for T {
    type Output = <Self as Waitable<'output>>::Output;

    fn immediately(&'output self) -> Result<Self::Output, TesterError> {
        match self.check() {
            ControlFlow::Continue(_) => Err(TesterError::AssertionFailure("TODO".into())),
            ControlFlow::Break(b) => Ok(b),
        }
    }
}

pub struct ElementCondition<'vdom> {
    data: &'vdom mut DocumentTester,
    query: SelectorList,
}

impl<'vdom> Waitable<'vdom> for ElementCondition<'vdom> {
    type Output = ResolvedElement<'vdom>;
    async fn pump(&mut self) {
        self.data.pump().await;
    }

    fn check(&'vdom self) -> ControlFlow<Self::Output> {
        let data: &'vdom DocumentTester = self.data;
        if let Some(element) = data.get_element(&self.query) {
            ControlFlow::Break(element)
        } else {
            ControlFlow::Continue(())
        }
    }
}

impl<'vdom> IntoFuture for ElementCondition<'vdom> {
    type Output = <WaitableFuture<'vdom, Self> as Future>::Output;
    type IntoFuture = WaitableFuture<'vdom, Self>;

    fn into_future(self) -> Self::IntoFuture {
        WaitableFuture::new(self)
    }
}

impl<'vdom> ElementCondition<'vdom> {
    pub fn click(self) -> impl Future<Output = Result<(), TesterError>> + 'vdom {
        async move {
            let element = self.await?;
            element.click();
            Ok(())
        }
    }

    pub fn tap(self) -> impl Future<Output = Result<(), TesterError>> + 'vdom {
        self.click()
    }

    pub fn expect<M>(self, matcher: M) -> impl Waitable<'vdom>
    where
        M: for<'a> Matcher<ResolvedElement<'a>> + 'vdom,
    {
        MatcherCondition {
            element: self,
            matcher,
        }
    }
}

pub struct AllElementsCondition<'vdom> {
    data: &'vdom mut DocumentTester,
    query: SelectorList,
}

impl<'vdom> Waitable<'vdom> for AllElementsCondition<'vdom> {
    type Output = Vec<ResolvedElement<'vdom>>;

    async fn pump(&mut self) {
        self.data.pump().await;
    }

    fn check(&'vdom self) -> ControlFlow<Self::Output> {
        let elements = self.data.get_elements(&self.query);
        if elements.is_empty() {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(elements)
        }
    }
}

impl<'vdom> IntoFuture for AllElementsCondition<'vdom> {
    type Output = <WaitableFuture<'vdom, Self> as Future>::Output;
    type IntoFuture = WaitableFuture<'vdom, Self>;

    fn into_future(self) -> Self::IntoFuture {
        WaitableFuture::new(self)
    }
}

struct MatcherCondition<'vdom, M> {
    element: ElementCondition<'vdom>,
    matcher: M,
}

impl<'vdom, M> Waitable<'vdom> for MatcherCondition<'vdom, M>
where
    M: for<'a> Matcher<ResolvedElement<'a>> + 'vdom,
{
    type Output = ();

    async fn pump(&mut self) {
        self.element.pump().await;
    }

    fn check(&'vdom self) -> ControlFlow<Self::Output> {
        match self.element.check() {
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
            ControlFlow::Break(n) => self.matcher.matches(n),
        }
    }
}

impl<'vdom, M: Unpin> IntoFuture for MatcherCondition<'vdom, M>
where
    M: for<'a> Matcher<ResolvedElement<'a>> + 'vdom,
{
    type Output = <WaitableFuture<'vdom, Self> as Future>::Output;
    type IntoFuture = WaitableFuture<'vdom, Self>;

    fn into_future(self) -> Self::IntoFuture {
        WaitableFuture::new(self)
    }
}
