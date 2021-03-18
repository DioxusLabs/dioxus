//! Debug virtual doms!
//! This renderer comes built in with dioxus core and shows how to implement a basic renderer.
//!
//! Renderers don't actually need to own the virtual dom (it's up to the implementer).

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

    pub fn step(&mut self, machine: &mut DiffMachine) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_creation() -> Result<(), ()> {
        // static Example: FC<()> = |ctx, props| {
        //     //
        //     ctx.render(html! { <div> "hello world" </div> })
        // };

        // let mut dom = VirtualDom::new(Example);
        // let machine = DiffMachine::new();

        Ok(())
    }
}
