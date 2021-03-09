//! Debug virtual doms!
//! This renderer comes built in with dioxus core and shows how to implement a basic renderer.
//!
//! Renderers don't actually need to own the virtual dom (it's up to the implementer).

use crate::prelude::VirtualDom;

pub struct DebugRenderer {
    vdom: VirtualDom,
}

impl DebugRenderer {
    pub fn new(vdom: VirtualDom) -> Self {
        Self { vdom }
    }

    pub async fn run(&mut self) -> Result<(), ()> {
        Ok(())
    }

    pub fn log_dom(&self) {}
}

#[cfg(old)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::scope::Properties;

    #[test]
    fn ensure_creation() -> Result<(), ()> {
        #[derive(PartialEq)]
        struct Creation {}
        impl FC for Creation {
            fn render(ctx: Context, props: &Self) -> DomTree {
                ctx.render(html! { <div>"hello world" </div> })
            }
        }

        let mut dom = VirtualDom::new_with_props(Creation {});

        Ok(())
    }
}
