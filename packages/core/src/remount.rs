use crate::{
    RenderTargetId, ScopeId, VirtualDom,
    arena::{ElementId, RenderTargetState},
    innerlude::{MountId, MultiWriter},
    mount::MountedParent,
    mutations::TargetRouter,
    portal::is_portal_driver,
    runtime::RuntimeGuard,
    scopes::MountedOutput,
};

/// A scope whose rendered output sits at the root element of a render target.
struct TargetRoot {
    scope: ScopeId,
    logical_parent: Option<MountId>,
    render_parent: Option<MountedParent>,
}

/// Every writer of the host except the one for `skipped`.
struct WithoutTarget<'a, M> {
    to: &'a mut M,
    skipped: RenderTargetId,
}

impl<M: MultiWriter> MultiWriter for WithoutTarget<'_, M> {
    type Writer = M::Writer;

    fn writer_for(&mut self, id: RenderTargetId) -> Option<&mut M::Writer> {
        if id == self.skipped {
            None
        } else {
            self.to.writer_for(id)
        }
    }
}

impl VirtualDom {
    /// Write the whole mounted tree of `target` into `to`, between render passes, for a renderer
    /// that lost the target's nodes, such as a webview that reloaded its page.
    ///
    /// Components keep their state and do not run. The [`ElementId`]s of `target` are reassigned,
    /// and portals inside it rewrite their own targets through `to`.
    pub fn remount_render_target(&mut self, target: RenderTargetId, to: &mut impl MultiWriter) {
        let _runtime = RuntimeGuard::new(self.runtime.clone());
        let roots = self.target_roots(target);
        if roots.is_empty() {
            return;
        }

        {
            // The renderer of `target` has no nodes to remove, the other renderers do.
            let mut others = WithoutTarget {
                to: &mut *to,
                skipped: target,
            };
            let mut router = TargetRouter::new(&mut others, self.runtime.clone());
            for root in &roots {
                self.remove_component_node(Some(&mut router), false, root.scope);
            }
        }
        self.reset_render_target(target);

        let mut router = TargetRouter::new(to, self.runtime.clone());
        for root in roots {
            if root.scope == ScopeId::ROOT {
                self.rebuild_with_writer(&mut router);
                continue;
            }
            let driver = self.runtime.get_state(root.scope).render_driver();
            self.runtime.clone().while_rendering(|| {
                driver.create(self, root.scope, root.logical_parent, Some(&mut router))
            });
            if let Some(render_parent) = root.render_parent {
                let root_mount = self.scopes[root.scope.index()]
                    .last_rendered_node
                    .as_ref()
                    .map(MountedOutput::root_mount)
                    .expect("a remounted scope has rendered output");
                self.set_mounted_render_parent(root_mount, render_parent);
            }
        }
    }

    /// The outermost rendered scopes that place their output at the root element of `target`, in
    /// scope id order. Their subtrees hold every node of `target`.
    fn target_roots(&self, target: RenderTargetId) -> Vec<TargetRoot> {
        let places_at_target_root = |scope: ScopeId| {
            self.runtime.try_get_state(scope).is_some_and(|state| {
                state.target_id() == target
                    && (scope == ScopeId::ROOT || is_portal_driver(&*state.render_driver()))
            })
        };
        let parent = |scope: &ScopeId| self.runtime.get_state(*scope).parent_id();

        self.scopes
            .iter()
            .map(|(index, _)| ScopeId::new(index))
            .filter(|&scope| places_at_target_root(scope))
            .filter(|scope| {
                std::iter::successors(parent(scope), parent).all(|id| !places_at_target_root(id))
            })
            .filter(|&scope| self.runtime.scope_should_render(scope))
            .filter_map(|scope| {
                let root_mount = self.scopes[scope.index()]
                    .last_rendered_node
                    .as_ref()?
                    .root_mount();
                Some(TargetRoot {
                    scope,
                    logical_parent: self.mounted_logical_parent(root_mount),
                    render_parent: self.mounted_render_parent(root_mount),
                })
            })
            .collect()
    }

    /// Forget the renderer-local state of `target` once no mount holds one of its elements.
    fn reset_render_target(&mut self, target: RenderTargetId) {
        let mut targets = self.runtime.render_targets.borrow_mut();
        let state = targets
            .get_mut(target.index())
            .expect("a remounted target is registered");
        debug_assert!(
            state
                .elements
                .iter()
                .all(|(index, _)| index == ElementId::ROOT.index()
                    || state.template_roots.values().any(|id| id.index() == index)),
            "every mounted element of the target is reclaimed before it is reset"
        );
        *state = RenderTargetState::new();
    }
}
