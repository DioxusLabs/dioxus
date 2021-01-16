///
///
///
///
///
// use crate::prelude::*;
use dioxus_core::prelude::*;
type VirtualNode = VNode;

/// Test a basic usage of a virtual dom + text renderer combo
#[test]
fn simple_integration() {
    let dom = VirtualDom::new(|_| html! { <div>Hello World!</div> });
    // let mut renderer = TextRenderer::new(dom);
    // let output = renderer.render();
}
