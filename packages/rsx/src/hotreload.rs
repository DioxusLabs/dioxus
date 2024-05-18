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
//! - Hotreloading of formatted strings is currently not possible - we can't hot reload the formatting.
//!   This might be fixable!
//!
//! - Hotreloading of literals is technically possible, but not currently implemented and would likely
//!   require changes to core to work.

use std::collections::HashMap;

use crate::{
    intern, AttributeType, Component, ComponentField, ContentField, ElementAttrName, ForLoop,
    IfChain, IfmtInput, TemplateBody, TextNode,
};
use crate::{BodyNode, CallBody, DynamicContext, HotReloadingContext};
use dioxus_core::{
    prelude::{FmtSegment, FmtedSegments, Template},
    TemplateAttribute, TemplateNode,
};

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

type DynamicNodeIdx = usize;

/// A result of hot reloading
///
/// This contains information about what has changed so the hotreloader can apply the right changes
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HotReload {
    pub templates: Vec<Template>,

    // The location of the original call
    // This should be in the form of `file:line:col:0` - 0 since this will be the base template
    pub location: &'static str,

    pub changed_strings: HashMap<String, FmtedSegments>,
}

impl HotReload {
    pub fn new<Ctx: HotReloadingContext>(
        old: &CallBody,
        new: &CallBody,
        location: &'static str,
    ) -> Option<Self> {
        let mut s = Self {
            templates: Default::default(),
            changed_strings: Default::default(),
            location,
        };
        s.hotreload_callbody::<Ctx>(old, new)?;

        Some(s)
    }

    fn make_location(&self, idx: usize) -> String {
        format!("{}:{}", self.location.trim_end_matches(":0"), idx)
    }

    /// The `old` callbody is the original callbody that was used to create the template
    ///
    /// The new one is the one that was used to update the template.
    /// We never update the old in a hotreloading session since that's our only source of truth for dynamic
    /// nodes, especially the ones actively living in the renderer. All new templates are simply a
    /// transformation of the original. This lets us preserve dynamic nodes between hotreloads.
    ///
    /// Location is in the form of `file:line:col:byte_index`
    pub fn hotreload_callbody<Ctx: HotReloadingContext>(
        &mut self,
        old: &CallBody,
        new: &CallBody,
    ) -> Option<()> {
        self.hotreload_body::<Ctx>(&old.body, &new.body)
    }

    /// Walk the dynamic contexts and do our best to find hotreloadable changes between the two
    /// sets of dynamic nodes/attributes. If there's a change we can't hotreload, we'll return None
    ///
    /// Otherwise, we pump out the list of templates that need to be updated.
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
        old_body: &TemplateBody,
        new_body: &TemplateBody,
    ) -> Option<()> {
        // Create a context that will be used to update the template
        let old = &DynamicContext::from_body::<Ctx>(&old_body.roots);
        let new = &DynamicContext::from_body::<Ctx>(&new_body.roots);

        // Quickly run through dynamic attributes first attempting to invalidate them
        let new_attribute_paths = self.hotreload_attributes::<Ctx>(old, new)?;

        // Now we can run through the dynamic nodes and see if we can hot reload them
        let new_node_paths = self.hotreload_dynamic_nodes::<Ctx>(old, new)?;

        // Create the new template nodes from the dynamic context, but with the new mapping
        let roots = self.render_dynamic_context::<Ctx>(
            new,
            new_body,
            &new_node_paths,
            &new_attribute_paths,
        );

        // Now we can assemble a template
        self.templates.push(Template {
            name: self.make_location(old_body.location.get()).leak(),
            roots: intern(roots.as_slice()),
            node_paths: intern(
                new_node_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
            attr_paths: intern(
                new_attribute_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
        });

        Some(())
    }

    /// Take two dynamic contexts and return a new node_paths field for the final template.
    ///
    /// The `node_paths` field needs to be in order of the original list of dynamic nodes since that
    /// is baked into the running code (can't change it). Basically, we're going to provide a new
    /// location for all the dynamic nodes in the template without actually changing the order of the
    /// dynamic nodes - only their paths. The strategy here is to simply walk the old nodes and find their
    /// corresponding match in the new nodes. We then push the new node's path into the `node_paths` field
    /// of the template.
    ///
    /// ```
    /// for old in old {
    ///     let new = new.find(old);
    ///     if let Some(new) = new {
    ///         node_paths.push(new.path);
    ///         remove_new_node_from_search_list();
    ///     } else {
    ///         node_paths.push(DUD_PATH); // IE this node got removed
    ///     }
    /// }
    ///
    /// // If a new dynamic node appeared, it's not hotreloadable and thus we have to abort
    /// if !search_list.is_empty() {
    ///     return None;
    /// }
    /// ```
    ///
    /// IE if we shuffle nodes around we should be able to still hot reload them. This kinda assumes that
    /// rendering has no side effects, but that's a reasonable assumption.
    ///
    /// ```rust
    /// // old
    /// rsx! {
    ///     h1 { "hi" }
    ///     div { id: "{id}", "Hi" }
    /// }
    ///
    ///
    /// // new
    /// rsx! {
    ///     div { id: "{id}", "Hi" }
    ///     h1 { "hi" }
    /// }
    /// ```
    fn hotreload_dynamic_nodes<Ctx: HotReloadingContext>(
        &mut self,
        old: &DynamicContext<'_>,
        new: &DynamicContext<'_>,
    ) -> Option<Vec<NodePath>> {
        // Build a list of new nodes that we'll use for scans later
        // Whenever we find a new node, we'll mark the node as `None` so it's skipped on the next scan
        // This is quadratic with number of dynamic nodes, but the checks are *really* fast, hotreloads
        // are not a common the common case, most templates are coming in sorted, etc. In reality it's
        // faster than computing a hash for every node.
        //
        // We could implement an optimization where `Nones` get swapped to the end, or something faster
        // so the linear scans are usually quick, but for the sake of simplicity we'll just do a linear
        // scan where most of the time we're comparing a None against a Some
        let mut new_nodes = new
            .dynamic_nodes
            .iter()
            .map(|f| Some(f))
            .collect::<Vec<_>>();

        // We're going to try returning the mapping of old node to new node
        // IE the new template has scrambled the dynamic nodes and we're going to return the IDs of the
        // original dynamic nodes but now in a new order.
        //
        //  - div { "{one}" "{two}" "{three}" } has order [0, 1, 2]
        //
        // if we see
        //
        //  - div { "{three}" "{one}" "{two}" }
        //
        // we want to return [2, 0, 1]
        //
        // The nodes still exist, mounted in the dom, but the order of the nodes has changed.
        // We need to return the new order of the nodes.
        let mut node_paths = Vec::new();

        // Walk the original template trying to find a match in the new template
        // This ensures the nodepaths come out in the same order as the original template since the
        // dynamic nodes are baked into the running code
        'outer: for (old_idx, old_node) in old.dynamic_nodes.iter().enumerate() {
            // Find the new node
            'inner: for (new_idx, maybe_new_node) in new_nodes.iter_mut().enumerate() {
                // Skip over nodes that we've already found
                // We could use another datastructure like a queue or linked list but this is fine
                let Some(new_node) = maybe_new_node else {
                    continue 'inner;
                };

                let is_match = match (old_node, new_node) {
                    // Elements are not dynamic nodes... nothing to do here
                    (BodyNode::Element(_), BodyNode::Element(_)) => unreachable!(),

                    // Text nodes can be dynamic nodes assuming they're formatted
                    // Eventually we might enable hotreloading of formatted text nodes too, but for now
                    // just check if the ifmt input is the same
                    (BodyNode::Text(a), BodyNode::Text(b)) => {
                        self.hotreload_text_segment::<Ctx>(a, b)?
                    }

                    // Nothing special for raw expressions - if an expresison changed we couldn't find it anyway
                    (BodyNode::RawExpr(a), BodyNode::RawExpr(b)) => a == b,

                    // If we found a matching forloop, its body might not be the same
                    // the bodies don't need to be the same but the pats/exprs do
                    (BodyNode::ForLoop(a), BodyNode::ForLoop(b)) => {
                        self.hotreload_forloop::<Ctx>(a, b)?
                    }

                    // Basically stealing the same logic as the for loop
                    (BodyNode::Component(a), BodyNode::Component(b)) => {
                        self.hotreload_component_body::<Ctx>(a, b)?
                    }

                    // Basically stealing the same logic as the for loop, but with multiple nestings
                    // We only support supports conditions
                    (BodyNode::IfChain(a), BodyNode::IfChain(b)) => {
                        self.hotreload_ifchain::<Ctx>(a, b)?
                    }

                    // Any other pairing is not a match and we should keep looking
                    _ => false,
                };

                if is_match {
                    // We found a match! Get this dynamic node's path and push it into the output
                    node_paths.push(new.node_paths[new_idx].clone());

                    // And then mark the original node as `None` so it's skipped on the next scan
                    _ = maybe_new_node.take();

                    // We're done looking for the match, so we can move out to the next dynamic node
                    continue 'outer;
                }
            }

            // Couldn't find the new node, so we're basically going to return a dud path that dioxus-core
            // knows to not render
            node_paths.push(vec![]);
        }

        // If there's any lingering new nodes, they can't be hot reloaded
        if new_nodes.iter().any(|n| n.is_some()) {
            return None;
        }

        Some(node_paths)
    }

    fn hotreload_text_segment<Ctx: HotReloadingContext>(
        &mut self,
        a: &TextNode,
        b: &TextNode,
    ) -> Option<bool> {
        self.hotreload_ifmt(&a.input, &b.input)
    }

    fn hotreload_ifmt(&mut self, a: &IfmtInput, b: &IfmtInput) -> Option<bool> {
        if a.is_static() && b.is_static() {
            return Some(a == b);
        }

        // Make sure all the dynamic segments of b show up in a
        for segment in b.segments.iter() {
            if segment.is_formatted() && !a.segments.contains(segment) {
                return None;
            }
        }

        // Collect all the formatted segments from the original
        let mut out = vec![];

        // the original list of formatted segments
        let mut fmted = a
            .segments
            .iter()
            .flat_map(|f| match f {
                crate::Segment::Literal(_) => None,
                crate::Segment::Formatted(f) => Some(f),
            })
            .cloned()
            .map(|f| Some(f))
            .collect::<Vec<_>>();

        for segment in b.segments.iter() {
            match segment {
                crate::Segment::Literal(lit) => {
                    // create a &'static str by leaking the string
                    let lit = Box::leak(lit.clone().into_boxed_str());
                    out.push(FmtSegment::Literal { value: lit });
                }
                crate::Segment::Formatted(fmt) => {
                    // Find the formatted segment in the original
                    // Set it to None when we find it so we don't re-render it on accident
                    let idx = fmted
                        .iter_mut()
                        .position(|_s| {
                            if let Some(s) = _s {
                                if s == fmt {
                                    *_s = None;
                                    return true;
                                }
                            }

                            false
                        })
                        .unwrap();

                    out.push(FmtSegment::Dynamic { id: idx });
                }
            }
        }

        let location = self.make_location(a.location.get());

        self.changed_strings
            .insert(location.to_string(), FmtedSegments::new(out));

        Some(true)
    }

    fn hotreload_forloop<Ctx: HotReloadingContext>(
        &mut self,
        a: &ForLoop,
        b: &ForLoop,
    ) -> Option<bool> {
        let matches = a.pat == b.pat && a.expr == b.expr;
        if matches {
            // We unfortunately cannot currently hot reload for loops that didn't have
            // a body. Usually this is unlikely, but dioxus-core would need to be changed
            // to allow templates with no roots
            let _ = a.body.roots.first()?;
            self.hotreload_body::<Ctx>(&a.body, &b.body)?;
        }

        Some(matches)
    }

    fn hotreload_component_body<Ctx: HotReloadingContext>(
        &mut self,
        a: &Component,
        b: &Component,
    ) -> Option<bool> {
        let matches = a.name == b.name
            && a.prop_gen_args == b.prop_gen_args
            && a.key == b.key
            // && a.fields == b.fields
            && a.manual_props == b.manual_props
            // todo: always just pass in dummy children so we can hotreload them
            // either both empty or both non-empty
            && (!a.children.is_empty() && !b.children.is_empty()
                || a.children.is_empty() && b.children.is_empty());

        if matches {
            self.hotreload_component_fields::<Ctx>(a, b)?;
            self.hotreload_body::<Ctx>(&a.children, &b.children)?;
        }

        Some(matches)
    }

    fn hotreload_component_fields<Ctx: HotReloadingContext>(
        &mut self,
        a: &Component,
        b: &Component,
    ) -> Option<()> {
        // make sure both are the same length
        if a.fields.len() != b.fields.len() {
            return None;
        }

        // Walk the attributes looking for literals
        // Those will have plumbing in the hotreloading code
        // All others just get diffed via tokensa
        for (idx, (old_attr, new_attr)) in a.fields.iter().zip(b.fields.iter()).enumerate() {
            match (&old_attr.content, &new_attr.content) {
                (_, _) if old_attr.name != new_attr.name => return None,
                (ContentField::Formatted(left), ContentField::Formatted(right)) => {
                    // try to hotreload this formatted string
                    _ = self.hotreload_ifmt(&left, &right);
                }
                _ => {
                    if old_attr != new_attr {
                        return None;
                    }
                }
            }
        }

        Some(())
    }

    /// Hot reload an if chain
    //
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
                self.hotreload_body::<Ctx>(&left, &right)?;
                break;
            }
        }

        Some(matches)
    }

    /// Take two dynamic contexts and return a mapping of dynamic attributes from the original to the new.
    ///
    /// IE if we shuffle attributes around we should be able to hot reload them.
    /// Same thing with dropping dynamic attributes.
    ///
    /// ```rust
    /// rsx! {
    ///     div { id: "{id}", class: "{class}", "Hi" }
    /// }
    ///
    /// rsx! {
    ///     div { class: "{class}", id: "{id}", "Hi" }
    /// }
    /// ```
    fn hotreload_attributes<Ctx: HotReloadingContext>(
        &mut self,
        old: &DynamicContext<'_>,
        new: &DynamicContext<'_>,
    ) -> Option<Vec<AttributePath>> {
        // Build a map of old attributes to their indexes
        // We can use the hash directly here but in theory we could support going from `class: "abc {def}"` to `class: "abc"`
        // This will require not running the format, but we basically need prop reloading to get that working
        //
        // Note that we might have duplicate attributes! We use a stack just to make sure we don't lose them
        let mut new_attrs = new
            .dynamic_attributes
            .iter()
            .map(|f| Some(f))
            .collect::<Vec<_>>();

        // Now we can run through the dynamic nodes and see if we can hot reload them
        let mut attr_paths = vec![];

        for old_attr in old.dynamic_attributes.iter() {
            for (new_idx, maybe_new_attr) in new_attrs.iter_mut().enumerate() {
                let Some(new_attr) = maybe_new_attr else {
                    continue;
                };

                if new_attr.as_slice() == old_attr.as_slice() {
                    // We found a match! Get this dynamic node's path and push it into the output
                    attr_paths.push(new.attr_paths[new_idx].clone());

                    // And then mark the original node as `None` so it's skipped on the next scan
                    _ = maybe_new_attr.take();

                    break;
                }
            }
        }

        // If there's any lingering new attrs, they can't be hot reloaded
        if new_attrs.iter().any(|n| n.is_some()) {
            return None;
        }

        Some(attr_paths)
    }

    pub fn callbody_to_template<Ctx: HotReloadingContext>(
        &mut self,
        callbody: &CallBody,
    ) -> Template {
        let ctx = DynamicContext::from_body::<Ctx>(&callbody.body.roots);

        // Rendering template nodes without a previous is just using ourselves as the previous mapping
        // Even though nothing changed, we can still use all the same rendering logic.
        let roots = self.render_dynamic_context::<Ctx>(
            &ctx,
            &callbody.body,
            &ctx.node_paths,
            &ctx.attr_paths,
        );

        Template {
            name: self.make_location(callbody.body.location.get()).leak(),
            roots: intern(roots.as_slice()),
            node_paths: intern(
                ctx.node_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
            attr_paths: intern(
                ctx.attr_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
        }
    }

    /// Use all the context we have to write out the template nodes
    ///
    /// This involves descendding throughout the bodynodes, hitting dynamic bits, and finding the corresponding
    /// nodes in the context.
    ///
    /// For dynamic attributes, we currently have a naive approach of just finding the old attribute in
    /// the old template. Eventually we might want to be more sophisticated about this to do things like
    /// hotreloading formatted segments.
    fn render_dynamic_context<Ctx: HotReloadingContext>(
        &mut self,
        ctx: &DynamicContext,
        new: &TemplateBody,
        node_paths: &[Vec<u8>],
        attr_paths: &[Vec<u8>],
    ) -> Vec<TemplateNode> {
        let mut nodes = Vec::new();

        for (idx, node) in new.roots.iter().enumerate() {
            nodes.push(self.render_template_node::<Ctx>(
                ctx,
                node,
                node_paths,
                attr_paths,
                vec![idx as u8],
            ));
        }

        nodes
    }

    fn render_template_node<Ctx: HotReloadingContext>(
        &mut self,
        ctx: &DynamicContext,
        node: &BodyNode,
        node_paths: &[Vec<u8>],
        attr_paths: &[Vec<u8>],
        cur_path: Vec<u8>,
    ) -> TemplateNode {
        match node {
            // The user is moving a static node around in the template
            BodyNode::Element(el) => {
                let rust_name = el.name.to_string();

                // Build an iterator that will yield attributes at the current path
                // This is mostly to preserve the order of the attributes
                // We will interleave these dynamic nodes into the merged attributes as we write those out
                // We could just dump all the dynamic attributes after the static ones, making this simpler,
                // but all our tests are designed to preserve the order of the attributes, so we'll do that
                //
                // The `rev` is just to match the old behavior of attributes being pushed and popped rather than
                // linear inserted.
                let mut attr_iter = attr_paths
                    .iter()
                    .enumerate()
                    .rev()
                    .filter(|(_idx, path)| *path == &cur_path)
                    .map(|(idx, _)| idx);

                // Write the attributes by interleaving static and dynamic
                let static_attr_array = el
                    .merged_attributes
                    .iter()
                    .map(|attr| match attr.as_static_str_literal() {
                        Some((name, value)) => {
                            make_static_attribute::<Ctx>(value, name, &rust_name)
                        }
                        None => TemplateAttribute::Dynamic {
                            id: attr_iter.next().expect("Attributes should be in order"),
                        },
                    })
                    .collect::<Vec<_>>();

                // Write out the children with the current path + the child index
                let children = el
                    .children
                    .iter()
                    .enumerate()
                    .map(|(idx, child)| {
                        let mut new_cur_path = cur_path.clone();
                        new_cur_path.push(idx as u8);
                        self.render_template_node::<Ctx>(
                            ctx,
                            child,
                            node_paths,
                            attr_paths,
                            new_cur_path.clone(),
                        )
                    })
                    .collect::<Vec<_>>();

                let (tag, namespace) =
                    Ctx::map_element(&rust_name).unwrap_or((intern(rust_name.as_str()), None));

                TemplateNode::Element {
                    tag,
                    namespace,
                    attrs: intern(static_attr_array.into_boxed_slice()),
                    children: intern(children.as_slice()),
                }
            }

            BodyNode::Text(text) if text.input.is_static() => {
                let text = text.input.source.as_ref().unwrap();
                let text = intern(text.value().as_str());
                TemplateNode::Text { text }
            }

            // Find the corresponding node in the node_paths map
            BodyNode::RawExpr(_)
            | BodyNode::Text(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::Component(_) => {
                // Just look for the current path in the node_paths map and thats our id
                let id = node_paths
                    .iter()
                    .position(|p| p == &cur_path)
                    .expect("Dynamic nodes to always be linked");

                match node {
                    BodyNode::Text(_) => TemplateNode::DynamicText { id },
                    _ => TemplateNode::Dynamic { id },
                }
            }
        }
    }
}

fn make_static_attribute<Ctx: HotReloadingContext>(
    value: &IfmtInput,
    name: &ElementAttrName,
    element_name_rust: &str,
) -> TemplateAttribute {
    let value = value.source.as_ref().unwrap();
    let attribute_name_rust = name.to_string();
    let (name, namespace) = Ctx::map_attribute(element_name_rust, &attribute_name_rust)
        .unwrap_or((intern(attribute_name_rust.as_str()), None));

    let static_attr = TemplateAttribute::Static {
        name,
        namespace,
        value: intern(value.value().as_str()),
    };

    static_attr
}
