//! Tests related to safety of the library.

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core::SuspenseContext;

/// Ensure no issues with not calling rebuild
#[test]
fn root_node_isnt_null() {
    let dom = VirtualDom::new(|cx| render!("Hello world!"));

    let scope = dom.base_scope();

    // We haven't built the tree, so trying to get out the root node should fail
    assert!(scope.try_root_node().is_none());

    // The height should be 0
    assert_eq!(scope.height(), 0);

    // There should be a default suspense context
    // todo: there should also be a default error boundary
    assert!(scope.has_context::<Rc<SuspenseContext>>().is_some());
}
