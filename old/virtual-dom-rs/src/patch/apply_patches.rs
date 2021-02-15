use crate::dom_updater::ActiveClosures;
use crate::patch::Patch;

use std::cmp::min;
use std::collections::HashMap;
use std::collections::HashSet;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Element, Node, Text};

/// Apply all of the patches to our old root node in order to create the new root node
/// that we desire.
/// This is usually used after diffing two virtual nodes.
pub fn patch<N: Into<Node>>(root_node: N, patches: &Vec<Patch>) -> Result<ActiveClosures, JsValue> {
    let root_node: Node = root_node.into();

    let mut cur_node_idx = 0;

    let mut nodes_to_find = HashSet::new();

    for patch in patches {
        nodes_to_find.insert(patch.node_idx());
    }

    let mut element_nodes_to_patch = HashMap::new();
    let mut text_nodes_to_patch = HashMap::new();

    // Closures that were added to the DOM during this patch operation.
    let mut active_closures = HashMap::new();

    find_nodes(
        root_node,
        &mut cur_node_idx,
        &mut nodes_to_find,
        &mut element_nodes_to_patch,
        &mut text_nodes_to_patch,
    );

    for patch in patches {
        let patch_node_idx = patch.node_idx();

        if let Some(element) = element_nodes_to_patch.get(&patch_node_idx) {
            let new_closures = apply_element_patch(&element, &patch)?;
            active_closures.extend(new_closures);
            continue;
        }

        if let Some(text_node) = text_nodes_to_patch.get(&patch_node_idx) {
            apply_text_patch(&text_node, &patch)?;
            continue;
        }

        unreachable!("Getting here means we didn't find the element or next node that we were supposed to patch.")
    }

    Ok(active_closures)
}

fn find_nodes(
    root_node: Node,
    cur_node_idx: &mut usize,
    nodes_to_find: &mut HashSet<usize>,
    element_nodes_to_patch: &mut HashMap<usize, Element>,
    text_nodes_to_patch: &mut HashMap<usize, Text>,
) {
    if nodes_to_find.len() == 0 {
        return;
    }

    // We use child_nodes() instead of children() because children() ignores text nodes
    let children = root_node.child_nodes();
    let child_node_count = children.length();

    // If the root node matches, mark it for patching
    if nodes_to_find.get(&cur_node_idx).is_some() {
        match root_node.node_type() {
            Node::ELEMENT_NODE => {
                element_nodes_to_patch.insert(*cur_node_idx, root_node.unchecked_into());
            }
            Node::TEXT_NODE => {
                text_nodes_to_patch.insert(*cur_node_idx, root_node.unchecked_into());
            }
            other => unimplemented!("Unsupported root node type: {}", other),
        }
        nodes_to_find.remove(&cur_node_idx);
    }

    *cur_node_idx += 1;

    for i in 0..child_node_count {
        let node = children.item(i).unwrap();

        match node.node_type() {
            Node::ELEMENT_NODE => {
                find_nodes(
                    node,
                    cur_node_idx,
                    nodes_to_find,
                    element_nodes_to_patch,
                    text_nodes_to_patch,
                );
            }
            Node::TEXT_NODE => {
                if nodes_to_find.get(&cur_node_idx).is_some() {
                    text_nodes_to_patch.insert(*cur_node_idx, node.unchecked_into());
                }

                *cur_node_idx += 1;
            }
            Node::COMMENT_NODE => {
                // At this time we do not support user entered comment nodes, so if we see a comment
                // then it was a delimiter created by virtual-dom-rs in order to ensure that two
                // neighboring text nodes did not get merged into one by the browser. So we skip
                // over this virtual-dom-rs generated comment node.
            }
            _other => {
                // Ignoring unsupported child node type
                // TODO: What do we do with this situation? Log a warning?
            }
        }
    }
}

fn apply_element_patch(node: &Element, patch: &Patch) -> Result<ActiveClosures, JsValue> {
    let active_closures = HashMap::new();

    match patch {
        Patch::AddAttributes(_node_idx, attributes) => {
            for (attrib_name, attrib_val) in attributes.iter() {
                node.set_attribute(attrib_name, attrib_val)?;
            }

            Ok(active_closures)
        }
        Patch::RemoveAttributes(_node_idx, attributes) => {
            for attrib_name in attributes.iter() {
                node.remove_attribute(attrib_name)?;
            }

            Ok(active_closures)
        }
        Patch::Replace(_node_idx, new_node) => {
            let created_node = new_node.create_dom_node();

            node.replace_with_with_node_1(&created_node.node)?;

            Ok(created_node.closures)
        }
        Patch::TruncateChildren(_node_idx, num_children_remaining) => {
            let children = node.child_nodes();
            let mut child_count = children.length();

            // We skip over any separators that we placed between two text nodes
            //   -> `<!--ptns-->`
            //  and trim all children that come after our new desired `num_children_remaining`
            let mut non_separator_children_found = 0;

            for index in 0 as u32..child_count {
                let child = children
                    .get(min(index, child_count - 1))
                    .expect("Potential child to truncate");

                // If this is a comment node then we know that it is a `<!--ptns-->`
                // text node separator that was created in virtual_node/mod.rs.
                if child.node_type() == Node::COMMENT_NODE {
                    continue;
                }

                non_separator_children_found += 1;

                if non_separator_children_found <= *num_children_remaining as u32 {
                    continue;
                }

                node.remove_child(&child).expect("Truncated children");
                child_count -= 1;
            }

            Ok(active_closures)
        }
        Patch::AppendChildren(_node_idx, new_nodes) => {
            let parent = &node;

            let mut active_closures = HashMap::new();

            for new_node in new_nodes {
                let created_node = new_node.create_dom_node();

                parent.append_child(&created_node.node)?;

                active_closures.extend(created_node.closures);
            }

            Ok(active_closures)
        }
        Patch::ChangeText(_node_idx, _new_node) => {
            unreachable!("Elements should not receive ChangeText patches.")
        }
    }
}

fn apply_text_patch(node: &Text, patch: &Patch) -> Result<(), JsValue> {
    match patch {
        Patch::ChangeText(_node_idx, new_node) => {
            node.set_node_value(Some(&new_node.text));
        }
        Patch::Replace(_node_idx, new_node) => {
            node.replace_with_with_node_1(&new_node.create_dom_node().node)?;
        }
        other => unreachable!(
            "Text nodes should only receive ChangeText or Replace patches, not {:?}.",
            other,
        ),
    };

    Ok(())
}
