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

use crate::innerlude::*;
use crate::HotReloadingContext;
use dioxus_core::internal::{
    FmtSegment, FmtedSegments, HotReloadAttribute, HotReloadAttributeValue, HotReloadDynamicNode,
    HotReloadLiteral, HotReloadedTemplate, NamedAttribute,
};
use std::cell::Cell;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
struct BakedItem<T> {
    inner: T,
    used: Cell<bool>,
}

impl<T> BakedItem<T> {
    fn new(inner: T) -> Self {
        Self {
            inner,
            used: Cell::new(false),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
struct BakedPool<T> {
    inner: Vec<BakedItem<T>>,
}

impl<T> BakedPool<T> {
    fn new(inner: impl IntoIterator<Item = T>) -> Self {
        Self {
            inner: inner.into_iter().map(BakedItem::new).collect(),
        }
    }

    fn position(&self, condition: impl Fn(&T) -> bool) -> Option<usize> {
        for (idx, baked_item) in self.inner.iter().enumerate() {
            if condition(&baked_item.inner) {
                baked_item.used.set(true);
                return Some(idx);
            }
        }
        None
    }

    fn unused_dynamic_items(&self) -> usize {
        self.inner
            .iter()
            .filter(|baked_item| !baked_item.used.get())
            .count()
    }

    fn reset_usage(&mut self) {
        for baked_item in self.inner.iter_mut() {
            baked_item.used.set(false);
        }
    }
}

/// The state of the last full rebuild.
/// This object contains the pool of compiled dynamic segments we can pull from for hot reloading
#[derive(Debug, PartialEq, Clone)]
pub struct LastBuildState {
    /// The formatted segments that were used in the last build. Eg: "{class}", "{id}"
    ///
    /// We are free to use each of these segments many times in the same build.
    /// We just clone the result (assuming display + debug have no side effects)
    dynamic_text_segments: BakedPool<FormattedSegment>,
    /// The dynamic nodes that were used in the last build. Eg: div { {children} }
    ///
    /// We are also free to clone these nodes many times in the same build.
    dynamic_nodes: BakedPool<BodyNode>,
    /// The attributes that were used in the last build. Eg: div { class: "{class}" }
    ///
    /// We are also free to clone these nodes many times in the same build.
    dynamic_attributes: BakedPool<Attribute>,
    /// The component literal properties we can hot reload from the last build. Eg: Component { class: "{class}" }
    ///
    /// In the new build, we must assign each of these a value even if we no longer use the component.
    /// The type must be the same as the last time we compiled the property
    component_properties: Vec<HotLiteral>,
    /// The root indexes of the last build
    root_index: DynIdx,
}

impl LastBuildState {
    /// Create a new LastBuildState from the given [`TemplateBody`]
    pub fn new(body: &TemplateBody) -> Self {
        let dynamic_text_segments = body.dynamic_text_segments.iter().cloned();
        let dynamic_nodes = body.dynamic_nodes().cloned();
        let dynamic_attributes = body.dynamic_attributes().cloned();
        let component_properties = body.literal_component_properties().cloned().collect();
        Self {
            dynamic_text_segments: BakedPool::new(dynamic_text_segments),
            dynamic_nodes: BakedPool::new(dynamic_nodes),
            dynamic_attributes: BakedPool::new(dynamic_attributes),
            component_properties,
            root_index: body.template_idx.clone(),
        }
    }

    /// Return the number of unused dynamic items in the pool
    pub fn unused_dynamic_items(&self) -> usize {
        self.dynamic_text_segments.unused_dynamic_items()
            + self.dynamic_nodes.unused_dynamic_items()
            + self.dynamic_attributes.unused_dynamic_items()
    }

    /// Reset the usage of the dynamic items in the pool
    pub fn reset_dynamic_items(&mut self) {
        self.dynamic_text_segments.reset_usage();
        self.dynamic_nodes.reset_usage();
        self.dynamic_attributes.reset_usage();
    }

    /// Hot reload a hot literal
    fn hotreload_hot_literal(&self, hot_literal: &HotLiteral) -> Option<HotReloadLiteral> {
        match hot_literal {
            // If the literal is a formatted segment, map the segments to the new formatted segments
            HotLiteral::Fmted(segments) => {
                let new_segments = self.hot_reload_formatted_segments(segments)?;
                Some(HotReloadLiteral::Fmted(new_segments))
            }
            // Otherwise just pass the literal through unchanged
            HotLiteral::Bool(b) => Some(HotReloadLiteral::Bool(b.value())),
            HotLiteral::Float(f) => Some(HotReloadLiteral::Float(f.base10_parse().ok()?)),
            HotLiteral::Int(i) => Some(HotReloadLiteral::Int(i.base10_parse().ok()?)),
        }
    }

    fn hot_reload_formatted_segments(
        &self,
        new: &HotReloadFormattedSegment,
    ) -> Option<FmtedSegments> {
        // Go through each dynamic segment and look for a match in the formatted segments pool.
        // If we find a match, we can hot reload the segment otherwise we need to do a full rebuild
        let mut segments = Vec::new();
        for segment in &new.segments {
            match segment {
                // If it is a literal, we can always hot reload it. Just add it to the segments
                Segment::Literal(value) => {
                    segments.push(FmtSegment::Literal {
                        value: Box::leak(value.clone().into_boxed_str()),
                    });
                } // If it is a dynamic segment, we need to check if it exists in the formatted segments pool
                Segment::Formatted(formatted) => {
                    let index = self.dynamic_text_segments.position(|s| s == formatted)?;

                    segments.push(FmtSegment::Dynamic { id: index });
                }
            }
        }

        Some(FmtedSegments::new(segments))
    }
}

/// A result of hot reloading
///
/// This contains information about what has changed so the hotreloader can apply the right changes
#[non_exhaustive]
#[derive(Debug, PartialEq, Clone)]
pub struct HotReloadState {
    /// The state of the last full rebuild.
    pub full_rebuild_state: LastBuildState,

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
    pub templates: HashMap<DynIdx, HotReloadedTemplate>,

    /// The dynamic nodes for the current node
    dynamic_nodes: Vec<HotReloadDynamicNode>,

    /// The dynamic attributes for the current node
    dynamic_attributes: Vec<HotReloadAttribute>,

    /// The literal component properties for the current node
    literal_component_properties: Vec<HotReloadLiteral>,
}

impl HotReloadState {
    /// Calculate the hotreload diff between two callbodies
    pub fn new<Ctx: HotReloadingContext>(
        full_rebuild_state: LastBuildState,
        new: &TemplateBody,
    ) -> Option<Self> {
        let mut s = Self {
            full_rebuild_state,
            templates: Default::default(),
            dynamic_nodes: Default::default(),
            dynamic_attributes: Default::default(),
            literal_component_properties: Default::default(),
        };

        s.hotreload_body::<Ctx>(new)?;

        Some(s)
    }

    fn extend(&mut self, other: Self) {
        self.templates.extend(other.templates);
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
    /// - We add or modify a new rust expression
    ///   - Adding a new formatted segment we haven't seen before
    ///   - Adding a new dynamic node (loop, fragment, if chain, etc)
    /// - We add a new component field
    /// - We remove a component field
    /// - We change the type of a component field
    ///
    /// If a dynamic node is removed, we don't necessarily need to kill hotreload - just unmounting it should be enough
    /// If the dynamic node is re-added, we want to be able to find it again.
    ///
    /// This encourages the hotreloader to hot onto DynamicContexts directly instead of the CallBody since
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
            .map(|node| node.to_template_node::<Ctx>())
            .collect();
        let roots = intern(&*roots);

        let template = HotReloadedTemplate::new(
            key,
            new_dynamic_nodes,
            new_dynamic_attributes,
            literal_component_properties,
            roots,
        );

        self.templates
            .insert(self.full_rebuild_state.root_index.clone(), template);

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
            if self.templates.contains_key(&body.template_idx) {
                continue;
            }
            if let Some(state) = Self::new::<Ctx>(LastBuildState::new(body), new_call_body) {
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

        self.full_rebuild_state.dynamic_nodes.inner[*index]
            .used
            .set(true);

        self.literal_component_properties
            .extend(literal_component_properties.iter().cloned());

        self.extend(new_body);

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
        if new_component.fields.len() != old_component.fields.len() {
            return None;
        }

        let mut new_fields = new_component.fields.clone();
        new_fields.sort_by(|a, b| a.name.to_string().cmp(&b.name.to_string()));
        let mut old_fields = old_component.fields.clone();
        old_fields.sort_by(|a, b| a.name.to_string().cmp(&b.name.to_string()));

        let mut literal_component_properties = Vec::new();

        for (new_field, old_field) in new_fields.iter().zip(old_fields.iter()) {
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
                    literal_component_properties.push(literal);
                }
                _ => {
                    if new_field.value != old_field.value {
                        return None;
                    }
                }
            }
        }

        Some(literal_component_properties)
    }

    /// Hot reload an if chain
    fn hotreload_if_chain<Ctx: HotReloadingContext>(&mut self, new: &IfChain) -> Option<()> {
        todo!()
        // let (mut elif_a, mut elif_b) = (Some(a), Some(b));

        // loop {
        //     // No point in continuing if we've hit the end of the chain
        //     if elif_a.is_none() && elif_b.is_none() {
        //         break;
        //     }

        //     // We assume both exist branches exist
        //     let (a, b) = (elif_a.take()?, elif_b.take()?);

        //     // Write the `then` branch
        //     self.hotreload_body::<Ctx>(&b.then_branch)?;

        //     // If there's an elseif branch, we set that as the next branch
        //     // Otherwise we continue to the else branch - which we assume both branches have
        //     if let (Some(left), Some(right)) =
        //         (a.else_if_branch.as_ref(), b.else_if_branch.as_ref())
        //     {
        //         elif_a = Some(left.as_ref());
        //         elif_b = Some(right.as_ref());
        //         continue;
        //     }

        //     // No else branches, that's fine
        //     if a.else_branch.is_none() && b.else_branch.is_none() {
        //         break;
        //     }

        //     // Write out the else branch and then we're done
        //     let (left, right) = (a.else_branch.as_ref()?, b.else_branch.as_ref()?);
        //     self.hotreload_body::<Ctx>(right)?;
        //     break;
        // }

        // Some(matches)
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
        let (tag, namespace) = attribute.html_tag_and_namespace::<Ctx>();

        // If the attribute is a spread, try to grab it from the last build
        // If it wasn't in the last build with the same name, we can't hot reload it
        if let AttributeName::Spread(_) = &attribute.name {
            let hot_reload_attribute = self
                .full_rebuild_state
                .dynamic_attributes
                .position(|a| a.name == attribute.name && a.value == attribute.value)?;
            self.dynamic_attributes
                .push(HotReloadAttribute::Spread(hot_reload_attribute));

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
                    !matches!(a.name, AttributeName::Spread(_)) && a.value == attribute.value
                })?;
                HotReloadAttributeValue::Dynamic(value_index)
            }
        };

        self.dynamic_attributes
            .push(HotReloadAttribute::Named(NamedAttribute::new(
                tag, namespace, value,
            )));

        Some(())
    }
}
