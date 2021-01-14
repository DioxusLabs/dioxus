use crate::Patch;
use crate::VirtualNode;
use std::cmp::min;
use std::collections::HashMap;
use std::mem;

/// Given two VirtualNode's generate Patch's that would turn the old virtual node's
/// real DOM node equivalent into the new VirtualNode's real DOM node equivalent.
pub fn diff<'a>(old: &'a VirtualNode, new: &'a VirtualNode) -> Vec<Patch<'a>> {
    diff_recursive(&old, &new, &mut 0)
}

fn diff_recursive<'a, 'b>(
    old: &'a VirtualNode,
    new: &'a VirtualNode,
    cur_node_idx: &'b mut usize,
) -> Vec<Patch<'a>> {
    let mut patches = vec![];
    let mut replace = false;

    // Different enum variants, replace!
    if mem::discriminant(old) != mem::discriminant(new) {
        replace = true;
    }

    if let (VirtualNode::Element(old_element), VirtualNode::Element(new_element)) = (old, new) {
        // Replace if there are different element tags
        if old_element.tag != new_element.tag {
            replace = true;
        }

        // Replace if two elements have different keys
        // TODO: More robust key support. This is just an early stopgap to allow you to force replace
        // an element... say if it's event changed. Just change the key name for now.
        // In the future we want keys to be used to create a Patch::ReOrder to re-order siblings
        if old_element.attrs.get("key").is_some()
            && old_element.attrs.get("key") != new_element.attrs.get("key")
        {
            replace = true;
        }
    }

    // Handle replacing of a node
    if replace {
        patches.push(Patch::Replace(*cur_node_idx, &new));
        if let VirtualNode::Element(old_element_node) = old {
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
        (VirtualNode::Text(old_text), VirtualNode::Text(new_text)) => {
            if old_text != new_text {
                patches.push(Patch::ChangeText(*cur_node_idx, &new_text));
            }
        }

        // We're comparing two element nodes
        (VirtualNode::Element(old_element), VirtualNode::Element(new_element)) => {
            let mut add_attributes: HashMap<&str, &str> = HashMap::new();
            let mut remove_attributes: Vec<&str> = vec![];

            // TODO: -> split out into func
            for (new_attr_name, new_attr_val) in new_element.attrs.iter() {
                match old_element.attrs.get(new_attr_name) {
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
            for (old_attr_name, old_attr_val) in old_element.attrs.iter() {
                if add_attributes.get(&old_attr_name[..]).is_some() {
                    continue;
                };

                match new_element.attrs.get(old_attr_name) {
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
                let append_patch: Vec<&'a VirtualNode> =
                    new_element.children[old_child_count..].iter().collect();
                patches.push(Patch::AppendChildren(*cur_node_idx, append_patch))
            }

            if new_child_count < old_child_count {
                patches.push(Patch::TruncateChildren(*cur_node_idx, new_child_count))
            }

            let min_count = min(old_child_count, new_child_count);
            for index in 0..min_count {
                *cur_node_idx = *cur_node_idx + 1;
                let old_child = &old_element.children[index];
                let new_child = &new_element.children[index];
                patches.append(&mut diff_recursive(&old_child, &new_child, cur_node_idx))
            }
            if new_child_count < old_child_count {
                for child in old_element.children[min_count..].iter() {
                    increment_node_idx_for_children(child, cur_node_idx);
                }
            }
        }
        (VirtualNode::Text(_), VirtualNode::Element(_))
        | (VirtualNode::Element(_), VirtualNode::Text(_)) => {
            unreachable!("Unequal variant discriminants should already have been handled");
        }
    };

    //    new_root.create_element()
    patches
}

fn increment_node_idx_for_children<'a, 'b>(old: &'a VirtualNode, cur_node_idx: &'b mut usize) {
    *cur_node_idx += 1;
    if let VirtualNode::Element(element_node) = old {
        for child in element_node.children.iter() {
            increment_node_idx_for_children(&child, cur_node_idx);
        }
    }
}

#[cfg(test)]
mod diff_test_case;
#[cfg(test)]
use self::diff_test_case::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{html, VText, VirtualNode};
    use std::collections::HashMap;

    #[test]
    fn replace_node() {
        DiffTestCase {
            description: "Replace the root if the tag changed",
            old: html! { <div> </div> },
            new: html! { <span> </span> },
            expected: vec![Patch::Replace(0, &html! { <span></span> })],
        }
        .test();
        DiffTestCase {
            description: "Replace a child node",
            old: html! { <div> <b></b> </div> },
            new: html! { <div> <strong></strong> </div> },
            expected: vec![Patch::Replace(1, &html! { <strong></strong> })],
        }
        .test();
        DiffTestCase {
            description: "Replace node with a child",
            old: html! { <div> <b>1</b> <b></b> </div> },
            new: html! { <div> <i>1</i> <i></i> </div>},
            expected: vec![
                Patch::Replace(1, &html! { <i>1</i> }),
                Patch::Replace(3, &html! { <i></i> }),
            ], //required to check correct index
        }
        .test();
    }

    #[test]
    fn add_children() {
        DiffTestCase {
            description: "Added a new node to the root node",
            old: html! { <div> <b></b> </div> },
            new: html! { <div> <b></b> <span></span> </div> },
            expected: vec![Patch::AppendChildren(0, vec![&html! { <span></span> }])],
        }
        .test();
    }

    #[test]
    fn remove_nodes() {
        DiffTestCase {
            description: "Remove all child nodes at and after child sibling index 1",
            old: html! { <div> <b></b> <span></span> </div> },
            new: html! { <div> </div> },
            expected: vec![Patch::TruncateChildren(0, 0)],
        }
        .test();
        DiffTestCase {
            description: "Remove a child and a grandchild node",
            old: html! {
            <div>
             <span>
               <b></b>
               // This `i` tag will get removed
               <i></i>
             </span>
             // This `strong` tag will get removed
             <strong></strong>
            </div> },
            new: html! {
            <div>
             <span>
              <b></b>
             </span>
            </div> },
            expected: vec![Patch::TruncateChildren(0, 1), Patch::TruncateChildren(1, 1)],
        }
        .test();
        DiffTestCase {
            description: "Removing child and change next node after parent",
            old: html! { <div> <b> <i></i> <i></i> </b> <b></b> </div> },
            new: html! { <div> <b> <i></i> </b> <i></i> </div>},
            expected: vec![
                Patch::TruncateChildren(1, 1),
                Patch::Replace(4, &html! { <i></i> }),
            ], //required to check correct index
        }
        .test();
    }

    #[test]
    fn add_attributes() {
        let mut attributes = HashMap::new();
        attributes.insert("id", "hello");

        DiffTestCase {
            old: html! { <div> </div> },
            new: html! { <div id="hello"> </div> },
            expected: vec![Patch::AddAttributes(0, attributes.clone())],
            description: "Add attributes",
        }
        .test();

        DiffTestCase {
            old: html! { <div id="foobar"> </div> },
            new: html! { <div id="hello"> </div> },
            expected: vec![Patch::AddAttributes(0, attributes)],
            description: "Change attribute",
        }
        .test();
    }

    #[test]
    fn remove_attributes() {
        DiffTestCase {
            old: html! { <div id="hey-there"></div> },
            new: html! { <div> </div> },
            expected: vec![Patch::RemoveAttributes(0, vec!["id"])],
            description: "Add attributes",
        }
        .test();
    }

    #[test]
    fn change_attribute() {
        let mut attributes = HashMap::new();
        attributes.insert("id", "changed");

        DiffTestCase {
            description: "Add attributes",
            old: html! { <div id="hey-there"></div> },
            new: html! { <div id="changed"> </div> },
            expected: vec![Patch::AddAttributes(0, attributes)],
        }
        .test();
    }

    #[test]
    fn replace_text_node() {
        DiffTestCase {
            description: "Replace text node",
            old: html! { Old },
            new: html! { New },
            expected: vec![Patch::ChangeText(0, &VText::new("New"))],
        }
        .test();
    }

    // Initially motivated by having two elements where all that changed was an event listener
    // because right now we don't patch event listeners. So.. until we have a solution
    // for that we can just give them different keys to force a replace.
    #[test]
    fn replace_if_different_keys() {
        DiffTestCase {
            description: "If two nodes have different keys always generate a full replace.",
            old: html! { <div key="1"> </div> },
            new: html! { <div key="2"> </div> },
            expected: vec![Patch::Replace(0, &html! {<div key="2"> </div>})],
        }
        .test()
    }

    //    // TODO: Key support
    //    #[test]
    //    fn reorder_chldren() {
    //        let mut attributes = HashMap::new();
    //        attributes.insert("class", "foo");
    //
    //        let old_children = vec![
    //            // old node 0
    //            html! { <div key="hello", id="same-id", style="",></div> },
    //            // removed
    //            html! { <div key="gets-removed",> { "This node gets removed"} </div>},
    //            // old node 2
    //            html! { <div key="world", class="changed-class",></div>},
    //            // removed
    //            html! { <div key="this-got-removed",> { "This node gets removed"} </div>},
    //        ];
    //
    //        let new_children = vec![
    //            html! { <div key="world", class="foo",></div> },
    //            html! { <div key="new",> </div>},
    //            html! { <div key="hello", id="same-id",></div>},
    //        ];
    //
    //        test(DiffTestCase {
    //            old: html! { <div> { old_children } </div> },
    //            new: html! { <div> { new_children } </div> },
    //            expected: vec![
    //                // TODO: Come up with the patch structure for keyed nodes..
    //                // keying should only work if all children have keys..
    //            ],
    //            description: "Add attributes",
    //        })
    //    }
}
