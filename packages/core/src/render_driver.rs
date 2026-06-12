use std::{any::Any, cell::RefCell, panic::AssertUnwindSafe, rc::Rc};

use crate::{
    ComponentFunction, Element, WriteMutations,
    diff::context::DiffContext,
    innerlude::{CapturedPanic, ElementRef},
    scopes::{LastRenderedNode, ScopeId},
    virtual_dom::VirtualDom,
};

/// A scope's rendering lifecycle and the inputs it renders from.
///
/// Every scope owns exactly one driver, attached when its [`VComponent`] is
/// constructed and fixed for the scope's lifetime: plain components use
/// [`BodyDriver`], which owns the component function and its props and
/// mounts/diffs the element the body returns, while portal and suspense
/// components attach drivers in their `into_vcomponent` that own their props
/// and manage the scope's `last_rendered_node` directly, with no body to run.
///
/// A driver instance is per component instance: the scope adopts a
/// [`Self::duplicate`] of the vnode's driver at creation so the live scope
/// never aliases inputs with a vnode, and [`Self::memoize`] is how a parent
/// render hands the scope its new inputs.
///
/// Dispatch invokes drivers with no scope pushed on the runtime stack for the
/// call; drivers push their own scope/suspense frames.
pub(crate) trait RenderDriver: 'static {
    /// The driver as `Any`, for [`Self::memoize`] hand-offs between two
    /// instances of the same driver type.
    fn as_any(&self) -> &dyn Any;

    /// Whether `other` renders the same component as this driver, i.e. a
    /// scope rendered by this driver can be diffed in place against
    /// `other`'s props rather than replaced. Two drivers of one type
    /// identify the same component by default; [`BodyDriver`] also compares
    /// its function value, since dynamic components can put different
    /// functions of one type in a slot.
    fn same_component(&self, other: &dyn RenderDriver) -> bool {
        self.as_any().type_id() == other.as_any().type_id()
    }

    /// Make this driver's props equal to `new_driver`'s (a driver of the
    /// same concrete type, guaranteed by the [`Self::same_component`]
    /// check). Returns whether the props were equal and the scope can be
    /// memoized.
    fn memoize(&self, new_driver: &dyn Any) -> bool;

    /// A fresh driver instance with cloned props, for [`VComponent`] clones
    /// and scope adoption.
    ///
    /// [`VComponent`]: crate::nodes::VComponent
    fn duplicate(&self) -> Rc<dyn RenderDriver>;

    /// Mount this scope's output. `new` is true when the scope was allocated
    /// for this create and has never run or rendered. Maintains
    /// `scopes[id].last_rendered_node`. Returns the number of nodes left on
    /// the renderer stack.
    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        new: bool,
        parent: Option<ElementRef>,
        to: Option<&mut dyn WriteMutations>,
    ) -> usize;

    /// Diff this scope's output against its current props. `to` is the
    /// ungated writer; the driver applies its own write gating.
    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        parent_context: Option<DiffContext<'_>>,
        to: Option<&mut dyn WriteMutations>,
    );

    /// Remove this scope's output. When `destroy_component_state` is false
    /// the output is only being lifted out of the real DOM and the driver
    /// must keep component state alive.
    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut dyn WriteMutations>,
        destroy_component_state: bool,
    );
}

/// Remove a scope's rendered output from the DOM, and drop the scope when
/// `destroy_component_state` is set. Shared by [`BodyDriver`] and drivers
/// whose output is removed the same way (suspense).
pub(crate) fn remove_rendered_output(
    dom: &mut VirtualDom,
    scope_id: ScopeId,
    mut to: Option<&mut dyn WriteMutations>,
    destroy_component_state: bool,
) {
    // Removal only fires after the scope has rendered at least once via
    // `create`, which sets `last_rendered_node` to `Some`. A scope that
    // never rendered is dropped without going through this path.
    let node = dom.scopes[scope_id.index()]
        .last_rendered_node
        .clone()
        .expect("scope being removed should have last_rendered_node set");
    node.remove_node_inner(dom, to.as_mut(), destroy_component_state);

    if destroy_component_state {
        // Now drop all the resources
        dom.drop_scope(scope_id);
    }
}

/// The rendering lifecycle of a plain component: the driver owns the
/// component function and its props, runs the body, and the element it
/// returns is the scope's rendered output.
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

    /// Run the component body with the current props, converting panics into
    /// an error element.
    fn render(&self) -> Element {
        fn render_inner(_name: &str, res: Result<Element, Box<dyn Any + Send>>) -> Element {
            match res {
                Ok(node) => node,
                Err(err) => {
                    // on wasm this massively bloats binary sizes and we can't even capture the panic
                    // so do nothing
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
        // A new scope runs once to get the initial state
        if new {
            let body = dom.run_scope_with(scope_id, || self.render());
            dom.scopes[scope_id.index()].last_rendered_node = Some(LastRenderedNode::new(body));
        }

        // If our scope landed in `dirty_scopes` during its initial render
        // (e.g. a hook synchronously queued an update for itself), drain the
        // entry now so we don't re-process the same scope after creation.
        dom.mark_clean(scope_id);

        let new_node = dom.scopes[scope_id.index()]
            .last_rendered_node
            .clone()
            .expect("Component to be mounted");

        dom.create_scope(to.as_mut(), scope_id, new_node, parent)
    }

    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        parent_context: Option<DiffContext<'_>>,
        mut to: Option<&mut dyn WriteMutations>,
    ) {
        let body = dom.run_scope_with(scope_id, || self.render());
        dom.diff_scope(to.as_mut(), scope_id, body, parent_context);
    }

    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut dyn WriteMutations>,
        destroy_component_state: bool,
    ) {
        remove_rendered_output(dom, scope_id, to, destroy_component_state);
    }
}
