#![cfg(feature = "hot_reload")]

//! This module contains hotreloading logic for rsx.
//!
//! There's a few details that I wish we could've gotten right but we can revisit later:
//!
//! - Empty rsx! blocks are written as `None` - it would be nice to be able to hot reload them
//!
//! - The byte index of the template is not the same as the byte index of the original template
//!   this forces us to make up IDs on the fly. We should just find an ID naming scheme, but that
//!   struggles when you have nested rsx! calls since file:line:col is the same for all expanded rsx!
//!
//! - There's lots of linear scans
//!
//! - Expanding an if chain is not possible - only its contents can be hot reloaded
//!
//! - Components that don't start with children can't be hotreloaded - IE going from `Comp {}` to `Comp { "foo" }`
//!   is not possible. We could in theory allow this by seeding all Components with a `children` field.
//!
//! - Cross-templates hot reloading is not possible - multiple templates don't share the dynamic nodes.
//!   This would require changes in core to work, I imagine.
//!
//! Future work
//!
//! - We've proven that binary patching is feasible but has a longer path to stabilization for all platforms.
//!   Binary patching is pretty quick, actually, and *might* remove the need to literal hotreloading.
//!   However, you could imagine a scenario where literal hotreloading would be useful without the
//!   compiler in the loop. Ideally we can slash most of this code once patching is stable.
//!
//! - We could also allow adding arbitrary nodes/attributes at runtime. The template system doesn't
//!   quite support that, unfortunately, since the number of dynamic nodes and attributes is baked into
//!   the template, but if that changed we'd be okay.

use crate::{innerlude::*, scoring::score_dynamic_node};
use crate::{scoring::score_attribute, HotReloadingContext};
use dioxus_core::{internal::HotReloadLiteral, Template};
use std::collections::HashMap;

/// The mapping of a node relative to the root of its containing template
///
/// IE [0, 1] would be the location of the h3 node in this template:
/// ```rust, ignore
/// rsx! {
///     div {
///         h1 { "title" }
///         h3 { class: "{class}", "Hi" }
///     }
/// }
/// ```
type NodePath = Vec<u8>;

/// The mapping of an attribute relative to the root of its containing template
/// Order doesn't matter for attributes, you can render them in any order on a given node.a
///
/// IE [0, 1] would be the location of the `class` attribute on this template:
/// ```rust, ignore
/// rsx! {
///     div {
///         h1 { "title" }
///         h3 { class: "{class}", "Hi" }
///     }
/// }
/// ```
type AttributePath = Vec<u8>;

/// A result of hot reloading
///
/// This contains information about what has changed so the hotreloader can apply the right changes
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
#[derive(Debug, PartialEq, Clone)]
pub struct HotReloadedTemplate {
    /// List of inner templates that changed (nested blocks like for/if/component bodies)
    pub templates: Vec<Template>,

    /// Previously changed lits that we're going to use to invalidate the old literals
    pub prev_lits: HashMap<String, HotReloadLiteral>,

    /// A map of Signal IDs to the new literals
    /// Eventually we'll want to move this to a more efficient data structure to have one signal per rsx! call
    pub changed_lits: HashMap<String, HotReloadLiteral>,

    // The location of the original call
    // This should be in the form of `file:line:col:0` - 0 since this will be the base template
    pub location: &'static str,
}

impl HotReloadedTemplate {
    /// Calculate the hotreload diff between two callbodies
    pub fn new<Ctx: HotReloadingContext>(
        old: &CallBody,
        new: &CallBody,
        location: &'static str,
        old_lits: HashMap<String, HotReloadLiteral>,
    ) -> Option<Self> {
        let mut s = Self {
            templates: Default::default(),
            changed_lits: Default::default(),
            prev_lits: old_lits,
            location,
        };

        s.hotreload_body::<Ctx>(&old.body, &new.body)?;

        Some(s)
    }

    /// Walk the dynamic contexts and do our best to find hotreloadable changes between the two
    /// sets of dynamic nodes/attributes. If there's a change we can't hotreload, we'll return None
    ///
    /// Otherwise, we pump out the list of templates that need to be updated. The templates will be
    /// re-ordered such that the node paths will be adjusted to match the new template for every
    /// existing dynamic node.
    ///
    /// ```ignore
    /// old:
    ///     [[0], [1], [2]]
    ///     rsx! {
    ///         "{one}"
    ///         "{two}"
    ///         "{three}"
    ///     }
    ///
    /// new:
    ///     [[0], [2], [1, 1]]
    ///     rsx! {
    ///        "{one}"
    ///         div { "{three}" }
    ///         "{two}"
    ///    }
    /// ```
    ///
    /// Generally we can't hotreload a node if:
    /// - We add a truly dynaamic node (except maybe text nodes - but even then.. only if we've seen them before)
    ///
    /// If a dynamic node is removed, we don't necessarily need to kill hotreload - just unmounting it should be enough
    /// If the dynamic node is re-added, we want to be able to find it again.
    ///
    /// This encourages the hotreloader to hot onto DynamicContexts directly instead of the CallBody since
    /// you can preserve more information about the nodes as they've changed over time.
    pub fn hotreload_body<Ctx: HotReloadingContext>(
        &mut self,
        old: &TemplateBody,
        new: &TemplateBody,
    ) -> Option<()> {
        // Quickly run through dynamic attributes first attempting to invalidate them
        // Move over old IDs onto the new template
        let new_attribute_paths = self.hotreload_attributes(old, new)?;

        // Now we can run through the dynamic nodes and see if we can hot reload them
        // Move over old IDs onto the new template
        let new_node_paths = self.hotreload_dynamic_nodes::<Ctx>(old, new)?;

        // Now render the new template out. We've proven that it's a close enough match to the old template
        //
        // The paths will be different but the dynamic indexes will be the same
        let template = new.to_template_with_custom_paths::<Ctx>(
            intern(self.make_location(old.template_idx.get()).leak()),
            new_node_paths,
            new_attribute_paths,
        );

        self.templates.push(template);

        Some(())
    }

    /// Take two dynamic contexts and return a mapping of dynamic attributes from the original to the new.
    ///
    /// IE if we shuffle attributes around we should be able to hot reload them.
    /// Same thing with dropping dynamic attributes.
    ///
    /// Does not apply with moving the dynamic contents from one attribute to another.
    ///
    /// ```rust, ignore
    /// rsx! {
    ///     div { id: "{id}", class: "{class}", "Hi" }
    /// }
    ///
    /// rsx! {
    ///     div { class: "{class}", id: "{id}", "Hi" }
    /// }
    /// ```
    fn hotreload_attributes(
        &mut self,
        old: &TemplateBody,
        new: &TemplateBody,
    ) -> Option<Vec<AttributePath>> {
        // Build a stack of old attributes so we can pop them off as we find matches in the new attributes
        //
        // Note that we might have duplicate attributes! We use a stack just to make sure we don't lose them
        // Also note that we use a vec + remove, but the idea is that in most cases we're removing from the end
        // which is an O(1) operation. We could use a linked list or a queue, but I don't want any
        // more complexity than necessary here since this can complex.
        let mut old_attrs = PopVec::new(old.dynamic_attributes());

        // Now we can run through the dynamic nodes and see if we can hot reload them
        // Here we create the new attribute paths for the final template - we'll fill them in as we find matches
        let mut attr_paths = vec![vec![]; old.attr_paths.len()];

        // Note that we walk the new attributes - we can remove segments from formatted text so
        // all `new` is a subset of `old`.
        for new_attr in new.dynamic_attributes() {
            // We're going to score the attributes based on their names and values
            // This ensures that we can handle the majority of cases where the attributes are shuffled around
            // or their contents have been stripped down
            //
            // A higher score is better - 0 is a mismatch, usize::MAX is a perfect match
            // As we find matches, the complexity of the search should reduce, making this quadratic
            // a little less painful
            let (old_idx, score) =
                old_attrs.highest_score(move |old_attr| score_attribute(old_attr, new_attr))?;

            // Remove it from the stack so we don't match it again
            let old_attr = old_attrs.remove(old_idx).unwrap();

            // This old node will now need to take on the new path
            attr_paths[old_attr.dyn_idx.get()] = new.attr_paths[new_attr.dyn_idx.get()].clone().0;

            // Now move over the idx of the old to the new
            //
            // We're going to reuse the new CallBody to render the new template, so we have to make sure
            // stuff like IDs are ported over properly
            //
            // it's a little dumb to modify the new one in place, but it us avoid a lot of complexity
            // we should change the semantics of these methods to take the new one mutably, making it
            // clear that we're going to modify it in place and use it render
            new_attr.dyn_idx.set(old_attr.dyn_idx.get());

            // While we're here, if it's a literal and not a perfect score, it's a mismatch and we need to
            // hotreload the literal
            self.hotreload_attribute(old_attr, new_attr, score)?;
        }

        Some(attr_paths)
    }

    fn hotreload_dynamic_nodes<Ctx: HotReloadingContext>(
        &mut self,
        old: &TemplateBody,
        new: &TemplateBody,
    ) -> Option<Vec<NodePath>> {
        use BodyNode::*;
        let mut old_nodes = PopVec::new(old.dynamic_nodes());

        let mut node_paths = vec![vec![]; old.node_paths.len()];

        for new_node in new.dynamic_nodes() {
            // Find the best match for the new node - this is done by comparing the dynamic contents of the various nodes to
            // find the best fit.
            //
            // We do this since two components/textnodes/attributes *might* be similar in terms of dynamic contents
            // but not be the same node.
            let (old_idx, score) =
                old_nodes.highest_score(move |old_node| score_dynamic_node(old_node, new_node))?;

            // Remove it from the stack so we don't match it again - this is O(1)
            let old_node = old_nodes.remove(old_idx)?;

            // This old node will now need to take on the new path in the new template
            node_paths[old_node.get_dyn_idx()].clone_from(&new.node_paths[new_node.get_dyn_idx()]);

            // But we also need to make sure the new node is taking on the old node's ID
            new_node.set_dyn_idx(old_node.get_dyn_idx());

            // Make sure we descend into the children, and then record any changed literals
            match (old_node, new_node) {
                // If the contents of the text changed, then we need to hotreload the text node
                (Text(a), Text(b)) if score != usize::MAX => {
                    self.hotreload_text_node(a, b)?;
                }

                // We want to attempt to  hotreload the component literals and the children
                (Component(a), Component(b)) => {
                    self.hotreload_component_fields(a, b)?;
                    self.hotreload_body::<Ctx>(&a.children, &b.children)?;
                }

                // We don't reload the exprs or condition - just the bodies
                (ForLoop(a), ForLoop(b)) => {
                    self.hotreload_body::<Ctx>(&a.body, &b.body)?;
                }

                // Ensure the if chains are the same and then hotreload the bodies
                // We don't handle new chains or "elses" just yet - but feasibly we could allow
                // for an `else` chain to be added/removed.
                //
                // Our ifchain parser would need to be better to support this.
                (IfChain(a), IfChain(b)) => {
                    self.hotreload_ifchain::<Ctx>(a, b)?;
                }

                // Just assert we never get these cases - attributes are handled separately
                (Element(_), Element(_)) => unreachable!("Elements are not dynamic nodes"),

                _ => {}
            }
        }

        Some(node_paths)
    }

    fn hotreload_text_node(&mut self, a: &TextNode, b: &TextNode) -> Option<()> {
        let idx = a.hr_idx.get();
        let location = self.make_location(idx);
        let segments = IfmtInput::fmt_segments(&a.input, &b.input)?;
        self.changed_lits
            .insert(location.to_string(), HotReloadLiteral::Fmted(segments));

        Some(())
    }

    fn hotreload_component_fields(&mut self, a: &Component, b: &Component) -> Option<()> {
        // make sure both are the same length
        if a.fields.len() != b.fields.len() {
            return None;
        }

        let mut left_fields = a.fields.iter().collect::<Vec<_>>();
        left_fields.sort_by(|a, b| a.name.to_string().cmp(&b.name.to_string()));

        let mut right_fields = b.fields.iter().collect::<Vec<_>>();
        right_fields.sort_by(|a, b| a.name.to_string().cmp(&b.name.to_string()));

        // Walk the attributes looking for literals
        // Those will have plumbing in the hotreloading code
        // All others just get diffed via tokensa
        for (old_attr, new_attr) in left_fields.iter().zip(right_fields.iter()) {
            self.hotreload_attribute(old_attr, new_attr, score_attribute(old_attr, new_attr))?;
        }

        Some(())
    }

    fn hotreload_attribute(
        &mut self,
        old_attr: &Attribute,
        new_attr: &Attribute,
        score: usize,
    ) -> Option<()> {
        // If the score is 0, the name didn't match or the values didn't match
        // A score of usize::MAX means the attributes are the same
        if score == 0 {
            return None;
        }

        // If it's a perfect match, we don't need to do anything special
        // ... well actually if it's a lit we need to invalidate the old lit
        // this is because a lit going from true -> false -> true doesn't count as a change from the diffing perspective
        if score == usize::MAX {
            // if score == usize::MAX && new_attr.as_lit().is_none() {
            return Some(());
        }

        // Prep the new literal
        let location = self.make_location(old_attr.as_lit().unwrap().hr_idx.get());

        // If we have a perfect match and no old lit exists, then this didn't change
        if score == usize::MAX && self.prev_lits.remove(&location).is_none() {
            return Some(());
        }

        let out = match &new_attr.as_lit().unwrap().value {
            HotLiteralType::Float(f) => HotReloadLiteral::Float(f.base10_parse().unwrap()),
            HotLiteralType::Int(f) => HotReloadLiteral::Int(f.base10_parse().unwrap()),
            HotLiteralType::Bool(f) => HotReloadLiteral::Bool(f.value),
            HotLiteralType::Fmted(new) => HotReloadLiteral::Fmted(
                IfmtInput::fmt_segments(old_attr.ifmt().unwrap(), new)
                    .expect("Fmt segments to generate"),
            ),
        };

        self.changed_lits.insert(location, out);

        Some(())
    }

    fn make_location(&self, idx: usize) -> String {
        format!("{}:{}", self.location.trim_end_matches(":0"), idx)
    }

    /// Hot reload an if chain
    fn hotreload_ifchain<Ctx: HotReloadingContext>(
        &mut self,
        a: &IfChain,
        b: &IfChain,
    ) -> Option<bool> {
        let matches = a.cond == b.cond;

        if matches {
            let (mut elif_a, mut elif_b) = (Some(a), Some(b));

            loop {
                // No point in continuing if we've hit the end of the chain
                if elif_a.is_none() && elif_b.is_none() {
                    break;
                }

                // We assume both exist branches exist
                let (a, b) = (elif_a.take()?, elif_b.take()?);

                // Write the `then` branch
                self.hotreload_body::<Ctx>(&a.then_branch, &b.then_branch)?;

                // If there's an elseif branch, we set that as the next branch
                // Otherwise we continue to the else branch - which we assume both branches have
                if let (Some(left), Some(right)) =
                    (a.else_if_branch.as_ref(), b.else_if_branch.as_ref())
                {
                    elif_a = Some(left.as_ref());
                    elif_b = Some(right.as_ref());
                    continue;
                }

                // No else branches, that's fine
                if a.else_branch.is_none() && b.else_branch.is_none() {
                    break;
                }

                // Write out the else branch and then we're done
                let (left, right) = (a.else_branch.as_ref()?, b.else_branch.as_ref()?);
                self.hotreload_body::<Ctx>(left, right)?;
                break;
            }
        }

        Some(matches)
    }
}
