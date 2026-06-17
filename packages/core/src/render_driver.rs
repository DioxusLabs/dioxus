use std::{any::Any, cell::RefCell, panic::AssertUnwindSafe, rc::Rc};

use crate::{
    ComponentFunction, Element, WriteMutations,
    innerlude::{CapturedPanic, ElementRef, ScopeOrder, SuspenseContext},
    scopes::{LastRenderedNode, ScopeId},
    virtual_dom::VirtualDom,
};

/// The rendering lifecycle driver for a scope.
///
/// Every scope owns exactly one driver via `Rc<dyn RenderDriver>`:
/// - Plain components use [`BodyDriver`], which owns the component function and props.
/// - Suspense boundaries use [`SuspenseDriver`](crate::suspense::SuspenseDriver),
///   which owns the [`SuspenseContext`] and manages children/fallback rendering.
pub(crate) trait RenderDriver: 'static {
    fn as_any(&self) -> &dyn Any;

    /// Whether `other` renders the same component as this driver.
    fn same_component(&self, other: &dyn RenderDriver) -> bool {
        self.as_any().type_id() == other.as_any().type_id()
    }

    /// Update this driver's props to match `new_driver`'s. Returns `true` if
    /// the props were equal (memoized).
    fn memoize(&self, new_driver: &dyn Any) -> bool;

    /// A fresh instance with cloned props.
    fn duplicate(&self) -> Rc<dyn RenderDriver>;

    /// Mount this scope's output. `to` receives DOM mutations; pass `None` for
    /// background rendering (e.g. suspended children).
    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        new: bool,
        parent: Option<ElementRef>,
        to: Option<&mut dyn WriteMutations>,
    ) -> usize;

    /// Diff this scope's output against its current props.
    fn diff(&self, dom: &mut VirtualDom, scope_id: ScopeId, to: Option<&mut dyn WriteMutations>);

    /// Remove this scope's output.
    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut dyn WriteMutations>,
        destroy_component_state: bool,
        replace_with: Option<usize>,
    );

    /// If this driver is a suspense boundary, return its context.
    fn suspense_context(&self) -> Option<SuspenseContext> {
        None
    }
}

/// Remove a scope's rendered output from the DOM, and drop the scope when
/// `destroy_component_state` is set. Shared by body and suspense drivers.
pub(crate) fn remove_rendered_output(
    dom: &mut VirtualDom,
    scope_id: ScopeId,
    mut to: Option<&mut dyn WriteMutations>,
    destroy_component_state: bool,
    replace_with: Option<usize>,
) {
    if let Some(node) = dom.scopes[scope_id.0].last_rendered_node.clone() {
        node.remove_node_inner(dom, to.as_mut(), destroy_component_state, replace_with)
    };

    if destroy_component_state {
        dom.drop_scope(scope_id);
    }
}

/// The concrete driver for plain (non-suspense) components.
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

impl<F: ComponentFunction<P, M> + Clone, P: Clone + 'static, M: 'static> RenderDriver
    for BodyDriver<F, P, M>
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn same_component(&self, other: &dyn RenderDriver) -> bool {
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

    fn duplicate(&self) -> Rc<dyn RenderDriver> {
        Rc::new(Self {
            render_fn: self.render_fn.clone(),
            memo: self.memo,
            props: RefCell::new(self.props.borrow().clone()),
            name: self.name,
            phantom: std::marker::PhantomData,
        })
    }

    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        new: bool,
        parent: Option<ElementRef>,
        mut to: Option<&mut dyn WriteMutations>,
    ) -> usize {
        if new {
            let body = dom.run_scope_with(scope_id, || self.render());
            dom.scopes[scope_id.0].last_rendered_node = Some(LastRenderedNode::new(body));
            let height = dom.runtime.get_state(scope_id).height;
            dom.dirty_scopes.remove(&ScopeOrder::new(height, scope_id));
        }
        let new_node = dom.scopes[scope_id.0]
            .last_rendered_node
            .clone()
            .expect("Component to be mounted");
        dom.create_scope(to.as_mut(), scope_id, new_node, parent)
    }

    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        mut to: Option<&mut dyn WriteMutations>,
    ) {
        let body = dom.run_scope_with(scope_id, || self.render());
        dom.diff_scope(to.as_mut(), scope_id, body);
    }

    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut dyn WriteMutations>,
        destroy_component_state: bool,
        replace_with: Option<usize>,
    ) {
        remove_rendered_output(dom, scope_id, to, destroy_component_state, replace_with);
    }
}
