//! Debug virtual doms!
//! This renderer comes built in with dioxus core and shows how to implement a basic renderer.
//!
//! Renderers don't actually need to own the virtual dom (it's up to the implementer).

use crate::innerlude::RealDom;
use crate::{events::EventTrigger, virtual_dom::VirtualDom};
use crate::{innerlude::Result, prelude::*};

pub struct DebugRenderer {
    internal_dom: VirtualDom,
}

impl DebugRenderer {
    /// Create a new instance of the Dioxus Virtual Dom with no properties for the root component.
    ///
    /// This means that the root component must either consumes its own context, or statics are used to generate the page.
    /// The root component can access things like routing in its context.
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }

    /// Create a new text-renderer instance from a functional component root.
    /// Automatically progresses the creation of the VNode tree to completion.
    ///
    /// A VDom is automatically created. If you want more granular control of the VDom, use `from_vdom`
    pub fn new_with_props<T: Properties + 'static>(root: FC<T>, root_props: T) -> Self {
        Self::from_vdom(VirtualDom::new_with_props(root, root_props))
    }

    /// Create a new text renderer from an existing Virtual DOM.
    pub fn from_vdom(dom: VirtualDom) -> Self {
        // todo: initialize the event registry properly
        Self { internal_dom: dom }
    }

    pub fn handle_event(&mut self, trigger: EventTrigger) -> Result<()> {
        Ok(())
    }

    // pub fn step<Dom: RealDom>(&mut self, machine: &mut DiffMachine<Dom>) -> Result<()> {
    //     Ok(())
    // }

    // this does a "holy" compare - if something is missing in the rhs, it doesn't complain.
    // it only complains if something shows up that's not in the lhs, *or* if a value is different.
    // This lets you exclude various fields if you just want to drill in to a specific prop
    // It leverages the internal diffing mechanism.
    // If you have a list or "nth" child, you do need to list those children, but you don't need to
    // fill in their children/attrs/etc
    // Does not handle children or lifecycles and will always fail the test if they show up in the rhs
    pub fn compare<'a, F>(&self, other: LazyNodes<'a, F>) -> Result<()>
    where
        F: for<'b> FnOnce(&'b NodeFactory<'a>) -> VNode<'a> + 'a,
    {
        Ok(())
    }

    // Do a full compare - everything must match
    // Ignores listeners and children components
    pub fn compare_full<'a, F>(&self, other: LazyNodes<'a, F>) -> Result<()>
    where
        F: for<'b> FnOnce(&'b NodeFactory<'a>) -> VNode<'a> + 'a,
    {
        Ok(())
    }

    pub fn trigger_listener(&mut self, id: usize) -> Result<()> {
        Ok(())
    }

    pub fn render_nodes<'a, F>(&self, other: LazyNodes<'a, F>) -> Result<()>
    where
        F: for<'b> FnOnce(&'b NodeFactory<'a>) -> VNode<'a> + 'a,
    {
        Ok(())
    }
}

pub struct DebugVNodeSource {
    bump: Bump,
}
impl DebugVNodeSource {
    fn new() -> Self {
        Self { bump: Bump::new() }
    }

    fn render_nodes(&self) -> VNode {
        // let cx = NodeFactory
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_creation() -> Result<(), ()> {
        // static Example: FC<()> = |cx| {
        //     //
        //     cx.render(html! { <div> "hello world" </div> })
        // };

        // let mut dom = VirtualDom::new(Example);
        // let machine = DiffMachine::new();

        Ok(())
    }
}
