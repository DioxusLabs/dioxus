use crate::{DocumentTester, Matcher, TesterError, element::ResolvedElement};
use blitz_dom::SelectorList;
use std::{marker::PhantomData, ops::ControlFlow, pin::Pin};

/// The maximum number of attempts [DocumentTester] will make to find a given element or make a
/// given assertion on the DOM before concluding that the element will not appear.
// TODO: Make this configurable.
const MAX_TRIES: usize = 5;

trait Waitable {
    type Output;
    fn pump(&mut self) -> impl Future<Output = ()>;
    fn check(&self) -> ControlFlow<Self::Output>;

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
                            break Err(TesterError::NoSuchElementWithCssSelector(
                                "TODO placeholder".to_string(),
                            ));
                        }
                    }
                }
                self.pump().await;
            }
        })
    }
}

pub trait Matchable<M> {
    fn matches(&self, matcher: &M) -> ControlFlow<()>;
}

pub struct ElementCondition<'vdom> {
    data: &'vdom mut DocumentTester,
    query: SelectorList,
}

impl<'vdom> Waitable for ElementCondition<'vdom> {
    type Output = usize;

    async fn pump(&mut self) {
        let _ = self.data.pump().await;
    }

    fn check(&self) -> ControlFlow<Self::Output> {
        if let Some(element) = self.data.get_element(&self.query) {
            ControlFlow::Break(element)
        } else {
            ControlFlow::Continue(())
        }
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

impl<'vdom> ElementCondition<'vdom> {
    pub(crate) fn new(data: &'vdom mut DocumentTester, query: SelectorList) -> Self {
        Self { data, query }
    }

    pub async fn click(self) -> Result<(), TesterError> {
        let element = self.into_future().await?;
        element.click();
        Ok(())
    }

    pub fn tap(self) -> impl Future<Output = Result<(), TesterError>> + 'vdom {
        self.click()
    }

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

    pub fn immediately(&'vdom self) -> Result<ResolvedElement<'vdom>, TesterError> {
        match self.check() {
            ControlFlow::Continue(_) => Err(TesterError::AssertionFailure("TODO".into())),
            ControlFlow::Break(b) => {
                let node = self.data.node_id_to_element(b);
                Ok(node)
            }
        }
    }
}

pub struct AllElementsCondition<'vdom> {
    data: &'vdom mut DocumentTester,
    query: SelectorList,
}

impl<'vdom> AllElementsCondition<'vdom> {
    pub(crate) fn new(data: &'vdom mut DocumentTester, query: SelectorList) -> Self {
        Self { data, query }
    }

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

    pub fn immediately(&'vdom self) -> Result<Vec<ResolvedElement<'vdom>>, TesterError> {
        match self.check() {
            ControlFlow::Continue(_) => Err(TesterError::AssertionFailure("TODO".into())),
            ControlFlow::Break(node_ids) => Ok(node_ids
                .into_iter()
                .map(|node_id| self.data.node_id_to_element(node_id))
                .collect()),
        }
    }
}

impl<'vdom> Waitable for AllElementsCondition<'vdom> {
    type Output = Vec<usize>;

    async fn pump(&mut self) {
        let _ = self.data.pump().await;
    }

    fn check(&self) -> ControlFlow<Self::Output> {
        let elements = self.data.get_elements(&self.query);
        if elements.is_empty() {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(elements)
        }
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
}

impl<'vdom> IntoFuture for AllElementsCondition<'vdom> {
    type Output = Result<Vec<ResolvedElement<'vdom>>, TesterError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + 'vdom>>;

    fn into_future(mut self) -> Self::IntoFuture {
        Box::pin(async move {
            let node_ids = self.to_waitable_future().await?;
            Ok(node_ids
                .into_iter()
                .map(|node_id| self.data.node_id_to_element(node_id))
                .collect())
        })
    }
}

pub struct MatcherCondition<'vdom, M, W> {
    element: W,
    matcher: M,
    phantom: PhantomData<&'vdom ()>,
}

impl<'vdom, M, W> MatcherCondition<'vdom, M, W>
where
    W: Matchable<M>,
{
    pub fn immediately(&'vdom self) -> Result<(), TesterError> {
        match self.element.matches(&self.matcher) {
            ControlFlow::Continue(_) => Err(TesterError::AssertionFailure("TODO".into())),
            ControlFlow::Break(_) => Ok(()),
        }
    }
}

impl<'vdom, M, W> Waitable for MatcherCondition<'vdom, M, W>
where
    W: Waitable + Matchable<M>,
{
    type Output = ();

    async fn pump(&mut self) {
        self.element.pump().await;
    }

    fn check(&self) -> ControlFlow<Self::Output> {
        self.element.matches(&self.matcher)
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
