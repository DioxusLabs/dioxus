//! Tests related to safety of the library.

use dioxus::prelude::*;

/// Ensure no issues with not calling rebuild_to_vec
#[test]
fn root_node_isnt_null() {
    let dom = VirtualDom::new(|| rsx!("Hello world!"));

    let scope = dom.base_scope();

    // We haven't built the tree, so trying to get out the root node should fail
    assert!(scope.try_root_node().is_none());

    dom.in_runtime(|| {
        // The height should be 0
        assert_eq!(ScopeId::ROOT.height(), 0);
    });
}
