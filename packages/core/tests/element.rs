use dioxus::prelude::*;
use dioxus_core::SuspenseContext;

/// Ensure no issues with not building the virtualdom before
#[test]
fn root_node_isnt_null() {
    let dom = VirtualDom::new(|cx| render!("Hello world!"));

    let scope = dom.base_scope();

    // The root should be a valid pointer
    assert_ne!(scope.root_node() as *const _, std::ptr::null_mut());

    // The height should be 0
    assert_eq!(scope.height(), 0);

    // There should be a default suspense context
    // todo: there should also be a default error boundary
    assert!(scope.has_context::<SuspenseContext>().is_some());
}
