//! This module contains the diffing logic for rsx hot reloading.
//!
//! There's a few details that I wish we could've gotten right but we can revisit later:
//!
//! - Expanding an if chain is not possible - only its contents can be hot reloaded
//!
//! - Components that don't start with children can't be hot reloaded - IE going from `Comp {}` to `Comp { "foo" }`
//!   is not possible. We could in theory allow this by seeding all Components with a `children` field.
//!
//! - Cross-templates hot reloading is not possible - multiple templates don't share the dynamic pool. This would require handling aliases
//!   in hot reload diffing.
//!
//! - We've proven that binary patching is feasible but has a longer path to stabilization for all platforms.
//!   Binary patching is pretty quick, actually, and *might* remove the need to literal hot reloading.
//!   However, you could imagine a scenario where literal hot reloading would be useful without the
//!   compiler in the loop. Ideally we can slash most of this code once patching is stable.
//!
//! ## Assigning/Scoring Templates
//!
//! We can clone most dynamic items from the last full rebuild:
//! - Dynamic text segments: `div { width: "{x}%" } -> div { width: "{x}%", height: "{x}%" }`
//! - Dynamic attributes: `div { width: dynamic } -> div { width: dynamic, height: dynamic }`
//! - Dynamic nodes: `div { {children} } -> div { {children} {children} }`
//!
//! But we cannot clone rsx bodies themselves because we cannot hot reload the new rsx body:
//! - `div { Component { "{text}" } } -> div { Component { "{text}" } Component { "hello" } }` // We can't create a template for both "{text}" and "hello"
//!
//! In some cases, two nodes with children are ambiguous. For example:
//! ```rust, ignore
//! rsx! {
//!     div {
//!         Component { "{text}" }
//!         Component { "hello" }
//!     }
//! }
//! ```
//!
//! Outside of the template, both components are compatible for hot reloading.
//!
//! After we create a list of all components with compatible names and props, we need to find the best match for the
//! template.
//!
//!
//! Dioxus uses a greedy algorithm to find the best match. We first try to create the child template with the dynamic context from the last full rebuild.
//! Then we use the child template that leaves the least unused dynamic items in the pool to create the new template.
//!
//! For the example above:
//! - Hot reloading `Component { "hello" }`:
//!   - Try to hot reload the component body `"hello"` with the dynamic pool from `"{text}"`: Success with 1 unused dynamic item
//!   - Try to hot reload the component body `"hello"` with the dynamic pool from `"hello"`: Success with 0 unused dynamic items
//!   - We use the the template that leaves the least unused dynamic items in the pool - `"hello"`
//! - Hot reloading `Component { "{text}" }`:
//!   - Try to hot reload the component body `"{text}"` with the dynamic pool from `"{text}"`: Success with 0 unused dynamic items
//!   - The `"hello"` template has already been hot reloaded, so we don't try to hot reload it again
//!   - We use the the template that leaves the least unused dynamic items in the pool - `"{text}"`
//!
//! Greedy algorithms are optimal when:
//! - The step we take reduces the problem size
//! - The subproblem is optimal
//!
//! In this case, hot reloading a template removes it from the pool of templates we can use to hot reload the next template which reduces the problem size.
//!
//! The subproblem is optimal because the alternative is leaving less dynamic items for the remaining templates to hot reload which just makes it
//! more difficult to match future templates.

use dioxus_core::internal::{
    FmtedSegments, HotReloadAttributeValue, HotReloadDynamicAttribute, HotReloadDynamicNode,
    HotReloadLiteral, HotReloadedTemplate, NamedAttribute,
};
use dioxus_core_types::HotReloadingContext;
use dioxus_rsx::*;
use std::collections::HashMap;

use crate::extensions::{html_tag_and_namespace, intern, to_template_node};

use super::last_build_state::LastBuildState;

/// A result of hot reloading
///
/// This contains information about what has changed so the hotreloader can apply the right changes
#[non_exhaustive]
#[derive(Debug, PartialEq, Clone)]
pub struct HotReloadResult {
    /// The child templates we have already used. As we walk through the template tree, we will run into child templates.
    /// Each of those child templates also need to be hot reloaded. We keep track of which ones we've already hotreloaded
    /// to avoid diffing the same template twice against different new templates.
    ///
    /// ```rust, ignore
    /// rsx! {
    ///     Component { class: "{class}", "{text}" } // The children of a Component is a new template
    ///     for item in items {
    ///         "{item}" // The children of a for loop is a new template
    ///     }
    ///     if true {
    ///         "{text}" // The children of an if chain is a new template
    ///     }
    /// }
    /// ```
    ///
    /// If we hotreload the component, we don't need to hotreload the for loop
    ///
    /// You should diff the result of this against the old template to see if you actually need to send down the result
    pub templates: HashMap<usize, HotReloadedTemplate>,

    /// The state of the last full rebuild.
    full_rebuild_state: LastBuildState,

    /// The dynamic nodes for the current node
    dynamic_nodes: Vec<HotReloadDynamicNode>,

    /// The dynamic attributes for the current node
    dynamic_attributes: Vec<HotReloadDynamicAttribute>,

    /// The literal component properties for the current node
    literal_component_properties: Vec<HotReloadLiteral>,
}

impl HotReloadResult {
    /// Calculate the hot reload diff between two template bodies
    pub fn new<Ctx: HotReloadingContext>(
        full_rebuild_state: &TemplateBody,
        new: &TemplateBody,
        name: String,
    ) -> Option<Self> {
        // Normalize both the full rebuild state and the new state for rendering
        let full_rebuild_state = full_rebuild_state.normalized();
        let new = new.normalized();
        let full_rebuild_state = LastBuildState::new(&full_rebuild_state, name);
        let mut s = Self {
            full_rebuild_state,
            templates: Default::default(),
            dynamic_nodes: Default::default(),
            dynamic_attributes: Default::default(),
            literal_component_properties: Default::default(),
        };

        s.hotreload_body::<Ctx>(&new)?;

        Some(s)
    }

    fn extend(&mut self, other: Self) {
        self.templates.extend(other.templates);
    }

    /// Walk the dynamic contexts and do our best to find hot reload-able changes between the two
    /// sets of dynamic nodes/attributes. If there's a change we can't hot reload, we'll return None
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
    /// Generally we can't hot reload a node if:
    /// - We add or modify a new rust expression
    ///   - Adding a new formatted segment we haven't seen before
    ///   - Adding a new dynamic node (loop, fragment, if chain, etc)
    /// - We add a new component field
    /// - We remove a component field
    /// - We change the type of a component field
    ///
    /// If a dynamic node is removed, we don't necessarily need to kill hot reload - just unmounting it should be enough
    /// If the dynamic node is re-added, we want to be able to find it again.
    ///
    /// This encourages the hot reloader to hot onto DynamicContexts directly instead of the CallBody since
    /// you can preserve more information about the nodes as they've changed over time.
    fn hotreload_body<Ctx: HotReloadingContext>(&mut self, new: &TemplateBody) -> Option<()> {
        // Quickly run through dynamic attributes first attempting to invalidate them
        // Move over old IDs onto the new template
        self.hotreload_attributes::<Ctx>(new)?;
        let new_dynamic_attributes = std::mem::take(&mut self.dynamic_attributes);

        // Now we can run through the dynamic nodes and see if we can hot reload them
        // Move over old IDs onto the new template
        self.hotreload_dynamic_nodes::<Ctx>(new)?;
        let new_dynamic_nodes = std::mem::take(&mut self.dynamic_nodes);
        let literal_component_properties = std::mem::take(&mut self.literal_component_properties);

        let key = self.hot_reload_key(new)?;

        let roots: Vec<_> = new
            .roots
            .iter()
            .map(|node| to_template_node::<Ctx>(node))
            .collect();
        let roots: &[dioxus_core::TemplateNode] = intern(&*roots);

        let template = HotReloadedTemplate::new(
            key,
            new_dynamic_nodes,
            new_dynamic_attributes,
            literal_component_properties,
            roots,
        );

        self.templates
            .insert(self.full_rebuild_state.root_index.get(), template);

        Some(())
    }

    fn hot_reload_key(&mut self, new: &TemplateBody) -> Option<Option<FmtedSegments>> {
        match new.implicit_key() {
            Some(AttributeValue::AttrLiteral(HotLiteral::Fmted(value))) => Some(Some(
                self.full_rebuild_state
                    .hot_reload_formatted_segments(value)?,
            )),
            None => Some(None),
            _ => None,
        }
    }

    fn hotreload_dynamic_nodes<Ctx: HotReloadingContext>(
        &mut self,
        new: &TemplateBody,
    ) -> Option<()> {
        for new_node in new.dynamic_nodes() {
            self.hot_reload_node::<Ctx>(new_node)?
        }

        Some(())
    }

    fn hot_reload_node<Ctx: HotReloadingContext>(&mut self, node: &BodyNode) -> Option<()> {
        match node {
            BodyNode::Text(text) => self.hotreload_text_node(text),
            BodyNode::Component(component) => self.hotreload_component::<Ctx>(component),
            BodyNode::ForLoop(forloop) => self.hotreload_for_loop::<Ctx>(forloop),
            BodyNode::IfChain(ifchain) => self.hotreload_if_chain::<Ctx>(ifchain),
            BodyNode::RawExpr(expr) => self.hotreload_raw_expr(expr),
            BodyNode::Element(_) => Some(()),
        }
    }

    fn hotreload_raw_expr(&mut self, expr: &ExprNode) -> Option<()> {
        // Try to find the raw expr in the last build
        let expr_index = self
            .full_rebuild_state
            .dynamic_nodes
            .position(|node| match &node {
                BodyNode::RawExpr(raw_expr) => raw_expr.expr == expr.expr,
                _ => false,
            })?;

        // If we find it, push it as a dynamic node
        self.dynamic_nodes
            .push(HotReloadDynamicNode::Dynamic(expr_index));

        Some(())
    }

    fn hotreload_for_loop<Ctx>(&mut self, forloop: &ForLoop) -> Option<()>
    where
        Ctx: HotReloadingContext,
    {
        // Find all for loops that have the same pattern and expression
        let candidate_for_loops = self
            .full_rebuild_state
            .dynamic_nodes
            .inner
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                if let BodyNode::ForLoop(for_loop) = &node.inner {
                    if for_loop.pat == forloop.pat && for_loop.expr == forloop.expr {
                        return Some((index, for_loop));
                    }
                }
                None
            })
            .collect::<Vec<_>>();

        // Then find the one that has the least wasted dynamic items when hot reloading the body
        let (index, best_call_body) = self.diff_best_call_body::<Ctx>(
            candidate_for_loops
                .iter()
                .map(|(_, for_loop)| &for_loop.body),
            &forloop.body,
        )?;

        // Push the new for loop as a dynamic node
        self.dynamic_nodes
            .push(HotReloadDynamicNode::Dynamic(candidate_for_loops[index].0));

        self.extend(best_call_body);

        Some(())
    }

    fn hotreload_text_node(&mut self, text_node: &TextNode) -> Option<()> {
        // If it is static, it is already included in the template and we don't need to do anything
        if text_node.input.is_static() {
            return Some(());
        }
        // Otherwise, hot reload the formatted segments and push that as a dynamic node
        let formatted_segments = self
            .full_rebuild_state
            .hot_reload_formatted_segments(&text_node.input)?;
        self.dynamic_nodes
            .push(HotReloadDynamicNode::Formatted(formatted_segments));
        Some(())
    }

    /// Find the call body that minimizes the number of wasted dynamic items
    ///
    /// Returns the index of the best call body and the state of the best call body
    fn diff_best_call_body<'a, Ctx>(
        &self,
        bodies: impl Iterator<Item = &'a TemplateBody>,
        new_call_body: &TemplateBody,
    ) -> Option<(usize, Self)>
    where
        Ctx: HotReloadingContext,
    {
        let mut best_score = usize::MAX;
        let mut best_output = None;
        for (index, body) in bodies.enumerate() {
            // Skip templates we've already hotreloaded
            if self.templates.contains_key(&body.template_idx.get()) {
                continue;
            }
            if let Some(state) =
                Self::new::<Ctx>(body, new_call_body, self.full_rebuild_state.name.clone())
            {
                let score = state.full_rebuild_state.unused_dynamic_items();
                if score < best_score {
                    best_score = score;
                    best_output = Some((index, state));
                }
            }
        }

        best_output
    }

    fn hotreload_component<Ctx>(&mut self, component: &Component) -> Option<()>
    where
        Ctx: HotReloadingContext,
    {
        // First we need to find the component that matches the best in the last build
        // We try each build and choose the option that wastes the least dynamic items
        let components_with_matching_attributes: Vec<_> = self
            .full_rebuild_state
            .dynamic_nodes
            .inner
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                if let BodyNode::Component(comp) = &node.inner {
                    return Some((
                        index,
                        comp,
                        self.hotreload_component_fields(comp, component)?,
                    ));
                }
                None
            })
            .collect();

        let possible_bodies = components_with_matching_attributes
            .iter()
            .map(|(_, comp, _)| &comp.children);

        let (index, new_body) =
            self.diff_best_call_body::<Ctx>(possible_bodies, &component.children)?;

        let (index, _, literal_component_properties) = &components_with_matching_attributes[index];
        let index = *index;

        self.full_rebuild_state.dynamic_nodes.inner[index]
            .used
            .set(true);

        self.literal_component_properties
            .extend(literal_component_properties.iter().cloned());

        self.extend(new_body);

        // Push the new component as a dynamic node
        self.dynamic_nodes
            .push(HotReloadDynamicNode::Dynamic(index));

        Some(())
    }

    fn hotreload_component_fields(
        &self,
        old_component: &Component,
        new_component: &Component,
    ) -> Option<Vec<HotReloadLiteral>> {
        // First check if the component is the same
        if new_component.name != old_component.name {
            return None;
        }

        // Then check if the fields are the same
        let new_non_key_fields: Vec<_> = new_component.component_props().collect();
        let old_non_key_fields: Vec<_> = old_component.component_props().collect();
        if new_non_key_fields.len() != old_non_key_fields.len() {
            return None;
        }

        let mut new_fields = new_non_key_fields.clone();
        new_fields.sort_by_key(|attribute| attribute.name.to_string());
        let mut old_fields = old_non_key_fields.iter().enumerate().collect::<Vec<_>>();
        old_fields.sort_by_key(|(_, attribute)| attribute.name.to_string());

        // The literal component properties for the component in same the order as the original component property literals
        let mut literal_component_properties = vec![None; old_fields.len()];

        for (new_field, (index, old_field)) in new_fields.iter().zip(old_fields.iter()) {
            // Verify the names match
            if new_field.name != old_field.name {
                return None;
            }

            // Verify the values match
            match (&new_field.value, &old_field.value) {
                // If the values are both literals, we can try to hotreload them
                (
                    AttributeValue::AttrLiteral(new_value),
                    AttributeValue::AttrLiteral(old_value),
                ) => {
                    // Make sure that the types are the same
                    if std::mem::discriminant(new_value) != std::mem::discriminant(old_value) {
                        return None;
                    }
                    let literal = self.full_rebuild_state.hotreload_hot_literal(new_value)?;
                    literal_component_properties[*index] = Some(literal);
                }
                _ => {
                    if new_field.value != old_field.value {
                        return None;
                    }
                }
            }
        }

        Some(literal_component_properties.into_iter().flatten().collect())
    }

    /// Hot reload an if chain
    fn hotreload_if_chain<Ctx: HotReloadingContext>(
        &mut self,
        new_if_chain: &IfChain,
    ) -> Option<()> {
        let mut best_if_chain = None;
        let mut best_score = usize::MAX;

        let if_chains = self
            .full_rebuild_state
            .dynamic_nodes
            .inner
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                if let BodyNode::IfChain(if_chain) = &node.inner {
                    return Some((index, if_chain));
                }
                None
            });

        // Find the if chain that matches all of the conditions and wastes the least dynamic items
        for (index, old_if_chain) in if_chains {
            let Some(chain_templates) = Self::diff_if_chains::<Ctx>(
                old_if_chain,
                new_if_chain,
                self.full_rebuild_state.name.clone(),
            ) else {
                continue;
            };
            let score = chain_templates
                .iter()
                .map(|t| t.full_rebuild_state.unused_dynamic_items())
                .sum();
            if score < best_score {
                best_score = score;
                best_if_chain = Some((index, chain_templates));
            }
        }

        // If we found a hot reloadable if chain, hotreload it
        let (index, chain_templates) = best_if_chain?;
        // Mark the if chain as used
        self.full_rebuild_state.dynamic_nodes.inner[index]
            .used
            .set(true);
        // Merge the hot reload changes into the current state
        for template in chain_templates {
            self.extend(template);
        }

        // Push the new if chain as a dynamic node
        self.dynamic_nodes
            .push(HotReloadDynamicNode::Dynamic(index));

        Some(())
    }

    /// Hot reload an if chain
    fn diff_if_chains<Ctx: HotReloadingContext>(
        old_if_chain: &IfChain,
        new_if_chain: &IfChain,
        name: String,
    ) -> Option<Vec<Self>> {
        // Go through each part of the if chain and find the best match
        let mut old_chain = old_if_chain;
        let mut new_chain = new_if_chain;

        let mut chain_templates = Vec::new();

        loop {
            // Make sure the conditions are the same
            if old_chain.cond != new_chain.cond {
                return None;
            }

            // If the branches are the same, we can hotreload them
            let hot_reload =
                Self::new::<Ctx>(&old_chain.then_branch, &new_chain.then_branch, name.clone())?;
            chain_templates.push(hot_reload);

            // Make sure the if else branches match
            match (
                old_chain.else_if_branch.as_ref(),
                new_chain.else_if_branch.as_ref(),
            ) {
                (Some(old), Some(new)) => {
                    old_chain = old;
                    new_chain = new;
                }
                (None, None) => {
                    break;
                }
                _ => return None,
            }
        }
        // Make sure the else branches match
        match (&old_chain.else_branch, &new_chain.else_branch) {
            (Some(old), Some(new)) => {
                let template = Self::new::<Ctx>(old, new, name.clone())?;
                chain_templates.push(template);
            }
            (None, None) => {}
            _ => return None,
        }

        Some(chain_templates)
    }

    /// Take a new template body and return the attributes that can be hot reloaded from the last build
    ///
    /// IE if we shuffle attributes, remove attributes or add new attributes with the same dynamic segments, around we should be able to hot reload them.
    ///
    /// ```rust, ignore
    /// rsx! {
    ///     div { id: "{id}", class: "{class}", width, "Hi" }
    /// }
    ///
    /// rsx! {
    ///     div { width, class: "{class}", id: "{id} and {class}", "Hi" }
    /// }
    /// ```
    fn hotreload_attributes<Ctx: HotReloadingContext>(&mut self, new: &TemplateBody) -> Option<()> {
        // Walk through each attribute and create a new HotReloadAttribute for each one
        for new_attr in new.dynamic_attributes() {
            // While we're here, if it's a literal and not a perfect score, it's a mismatch and we need to
            // hotreload the literal
            self.hotreload_attribute::<Ctx>(new_attr)?;
        }

        Some(())
    }

    /// Try to hot reload an attribute and return the new HotReloadAttribute
    fn hotreload_attribute<Ctx: HotReloadingContext>(
        &mut self,
        attribute: &Attribute,
    ) -> Option<()> {
        let (tag, namespace) = html_tag_and_namespace::<Ctx>(attribute);

        // If the attribute is a spread, try to grab it from the last build
        // If it wasn't in the last build with the same name, we can't hot reload it
        if let AttributeName::Spread(_) = &attribute.name {
            let hot_reload_attribute = self
                .full_rebuild_state
                .dynamic_attributes
                .position(|a| a.name == attribute.name && a.value == attribute.value)?;
            self.dynamic_attributes
                .push(HotReloadDynamicAttribute::Dynamic(hot_reload_attribute));

            return Some(());
        }

        // Otherwise the attribute is named, try to hot reload the value
        let value = match &attribute.value {
            // If the attribute is a literal, we can generally hot reload it if the formatted segments exist in the last build
            AttributeValue::AttrLiteral(literal) => {
                // If it is static, it is already included in the template and we don't need to do anything
                if literal.is_static() {
                    return Some(());
                }
                // Otherwise, hot reload the literal and push that as a dynamic attribute
                let hot_reload_literal = self.full_rebuild_state.hotreload_hot_literal(literal)?;
                HotReloadAttributeValue::Literal(hot_reload_literal)
            }
            // If it isn't a literal, try to find an exact match for the attribute value from the last build
            _ => {
                let value_index = self.full_rebuild_state.dynamic_attributes.position(|a| {
                    // Spread attributes are not hot reloaded
                    if matches!(a.name, AttributeName::Spread(_)) {
                        return false;
                    }
                    if a.value != attribute.value {
                        return false;
                    }
                    // The type of event handlers is influenced by the event name, so te cannot hot reload between different event
                    // names
                    if matches!(a.value, AttributeValue::EventTokens(_)) && a.name != attribute.name
                    {
                        return false;
                    }
                    true
                })?;
                HotReloadAttributeValue::Dynamic(value_index)
            }
        };

        self.dynamic_attributes
            .push(HotReloadDynamicAttribute::Named(NamedAttribute::new(
                tag, namespace, value,
            )));

        Some(())
    }
}
