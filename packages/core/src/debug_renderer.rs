//! Debug virtual doms!
//! This renderer comes built in with dioxus core and shows how to implement a basic renderer.
//!
//! Renderers don't actually need to own the virtual dom (it's up to the implementer).

use crate::prelude::{Properties, VirtualDom};

pub struct DebugRenderer<'a, P: Properties> {
    vdom: &'a mut VirtualDom<P>,
}

impl<'a, P: Properties> DebugRenderer<'a, P> {
    pub fn new(vdom: &'a mut VirtualDom<P>) -> Self {
        Self { vdom }
    }

    pub async fn run(&mut self) -> Result<(), ()> {
        Ok(())
    }

    pub fn log_dom(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn ensure_creation() -> Result<(), ()> {
        let mut dom = VirtualDom::new(|ctx, props| {
            //
            ctx.view(html! { <div>"hello world" </div> })
        });

        dom.progress()?;
        Ok(())
    }
}
