//! A primitive diffing algorithm
//!
//!
//!
//!
//!

use std::{collections::HashMap, mem};

use crate::innerlude::*;
use crate::patch::Patch;
use fxhash::{FxBuildHasher, FxHashMap, FxHashSet};
use generational_arena::Index;

pub struct DiffMachine {
    immediate_queue: Vec<Index>,
    diffed: FxHashSet<Index>,
    need_to_diff: FxHashSet<Index>,
    marked_for_removal: Vec<Index>,
}

impl DiffMachine {
    pub fn new() -> Self {
        Self {
            immediate_queue: vec![],
            diffed: FxHashSet::default(),
            need_to_diff: FxHashSet::default(),
            marked_for_removal: vec![],
        }
    }

    /// Given two VirtualNode's generate Patch's that would turn the old virtual node's
    /// real DOM node equivalent into the new VirtualNode's real DOM node equivalent.
    pub fn diff<'a>(&mut self, old: &'a VNode, new: &'a VNode) -> Vec<Patch<'a>> {
        self.diff_recursive(&old, &new, &mut 0)
    }

    pub fn diff_recursive<'a, 'b>(
        &mut self,
        old: &'a VNode,
        new: &'a VNode,
        cur_node_idx: &'b mut usize,
    ) -> Vec<Patch<'a>> {
        let mut patches = vec![];
        let mut replace = false;

        // Different enum variants, replace!
        if mem::discriminant(old) != mem::discriminant(new) {
            replace = true;
        }

        if let (VNode::Element(old_element), VNode::Element(new_element)) = (old, new) {
            // Replace if there are different element tags
            if old_element.tag_name != new_element.tag_name {
                // if old_element.tag != new_element.tag {
                replace = true;
            }

            // Replace if two elements have different keys
            // TODO: More robust key support. This is just an early stopgap to allow you to force replace
            // an element... say if it's event changed. Just change the key name for now.
            // In the future we want keys to be used to create a Patch::ReOrder to re-order siblings
            // todo!
            // if old_element.attributes.get("key").is_some()
            //     && old_element.attrs.get("key") != new_element.attrs.get("key")
            // {
            //     replace = true;
            // }
        }

        // Handle replacing of a node
        if replace {
            patches.push(Patch::Replace(*cur_node_idx, &new));
            if let VNode::Element(old_element_node) = old {
                for child in old_element_node.children.iter() {
                    increment_node_idx_for_children(child, cur_node_idx);
                }
            }
            return patches;
        }

        // The following comparison can only contain identical variants, other
        // cases have already been handled above by comparing variant
        // discriminants.
        match (old, new) {
            // We're comparing two text nodes
            (VNode::Text(old_text), VNode::Text(new_text)) => {
                if old_text != new_text {
                    patches.push(Patch::ChangeText(*cur_node_idx, &new_text));
                }
            }

            // We're comparing two element nodes
            (VNode::Element(old_element), VNode::Element(new_element)) => {
                // let b: HashMap<&str, &str, FxBuildHasher>  = HashMap::new()
                let old_attrs = old_element
                    .attributes
                    .iter()
                    .map(|f| (f.name, f.value))
                    .collect::<HashMap<&'static str, &str, FxBuildHasher>>();

                let new_attrs = old_element
                    .attributes
                    .iter()
                    .map(|f| (f.name, f.value))
                    .collect::<HashMap<&'static str, &str, FxBuildHasher>>();

                let mut add_attributes = FxHashMap::<&'static str, &str>::default();
                // [("blah", "blah")]
                // .into_iter()
                // .map(|f| (f.0, f.1))
                // .collect::<HashMap<&'static str, &str, FxBuildHasher>>();

                // let mut add_attribute = HashMap::<&str, &str, FxBuildHasher>::new();
                let mut remove_attributes: Vec<&str> = vec![];

                // TODO: -> split out into func
                for (new_attr_name, new_attr_val) in new_attrs.iter() {
                    // for (new_attr_name, new_attr_val) in new_element.attrs.iter() {
                    match old_attrs.get(new_attr_name) {
                        // match old_element.attrs.get(new_attr_name) {
                        Some(ref old_attr_val) => {
                            if old_attr_val != &new_attr_val {
                                add_attributes.insert(new_attr_name, new_attr_val);
                            }
                        }
                        None => {
                            add_attributes.insert(new_attr_name, new_attr_val);
                        }
                    };
                }

                // TODO: -> split out into func
                for (old_attr_name, old_attr_val) in old_attrs.iter() {
                    // for (old_attr_name, old_attr_val) in old_element.attrs.iter() {
                    if add_attributes.get(&old_attr_name[..]).is_some() {
                        continue;
                    };

                    match new_attrs.get(old_attr_name) {
                        // match new_element.attrs.get(old_attr_name) {
                        Some(ref new_attr_val) => {
                            if new_attr_val != &old_attr_val {
                                remove_attributes.push(old_attr_name);
                            }
                        }
                        None => {
                            remove_attributes.push(old_attr_name);
                        }
                    };
                }

                if add_attributes.len() > 0 {
                    patches.push(Patch::AddAttributes(*cur_node_idx, add_attributes));
                }
                if remove_attributes.len() > 0 {
                    patches.push(Patch::RemoveAttributes(*cur_node_idx, remove_attributes));
                }

                let old_child_count = old_element.children.len();
                let new_child_count = new_element.children.len();

                if new_child_count > old_child_count {
                    let append_patch: Vec<&'a VNode> =
                        new_element.children[old_child_count..].iter().collect();
                    patches.push(Patch::AppendChildren(*cur_node_idx, append_patch))
                }

                if new_child_count < old_child_count {
                    patches.push(Patch::TruncateChildren(*cur_node_idx, new_child_count))
                }

                let min_count = std::cmp::min(old_child_count, new_child_count);
                for index in 0..min_count {
                    *cur_node_idx = *cur_node_idx + 1;
                    let old_child = &old_element.children[index];
                    let new_child = &new_element.children[index];
                    patches.append(&mut self.diff_recursive(&old_child, &new_child, cur_node_idx))
                }
                if new_child_count < old_child_count {
                    for child in old_element.children[min_count..].iter() {
                        increment_node_idx_for_children(child, cur_node_idx);
                    }
                }
            }

            (VNode::Suspended, _)
            | (_, VNode::Suspended)
            | (VNode::Component(_), _)
            | (_, VNode::Component(_)) => {
                todo!("cant yet handle these two")
            }

            (VNode::Text(_), VNode::Element(_))
            | (VirtualNode::Element(_), VirtualNode::Text(_)) => {
                unreachable!("Unequal variant discriminants should already have been handled");
            }
        };

        //    new_root.create_element()
        patches
    }
}

fn increment_node_idx_for_children<'a, 'b>(old: &'a VirtualNode, cur_node_idx: &'b mut usize) {
    *cur_node_idx += 1;
    if let VirtualNode::Element(element_node) = old {
        for child in element_node.children.iter() {
            increment_node_idx_for_children(&child, cur_node_idx);
        }
    }
}

// #[cfg(test)]
mod tests {
    use bumpalo::Bump;

    use super::*;

    fn test_diff(
        tree1: impl Fn(&Bump) -> VNode<'_>,
        tree2: impl Fn(&Bump) -> VNode<'_>,
        expected_patches: Vec<Patch>,
        description: &'static str,
    ) {
        let bump = Bump::new();

        let nodes1 = tree1(&bump);
        let nodes2 = tree1(&bump);

        let mut machine = DiffMachine::new();

        let patches = machine.diff(&nodes1, &nodes2);

        patches
            .iter()
            .zip(expected_patches.iter())
            .for_each(|f| assert_eq!(compare_patch(f.0, f.1), true, "{}", description));
    }

    fn compare_patch(patch1: &Patch, patch2: &Patch) -> bool {
        match (patch1, patch2) {
            (Patch::AppendChildren(_, _), Patch::AppendChildren(_, _)) => true,
            (Patch::AppendChildren(_, _), _) => false,

            (Patch::TruncateChildren(_, _), Patch::TruncateChildren(_, _)) => true,
            (Patch::TruncateChildren(_, _), _) => false,

            (Patch::Replace(_, _), Patch::Replace(_, _)) => true,
            (Patch::Replace(_, _), _) => false,

            (Patch::AddAttributes(_, _), Patch::AddAttributes(_, _)) => true,
            (Patch::AddAttributes(_, _), _) => false,

            (Patch::RemoveAttributes(_, _), Patch::RemoveAttributes(_, _)) => true,
            (Patch::RemoveAttributes(_, _), _) => false,

            (Patch::ChangeText(_, _), Patch::ChangeText(_, _)) => true,
            (Patch::ChangeText(_, _), _) => false,
        }
    }

    fn printdiff(
        tree1: impl for<'a> Fn(&'a Bump) -> VNode<'a>,
        tree2: impl for<'a> Fn(&'a Bump) -> VNode<'a>,
        desc: &'static str,
    ) {
        let bump = Bump::new();

        let nodes1 = tree1(&bump);
        let nodes2 = tree2(&bump);

        let mut machine = DiffMachine::new();

        let patches = machine.diff(&nodes1, &nodes2);

        patches.iter().for_each(|f| match f {
            Patch::AppendChildren(idx, a) => {
                println!("AppendChildren");
            }
            Patch::TruncateChildren(idx, a) => {
                println!("TruncateChildren");
            }
            Patch::Replace(idx, a) => {
                println!("Replace");
            }
            Patch::AddAttributes(idx, a) => {
                println!("AddAttributes");
            }
            Patch::RemoveAttributes(idx, a) => {
                println!("RemoveAttributes");
            }
            Patch::ChangeText(idx, a) => {
                println!("ChangeText");
            }
        });
    }

    #[test]
    fn example_diff() {
        printdiff(
            html! { <div> </div> },
            html! { <div>"Hello world!" </div> },
            "demo the difference between two simple dom tree",
        );

        printdiff(
            html! {
                <div>
                    "Hello world!"
                </div>
            },
            html! {
                <div>
                    <div>
                        "Hello world!"
                        "Hello world!"
                        "Hello world!"
                        "Hello world!"
                        "Hello world!"
                    </div>
                </div>
            },
            "demo the difference between two simple dom tree",
        );
    }
}
