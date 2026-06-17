use std::{any::Any, cell::RefCell, panic::AssertUnwindSafe, rc::Rc};

use crate::{
    ComponentFunction, Element, WriteMutations,
    innerlude::{CapturedPanic, ElementRef, ScopeOrder},
    scopes::{LastRenderedNode, ScopeId},
    suspense::{SuspenseContext, SuspenseDriver},
    virtual_dom::VirtualDom,
};

/// Type-erased interface for a plain component's props and render function.
///
/// This handles the generic `<F, P, M>` parameters of [`BodyDriver`] behind
/// a trait object so that [`RenderDriver::Body`] can store any component.
pub(crate) trait BodyProps: 'static {
    fn as_any(&self) -> &dyn Any;

    /// Whether `other` renders the same component as this driver.
    fn same_component(&self, other: &dyn BodyProps) -> bool;

    /// Make this driver's props equal to `new_driver`'s. Returns whether the
    /// props were equal and the scope can be memoized.
    fn memoize(&self, new_driver: &dyn Any) -> bool;

    /// A fresh instance with cloned props.
    fn duplicate(&self) -> Rc<dyn BodyProps>;

    /// Run the component body and return the rendered element.
    fn render(&self) -> Element;
}

/// The rendering lifecycle driver for a scope.
///
/// Every scope owns exactly one driver: plain components use
/// [`RenderDriver::Body`], which owns the component function and its props;
/// suspense boundaries use [`RenderDriver::Suspense`], which owns the
/// [`SuspenseContext`] and manages children/fallback rendering.
///
/// Because this is an enum (not a trait object), its methods can be generic
/// over `M: WriteMutations`, using the same pattern as the rest of the codebase.
#[derive(Clone)]
pub(crate) enum RenderDriver {
    Body(Rc<dyn BodyProps>),
    Suspense(Rc<SuspenseDriver>),
}

impl RenderDriver {
    /// Whether `other` renders the same component as this driver.
    pub fn same_component(&self, other: &RenderDriver) -> bool {
        match (self, other) {
            (RenderDriver::Body(a), RenderDriver::Body(b)) => a.same_component(&**b),
            (RenderDriver::Suspense(_), RenderDriver::Suspense(_)) => true,
            _ => false,
        }
    }

    /// Update this driver's props to match `other`'s. Returns `true` if the
    /// props were equal (memoized).
    pub fn memoize(&self, other: &RenderDriver) -> bool {
        match (self, other) {
            (RenderDriver::Body(a), RenderDriver::Body(b)) => a.memoize(b.as_any()),
            (RenderDriver::Suspense(a), RenderDriver::Suspense(b)) => a.memoize(b),
            _ => false,
        }
    }

    /// A fresh driver instance with cloned props.
    pub fn duplicate(&self) -> RenderDriver {
        match self {
            RenderDriver::Body(b) => RenderDriver::Body(b.duplicate()),
            RenderDriver::Suspense(s) => RenderDriver::Suspense(Rc::new(s.duplicate())),
        }
    }

    /// Mount this scope's output.
    pub fn create<M: WriteMutations>(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        new: bool,
        parent: Option<ElementRef>,
        to: Option<&mut M>,
    ) -> usize {
        match self {
            RenderDriver::Body(b) => body_create(b, dom, scope_id, new, parent, to),
            RenderDriver::Suspense(s) => s.create(dom, scope_id, new, parent, to),
        }
    }

    /// Diff this scope's output against its current props.
    pub fn diff<M: WriteMutations>(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut M>,
    ) {
        match self {
            RenderDriver::Body(b) => body_diff(b, dom, scope_id, to),
            RenderDriver::Suspense(s) => s.diff(dom, scope_id, to),
        }
    }

    /// Remove this scope's output.
    pub fn remove<M: WriteMutations>(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut M>,
        destroy_component_state: bool,
        replace_with: Option<usize>,
    ) {
        match self {
            RenderDriver::Body(_) => {
                remove_rendered_output(dom, scope_id, to, destroy_component_state, replace_with)
            }
            RenderDriver::Suspense(s) => {
                s.remove(dom, scope_id, to, destroy_component_state, replace_with)
            }
        }
    }

    /// If this driver is a suspense boundary, return its context.
    pub fn suspense_context(&self) -> Option<SuspenseContext> {
        match self {
            RenderDriver::Suspense(s) => Some(s.context()),
            _ => None,
        }
    }
}

/// Remove a scope's rendered output from the DOM, and drop the scope when
/// `destroy_component_state` is set. Shared by body and suspense drivers.
pub(crate) fn remove_rendered_output<M: WriteMutations>(
    dom: &mut VirtualDom,
    scope_id: ScopeId,
    to: Option<&mut M>,
    destroy_component_state: bool,
    replace_with: Option<usize>,
) {
    if let Some(node) = dom.scopes[scope_id.0].last_rendered_node.clone() {
        node.remove_node_inner(dom, to, destroy_component_state, replace_with)
    };

    if destroy_component_state {
        dom.drop_scope(scope_id);
    }
}

/// Mount a plain component scope's output.
fn body_create<M: WriteMutations>(
    body: &Rc<dyn BodyProps>,
    dom: &mut VirtualDom,
    scope_id: ScopeId,
    new: bool,
    parent: Option<ElementRef>,
    to: Option<&mut M>,
) -> usize {
    if new {
        let body_element = dom.run_scope_with(scope_id, || body.render());
        dom.scopes[scope_id.0].last_rendered_node = Some(LastRenderedNode::new(body_element));

        // If our scope landed in `dirty_scopes` during its initial render
        // (e.g. a hook synchronously queued an update for itself), drain the
        // entry now so we don't re-process the same scope after creation.
        let height = dom.runtime.get_state(scope_id).height;
        dom.dirty_scopes.remove(&ScopeOrder::new(height, scope_id));
    }

    let new_node = dom.scopes[scope_id.0]
        .last_rendered_node
        .clone()
        .expect("Component to be mounted");

    dom.create_scope(to, scope_id, new_node, parent)
}

/// Diff a plain component scope against its current output.
fn body_diff<M: WriteMutations>(
    body: &Rc<dyn BodyProps>,
    dom: &mut VirtualDom,
    scope_id: ScopeId,
    to: Option<&mut M>,
) {
    let element = dom.run_scope_with(scope_id, || body.render());
    dom.diff_scope(to, scope_id, element);
}

/// The concrete implementation of [`BodyProps`] for a given component function.
pub(crate) struct BodyDriver<F: ComponentFunction<P, M>, P, M> {
    render_fn: F,
    memo: fn(&mut P, &P) -> bool,
    props: RefCell<P>,
    name: &'static str,
    phantom: std::marker::PhantomData<M>,
}

impl<F: ComponentFunction<P, M> + Clone, P: Clone + 'static, M: 'static> BodyDriver<F, P, M> {
    pub fn new(
        render_fn: F,
        memo: fn(&mut P, &P) -> bool,
        props: P,
        name: &'static str,
    ) -> BodyDriver<F, P, M> {
        BodyDriver {
            render_fn,
            memo,
            props: RefCell::new(props),
            name,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<F: ComponentFunction<P, M> + Clone, P: Clone + 'static, M: 'static> BodyProps
    for BodyDriver<F, P, M>
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn same_component(&self, other: &dyn BodyProps) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| other.render_fn.fn_ptr() == self.render_fn.fn_ptr())
    }

    fn memoize(&self, new_driver: &dyn Any) -> bool {
        match new_driver.downcast_ref::<Self>() {
            Some(new) => (self.memo)(&mut self.props.borrow_mut(), &new.props.borrow()),
            None => false,
        }
    }

    fn duplicate(&self) -> Rc<dyn BodyProps> {
        Rc::new(Self {
            render_fn: self.render_fn.clone(),
            memo: self.memo,
            props: RefCell::new(self.props.borrow().clone()),
            name: self.name,
            phantom: std::marker::PhantomData,
        })
    }

    fn render(&self) -> Element {
        fn render_inner(_name: &str, res: Result<Element, Box<dyn Any + Send>>) -> Element {
            match res {
                Ok(node) => node,
                Err(err) => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        tracing::error!("Panic while rendering component `{_name}`: {err:?}");
                    }
                    Element::Err(CapturedPanic(err).into())
                }
            }
        }

        let props = self.props.borrow().clone();
        render_inner(
            self.name,
            std::panic::catch_unwind(AssertUnwindSafe(move || self.render_fn.rebuild(props))),
        )
    }
}
