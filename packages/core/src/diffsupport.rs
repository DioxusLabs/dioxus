/// Destroy a scope and all of its descendents.
///
/// Calling this will run the destuctors on all hooks in the tree.
/// It will also add the destroyed nodes to the `seen_nodes` cache to prevent them from being renderered.
fn destroy_scopes(&mut self, old_scope: ScopeId) {
    let mut nodes_to_delete = vec![old_scope];
    let mut scopes_to_explore = vec![old_scope];

    // explore the scope tree breadth first
    while let Some(scope_id) = scopes_to_explore.pop() {
        // If we're planning on deleting this node, then we don't need to both rendering it
        self.seen_scopes.insert(scope_id);
        let scope = self.get_scope(&scope_id).unwrap();
        for child in scope.descendents.borrow().iter() {
            // Add this node to be explored
            scopes_to_explore.push(child.clone());

            // Also add it for deletion
            nodes_to_delete.push(child.clone());
        }
    }

    // Delete all scopes that we found as part of this subtree
    for node in nodes_to_delete {
        log::debug!("Removing scope {:#?}", node);
        let _scope = self.vdom.try_remove(node).unwrap();
        // do anything we need to do to delete the scope
        // I think we need to run the destructors on the hooks
        // TODO
    }
}

pub(crate) fn get_scope_mut(&mut self, id: &ScopeId) -> Option<&'bump mut Scope> {
    // ensure we haven't seen this scope before
    // if we have, then we're trying to alias it, which is not allowed
    debug_assert!(!self.seen_scopes.contains(id));

    unsafe { self.vdom.get_scope_mut(*id) }
}
pub(crate) fn get_scope(&mut self, id: &ScopeId) -> Option<&'bump Scope> {
    // ensure we haven't seen this scope before
    // if we have, then we're trying to alias it, which is not allowed
    unsafe { self.vdom.get_scope(*id) }
}

fn compare_strs(a: &str, b: &str) -> bool {
    // Check by pointer, optimizing for static strs
    if !std::ptr::eq(a, b) {
        // If the pointers are different then check by value
        a == b
    } else {
        true
    }
}

fn find_first_real_node<'a>(
    nodes: impl IntoIterator<Item = &'a VNode<'a>>,
    scopes: &'a SharedResources,
) -> Option<&'a VNode<'a>> {
    for node in nodes {
        let mut iter = RealChildIterator::new(node, scopes);
        if let Some(node) = iter.next() {
            return Some(node);
        }
    }

    None
}

fn remove_children(&mut self, old: &'bump [VNode<'bump>]) {
    self.replace_and_create_many_with_many(old, None)
}
