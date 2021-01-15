use html_macro::html;
use virtual_node::{IterableNodes, VElement, VirtualNode};

/// A re-export of everything to get macros working smoothly
pub mod prelude {
    pub use crate::component::Context;
    pub use crate::renderer::TextRenderer;
    pub use crate::types::FC;
    pub use crate::virtual_dom::VirtualDom;
    pub use html_macro::html;
    pub use virtual_node::{IterableNodes, VirtualNode};
}

pub mod virtual_dom {
    use super::*;

    pub struct VirtualDom {}

    impl VirtualDom {
        pub fn new(root: types::FC) -> Self {
            Self {}
        }
    }
}

/// Virtual Node Support
pub mod nodes {
    pub type VNode = virtual_node::VirtualNode;
    // pub enum VNode {
    //     VText,
    //     VElement,
    //     VComponent,
    // }
}

/// Example on how to craft a renderer that interacts with the VirtualDom
pub mod renderer {
    use crate::virtual_dom::VirtualDom;

    use super::*;

    /// Renders a full Dioxus app to a String
    ///
    pub struct TextRenderer {}

    impl TextRenderer {
        /// Create a new Text Renderer which renders the VirtualDom to a string
        pub fn new(dom: VirtualDom) -> Self {
            Self {}
        }

        pub fn render(&mut self) -> String {
            todo!()
        }
    }
}

pub mod component {

    /// A wrapper around component contexts that hides component property types
    pub struct AnyContext {}
    pub struct Context<T> {
        _props: std::marker::PhantomData<T>,
    }

    pub trait Properties {}
    impl Properties for () {}

    fn test() {}
}

/// Utility types that wrap internals
pub mod types {
    use super::*;
    use component::{AnyContext, Context};
    use nodes::VNode;

    pub type FC = fn(&mut AnyContext) -> VNode;
}

// #[cg(test)]
mod integration_tests {
    use crate::prelude::*;

    /// Test a basic usage of a virtual dom + text renderer combo
    #[test]
    fn simple_integration() {
        let dom = VirtualDom::new(|_| html! { <div>Hello World!</div> });
        let mut renderer = TextRenderer::new(dom);
        let output = renderer.render();
    }
}
