
pub(crate) fn create_suspense_boundary(
    &mut self,
    mount: MountId,
    idx: usize,
    component: &VComponent,
    parent: Option<ElementRef>,
) -> usize {
    todo!()
    //     let mut scope_id = ScopeId(self.get_mounted_dyn_node(mount, idx));
    //     // If the ScopeId is a placeholder, we need to load up a new scope for this vcomponent. If it's already mounted, then we can just use that
    //     if scope_id.is_placeholder() {
    //         {
    //             let suspense_context = SuspenseContext::new();

    //             let suspense_boundary_location =
    //                 crate::scope_context::SuspenseLocation::SuspenseBoundary(
    //                     suspense_context.clone(),
    //                 );
    //             self.runtime
    //                 .clone()
    //                 .with_suspense_location(suspense_boundary_location, || {
    //                     let scope_state = self
    //                         .dom
    //                         .new_scope(component.props.duplicate(), component.name)
    //                         .state();
    //                     suspense_context.mount(scope_state.id);
    //                     scope_id = scope_state.id;
    //                 });
    //         }

    //         // Store the scope id for the next render
    //         self.set_mounted_dyn_node(mount, idx, scope_id.0);
    //     }
    //     self.runtime.clone().with_scope_on_stack(scope_id, || {
    //         let scope_state = &mut self.dom.scopes[scope_id.0];
    //         let props =
    //             SuspenseBoundaryProps::downcast_from_props(&mut *scope_state.props).unwrap();
    //         let suspense_context =
    //             SuspenseContext::downcast_suspense_boundary_from_scope(self.runtime, scope_id)
    //                 .unwrap();

    //         let children = props.children.clone();

    //         // First always render the children in the background. Rendering the children may cause this boundary to suspend
    //         suspense_context.under_suspense_boundary(self.runtime, || {
    //             let write = self.write;
    //             self.write = false;
    //             self.create(children.as_vnode(), parent);
    //             self.write = write;
    //         });

    //         // Store the (now mounted) children back into the scope state
    //         let scope_state = &mut self.dom.scopes[scope_id.0];
    //         let props =
    //             SuspenseBoundaryProps::downcast_from_props(&mut *scope_state.props).unwrap();
    //         props.children.clone_from(&children);

    //         let scope_state = &mut self.dom.scopes[scope_id.0];
    //         let suspense_context = scope_state
    //             .state()
    //             .suspense_location()
    //             .suspense_context()
    //             .unwrap()
    //             .clone();
    //         // If there are suspended futures, render the fallback
    //         let nodes_created = if !suspense_context.suspended_futures().is_empty() {
    //             let (node, nodes_created) =
    //                 suspense_context.in_suspense_placeholder(self.runtime, || {
    //                     let scope_state = &mut self.dom.scopes[scope_id.0];
    //                     let props =
    //                         SuspenseBoundaryProps::downcast_from_props(&mut *scope_state.props)
    //                             .unwrap();
    //                     let suspense_context =
    //                         SuspenseContext::downcast_suspense_boundary_from_scope(
    //                             self.runtime,
    //                             scope_id,
    //                         )
    //                         .unwrap();
    //                     suspense_context.set_suspended_nodes(children.into());
    //                     let suspense_placeholder = props.fallback.call(suspense_context);
    //                     self.write = false;
    //                     let nodes_created = self.create(suspense_placeholder.as_vnode(), parent);
    //                     self.write = true;
    //                     (suspense_placeholder, nodes_created)
    //                 });

    //             let scope_state = &mut self.dom.scopes[scope_id.0];
    //             scope_state.last_rendered_node = Some(node);

    //             nodes_created
    //         } else {
    //             // Otherwise just render the children in the real dom
    //             debug_assert!(children.as_vnode().mount.get().mounted());
    //             let nodes_created = suspense_context.under_suspense_boundary(self.runtime, || {
    //                 self.create(children.as_vnode(), parent)
    //             });
    //             let scope_state = &mut self.dom.scopes[scope_id.0];
    //             scope_state.last_rendered_node = Some(children);
    //             let suspense_context =
    //                 SuspenseContext::downcast_suspense_boundary_from_scope(self.runtime, scope_id)
    //                     .unwrap();
    //             suspense_context.take_suspended_nodes();
    //             self.mark_suspense_resolved(&suspense_context, scope_id);

    //             nodes_created
    //         };
    //         nodes_created
    //     })
}

pub(crate) fn diff_suspense(&mut self, scope_id: ScopeId) {
    //     self.runtime.clone().with_scope_on_stack(scope_id, || {
    //         let scope = &mut self.dom.scopes[scope_id.0];
    //         let myself = SuspenseBoundaryProps::downcast_from_props(&mut *scope.props)
    //             .unwrap()
    //             .clone();

    //         let last_rendered_node = scope.last_rendered_node.as_ref().unwrap().clone();

    //         let SuspenseBoundaryProps {
    //             fallback, children, ..
    //         } = myself;

    //         let suspense_context = scope.state().suspense_boundary().unwrap().clone();
    //         let suspended_nodes = suspense_context.suspended_nodes();
    //         let suspended = !suspense_context.suspended_futures().is_empty();
    //         match (suspended_nodes, suspended) {
    //             // We already have suspended nodes that still need to be suspended
    //             // Just diff the normal and suspended nodes
    //             (Some(suspended_nodes), true) => {
    //                 let new_suspended_nodes: VNode = children.into();

    //                 // Diff the placeholder nodes in the dom
    //                 let new_placeholder =
    //                     suspense_context.in_suspense_placeholder(self.runtime, || {
    //                         let old_placeholder = last_rendered_node;
    //                         let new_placeholder = fallback.call(suspense_context.clone());

    //                         self.write = true;
    //                         self.diff_node(old_placeholder.as_vnode(), new_placeholder.as_vnode());
    //                         self.write = false;

    //                         new_placeholder
    //                     });

    //                 // Set the last rendered node to the placeholder
    //                 self.dom.scopes[scope_id.0].last_rendered_node = Some(new_placeholder);

    //                 // Diff the suspended nodes in the background
    //                 suspense_context.under_suspense_boundary(self.runtime, || {
    //                     self.write = false;
    //                     self.diff_node(&suspended_nodes, &new_suspended_nodes);
    //                     self.write = false;
    //                 });

    //                 let suspense_context = SuspenseContext::downcast_suspense_boundary_from_scope(
    //                     self.runtime,
    //                     scope_id,
    //                 )
    //                 .unwrap();
    //                 suspense_context.set_suspended_nodes(new_suspended_nodes);
    //             }

    //             // We have no suspended nodes, and we are not suspended. Just diff the children like normal
    //             (None, false) => {
    //                 let old_children = last_rendered_node;
    //                 let new_children = children;

    //                 suspense_context.under_suspense_boundary(self.runtime, || {
    //                     self.write = true;
    //                     self.diff_node(old_children.as_vnode(), new_children.as_vnode());
    //                     self.write = false;
    //                 });

    //                 // Set the last rendered node to the new children
    //                 self.dom.scopes[scope_id.0].last_rendered_node = Some(new_children);
    //             }

    //             // We have no suspended nodes, but we just became suspended. Move the children to the background
    //             (None, true) => {
    //                 let old_children = last_rendered_node.as_vnode();
    //                 let new_children: VNode = children.into();

    //                 let new_placeholder = fallback.call(suspense_context.clone());

    //                 // Move the children to the background
    //                 let parent = self.get_mounted_parent(old_children.mount.get());

    //                 suspense_context.in_suspense_placeholder(self.runtime, || {
    //                     self.write = false;
    //                     self.move_node_to_background(
    //                         old_children,
    //                         std::slice::from_ref(new_placeholder.as_vnode()),
    //                         parent,
    //                     );
    //                     self.write = true;
    //                 });

    //                 // Then diff the new children in the background
    //                 suspense_context.under_suspense_boundary(self.runtime, || {
    //                     self.write = false;
    //                     self.diff_node(old_children, &new_children);
    //                     self.write = true;
    //                 });

    //                 // Set the last rendered node to the new suspense placeholder
    //                 self.dom.scopes[scope_id.0].last_rendered_node = Some(new_placeholder);

    //                 let suspense_context = SuspenseContext::downcast_suspense_boundary_from_scope(
    //                     self.runtime,
    //                     scope_id,
    //                 )
    //                 .unwrap();
    //                 suspense_context.set_suspended_nodes(new_children);

    //                 // Move from a resolved suspense state to an suspended state
    //                 self.dom.resolved_scopes.retain(|&id| id != scope_id);
    //             }

    //             // We have suspended nodes, but we just got out of suspense. Move the suspended nodes to the foreground
    //             (Some(_), false) => {
    //                 // Take the suspended nodes out of the suspense boundary so the children know that the boundary is not suspended while diffing
    //                 let old_suspended_nodes = suspense_context.take_suspended_nodes().unwrap();
    //                 let old_placeholder = last_rendered_node;
    //                 let new_children = children;

    //                 // First diff the two children nodes in the background
    //                 suspense_context.under_suspense_boundary(self.runtime, || {
    //                     self.write = false;
    //                     self.diff_node(&old_suspended_nodes, new_children.as_vnode());
    //                     self.write = true;

    //                     // Then replace the placeholder with the new children
    //                     let mount = old_placeholder.as_vnode().mount.get();
    //                     let parent = self.get_mounted_parent(mount);
    //                     self.replace(
    //                         old_placeholder.as_vnode(),
    //                         std::slice::from_ref(new_children.as_vnode()),
    //                         parent,
    //                     );
    //                 });

    //                 // Set the last rendered node to the new children
    //                 self.dom.scopes[scope_id.0].last_rendered_node = Some(new_children);
    //                 self.mark_suspense_resolved(&suspense_context, scope_id);
    //             }
    //         }
    //     })
}

// /// Move to a resolved suspense state
// pub(crate) fn mark_suspense_resolved(
//     &mut self,
//     suspense_context: &SuspenseContext,
//     scope_id: ScopeId,
// ) {
//     self.dom.resolved_scopes.push(scope_id);

//     // Run any closures that were waiting for the suspense to resolve
//     suspense_context.run_resolved_closures(self.runtime);
// }

// #[doc(hidden)]
// /// Manually rerun the children of this suspense boundary without diffing against the old nodes.
// ///
// /// This should only be called by dioxus-web after the suspense boundary has been streamed in from the server.
// pub fn resolve_suspense(
//     &mut self,
//     scope_id: ScopeId,
//     only_write_templates: impl FnOnce(&mut M),
//     replace_with: usize,
// ) {
//     self.runtime.clone().with_scope_on_stack(scope_id, || {
//         let _runtime = RuntimeGuard::new(self.runtime.clone());
//         let Some(scope_state) = self.dom.scopes.get_mut(scope_id.0) else {
//             return;
//         };

//         // Reset the suspense context
//         let suspense_context = scope_state
//             .state()
//             .suspense_location()
//             .suspense_context()
//             .unwrap()
//             .clone();
//         suspense_context.inner.suspended_tasks.borrow_mut().clear();

//         // Get the parent of the suspense boundary to later create children with the right parent
//         let currently_rendered = scope_state.last_rendered_node.as_ref().unwrap().clone();
//         let mount = currently_rendered.as_vnode().mount.get();
//         let parent = {
//             let mounts = self.dom.runtime.mounts.borrow();
//             mounts
//                 .get(mount.0)
//                 .expect("suspense placeholder is not mounted")
//                 .parent
//         };

//         let props =
//             SuspenseBoundaryProps::downcast_from_props(&mut *scope_state.props).unwrap();

//         // Unmount any children to reset any scopes under this suspense boundary
//         let children = props.children.clone();
//         let suspense_context =
//             SuspenseContext::downcast_suspense_boundary_from_scope(self.runtime, scope_id)
//                 .unwrap();

//         // Take the suspended nodes out of the suspense boundary so the children know that the boundary is not suspended while diffing
//         let suspended = suspense_context.take_suspended_nodes();
//         if let Some(node) = suspended {
//             self.write = false;
//             self.remove_node(&node, None);
//             self.write = true;
//         }

//         // Replace the rendered nodes with resolved nodes
//         self.write = true;
//         self.remove_node(currently_rendered.as_vnode(), Some(replace_with));
//         self.write = false;

//         // Switch to only writing templates
//         only_write_templates(self.to);

//         children.as_vnode().mount.take();

//         // First always render the children in the background. Rendering the children may cause this boundary to suspend
//         suspense_context.under_suspense_boundary(self.runtime, || {
//             self.write = true;
//             self.create(children.as_vnode(), parent);
//             self.write = false;
//         });

//         // Store the (now mounted) children back into the scope state
//         let scope_state = &mut self.dom.scopes[scope_id.0];
//         let props =
//             SuspenseBoundaryProps::downcast_from_props(&mut *scope_state.props).unwrap();
//         props.children.clone_from(&children);
//         scope_state.last_rendered_node = Some(children);

//         // Run any closures that were waiting for the suspense to resolve
//         suspense_context.run_resolved_closures(self.runtime);
//     })
// }
