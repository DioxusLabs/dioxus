use std::{any::Any, cell::RefCell, panic::AssertUnwindSafe, rc::Rc};

use crate::{
    AttributeValue, ComponentFunction, Element, Template, WriteMutations,
    arena::ElementId,
    innerlude::{CapturedPanic, ElementRef, ScopeOrder},
    scopes::{LastRenderedNode, ScopeId},
    virtual_dom::VirtualDom,
};

/// A sized wrapper around `&mut dyn WriteMutations` that itself implements
/// `WriteMutations`, letting the `dyn RenderDriver` layer bridge into the
/// generic (`M: WriteMutations + Sized`) diffing methods without requiring
/// `?Sized` bounds throughout the diff pipeline.
pub(crate) struct DynWriter<'a>(&'a mut dyn WriteMutations);

impl<'a> DynWriter<'a> {
    /// Erase a generic `Option<&mut M>` into `Option<&mut DynWriter>`.
    ///
    /// The returned option borrows the original writer through the `DynWriter`
    /// wrapper, so the caller must not use the original `to` while the wrapper
    /// is alive.
    #[inline]
    pub fn erase<M: WriteMutations>(to: Option<&mut M>) -> Option<DynWriter<'_>> {
        to.map(|m| DynWriter(m as &mut dyn WriteMutations))
    }
}

impl WriteMutations for DynWriter<'_> {
    fn append_children(&mut self, id: ElementId, m: usize) {
        self.0.append_children(id, m)
    }
    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        self.0.assign_node_id(path, id)
    }
    fn create_placeholder(&mut self, id: ElementId) {
        self.0.create_placeholder(id)
    }
    fn create_text_node(&mut self, value: &str, id: ElementId) {
        self.0.create_text_node(value, id)
    }
    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        self.0.load_template(template, index, id)
    }
    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        self.0.replace_node_with(id, m)
    }
    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        self.0.replace_placeholder_with_nodes(path, m)
    }
    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        self.0.insert_nodes_after(id, m)
    }
    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        self.0.insert_nodes_before(id, m)
    }
    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        self.0.set_attribute(name, ns, value, id)
    }
    fn set_node_text(&mut self, value: &str, id: ElementId) {
        self.0.set_node_text(value, id)
    }
    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.0.create_event_listener(name, id)
    }
    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.0.remove_event_listener(name, id)
    }
    fn remove_node(&mut self, id: ElementId) {
        self.0.remove_node(id)
    }
    fn push_root(&mut self, id: ElementId) {
        self.0.push_root(id)
    }
}

/// A scope's rendering lifecycle and the inputs it renders from.
///
/// Every scope owns exactly one driver, attached when its [`VComponent`] is
/// constructed and fixed for the scope's lifetime: plain components use
/// [`BodyDriver`], which owns the component function and its props and
/// mounts/diffs the element the body returns, while suspense components attach
/// drivers in their `into_vcomponent` that own their props and manage the
/// scope's `last_rendered_node` directly, with no body to run.
///
/// A driver instance is per component instance: the scope adopts a
/// [`Self::duplicate`] of the vnode's driver at creation so the live scope
/// never aliases inputs with a vnode, and [`Self::memoize`] is how a parent
/// render hands the scope its new inputs.
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
    /// for this create and has never run or rendered.
    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        new: bool,
        parent: Option<ElementRef>,
        to: Option<DynWriter<'_>>,
    ) -> usize;

    /// Diff this scope's output against its current props.
    fn diff(&self, dom: &mut VirtualDom, scope_id: ScopeId, to: Option<DynWriter<'_>>);

    /// Remove this scope's output. When `destroy_component_state` is false
    /// the output is only being lifted out of the real DOM and the driver
    /// must keep component state alive.
    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<DynWriter<'_>>,
        destroy_component_state: bool,
        replace_with: Option<usize>,
    );
}

/// Remove a scope's rendered output from the DOM, and drop the scope when
/// `destroy_component_state` is set. Shared by [`BodyDriver`] and drivers
/// whose output is removed the same way (suspense).
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
        mut to: Option<DynWriter<'_>>,
    ) -> usize {
        if new {
            let body = dom.run_scope_with(scope_id, || self.render());
            dom.scopes[scope_id.0].last_rendered_node = Some(LastRenderedNode::new(body));

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

        dom.create_scope(to.as_mut(), scope_id, new_node, parent)
    }

    fn diff(&self, dom: &mut VirtualDom, scope_id: ScopeId, mut to: Option<DynWriter<'_>>) {
        let body = dom.run_scope_with(scope_id, || self.render());
        dom.diff_scope(to.as_mut(), scope_id, body);
    }

    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        mut to: Option<DynWriter<'_>>,
        destroy_component_state: bool,
        replace_with: Option<usize>,
    ) {
        remove_rendered_output(
            dom,
            scope_id,
            to.as_mut(),
            destroy_component_state,
            replace_with,
        );
    }
}
