use std::collections::HashMap;

use dioxus_core::prelude::Template;

use crate::{intern, node};
use crate::{location, BodyNode, CallBody, DynamicContext, HotReloadingContext};

/// The mapping of a node relative to the root of its containing template
///
/// IE [0, 1] would be the location of the h3 node in this template:
/// ```rust
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
/// ```rust
/// rsx! {
///     div {
///         h1 { "title" }
///         h3 { class: "{class}", "Hi" }
///     }
/// }
/// ```
type AttributePath = Vec<u8>;

/// The `old` callbody is the original callbody that was used to create the template
///
/// The new one is the one that was used to update the template.
/// We never update the old in a hotreloading session since that's our only source of truth for dynamic
/// nodes, especially the ones actively living in the renderer. All new templates are simply a
/// transformation of the original. This lets us preserve dynamic nodes between hotreloads.
pub fn hotreload_callbody<Ctx: HotReloadingContext>(
    old: &CallBody,
    new: &CallBody,
    location: &'static str,
) -> Option<Vec<Template>> {
    let mut templates = vec![];

    hotreload_bodynodes::<Ctx>(&old.roots, &new.roots, &mut templates)?;

    Some(templates)
}

pub fn hotreload_bodynodes<Ctx: HotReloadingContext>(
    old: &[BodyNode],
    new: &[BodyNode],
    templates: &mut Vec<Template>,
) -> Option<()> {
    // Create a context that will be used to update the template
    let old_mapping = DynamicContext::from_body::<Ctx>(&old);
    let new_mapping = DynamicContext::from_body::<Ctx>(&new);

    hotreload_dynamic_context::<Ctx>(&old_mapping, &new_mapping, templates)?;

    Some(())
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
pub fn hotreload_dynamic_context<Ctx: HotReloadingContext>(
    old: &DynamicContext<'_>,
    new: &DynamicContext<'_>,
    templates: &mut Vec<Template>,
) -> Option<()> {
    // If the old dynamic context is the same as the new one, we can just return since there's nothing to do
    // This is expensive, we should maybe not do this - we're checking literally everything,
    // and this scales to number the depth of the tree
    // Maybe we can run the hotreload on each of the pieces first and then do a check on the outputs
    // todo: make this faster and/or change the behavior since we're relying on partialeq and that
    // might not be reliable with macros
    if old == new {
        return Some(());
    }

    // Quickly run through dynamic attributes first attempting to invalidate them
    let new_attribute_paths = hotreload_attributes::<Ctx>(old, new)?;

    // Now we can run through the dynamic nodes and see if we can hot reload them
    let new_node_paths = hotreload_dynamic_nodes::<Ctx>(old, new, templates)?;

    // Todo: actually create the templatenodes, should be available from the DynamicContext
    let roots = vec![];

    let location = "asdasd";

    // Now we can assemble a template
    templates.push(Template {
        name: location,
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

/// Take two dynamic contexts and return a mapping of dynamic attributes from the original to the new.
///
/// IE if we shuffle attributes around we should be able to
///
/// ```rust
/// rsx! {
///     div { id: "{id}", "Hi" }
/// }
///
///
///
/// ```
fn hotreload_attributes<Ctx: HotReloadingContext>(
    old: &DynamicContext<'_>,
    new: &DynamicContext<'_>,
) -> Option<Vec<AttributePath>> {
    // Build a map of old attributes to their indexes
    // We can use the hash directly here but in theory we could support going from `class: "abc {def}"` to `class: "abc"`
    // This will require not running the format, but we basically need prop reloading to get that working
    //
    // Note that we might have duplicate attributes! We use a stack just to make sure we don't lose them
    // let mut old_attr_map = HashMap::new();
    // for (idx, old_attrs) in old.dynamic_attributes.iter().enumerate() {
    //     for attr in old_attrs {
    //         old_attr_map.entry(attr).or_insert_with(Vec::new).push(idx);
    //     }
    // }

    Some(vec![])
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
    old: &DynamicContext<'_>,
    new: &DynamicContext<'_>,
    templates: &mut Vec<Template>,
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
    'outer: for old_node in old.dynamic_nodes.iter() {
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
                (BodyNode::Text(a), BodyNode::Text(b)) => a == b,

                // Nothing special for raw expressions - if an expresison changed we couldn't find it anyway
                (BodyNode::RawExpr(a), BodyNode::RawExpr(b)) => a == b,

                (BodyNode::Component(a), BodyNode::Component(b)) => a == b,
                (BodyNode::IfChain(a), BodyNode::IfChain(b)) => a == b,

                // If we found a matching forloop, its body might not be the same
                // the bodies don't need to be the same but the pats/exprs do
                (BodyNode::ForLoop(a), BodyNode::ForLoop(b)) => {
                    let matches = a.pat == b.pat && a.expr == b.expr;

                    // While we're here, lets try and reload its template contents too, if it's reloadable
                    // Note that there might be duplicates of the expressions.
                    //
                    // Currently if you shuffle for loops around that have the same expression, it will
                    // likely not be reloaded. This is because we don't know if the order of the expressions
                    // is the same or not. We should try to do a more complex analysis of the contents
                    // of the for loops to see if they're the same, but for now, we don't
                    //
                    // We need to use the old location info to find the new location info
                    //
                    // This is just the file+line+col+byte index from the original
                    if matches {
                        hotreload_bodynodes::<Ctx>(&a.body, &b.body, templates)?;
                    }

                    matches
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
