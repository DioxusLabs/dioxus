use std::any::Any;

use crate::factory::RenderReturn;
use crate::innerlude::Mutations;
use crate::virtual_dom::VirtualDom;
use crate::{Attribute, AttributeValue, TemplateNode};

use crate::any_props::VComponentProps;

use crate::mutations::Mutation;
use crate::nodes::{DynamicNode, Template, TemplateId};
use crate::scopes::Scope;
use crate::{
    any_props::AnyProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    nodes::VNode,
    scopes::{ScopeId, ScopeState},
};
use fxhash::{FxHashMap, FxHashSet};
use slab::Slab;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DirtyScope {
    pub height: u32,
    pub id: ScopeId,
}

impl PartialOrd for DirtyScope {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.height.cmp(&other.height))
    }
}

impl Ord for DirtyScope {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.height.cmp(&other.height)
    }
}

impl<'b> VirtualDom {
    pub fn diff_scope(&mut self, mutations: &mut Mutations<'b>, scope: ScopeId) {
        let scope_state = &mut self.scopes[scope.0];

        let cur_arena = scope_state.current_frame();
        let prev_arena = scope_state.previous_frame();

        // relax the borrow checker
        let cur_arena: &BumpFrame = unsafe { std::mem::transmute(cur_arena) };
        let prev_arena: &BumpFrame = unsafe { std::mem::transmute(prev_arena) };

        // Make sure the nodes arent null (they've been set properly)
        assert_ne!(
            cur_arena.node.get(),
            std::ptr::null_mut(),
            "Call rebuild before diffing"
        );
        assert_ne!(
            prev_arena.node.get(),
            std::ptr::null_mut(),
            "Call rebuild before diffing"
        );

        self.scope_stack.push(scope);
        let left = unsafe { prev_arena.load_node() };
        let right = unsafe { cur_arena.load_node() };
        self.diff_maybe_node(mutations, left, right);
        self.scope_stack.pop();
    }

    fn diff_maybe_node(
        &mut self,
        m: &mut Mutations<'b>,
        left: &'b RenderReturn<'b>,
        right: &'b RenderReturn<'b>,
    ) {
        use RenderReturn::{Async, Sync};
        match (left, right) {
            // diff
            (Sync(Some(l)), Sync(Some(r))) => self.diff_node(m, l, r),

            // remove old with placeholder
            (Sync(Some(l)), Sync(None)) | (Sync(Some(l)), Async(_)) => {
                //
                let id = self.next_element(l); // todo!
                m.push(Mutation::CreatePlaceholder { id });
                self.drop_template(m, l, true);
            }

            // remove placeholder with nodes
            (Sync(None), Sync(Some(_))) => {}
            (Async(_), Sync(Some(v))) => {}

            // nothing...
            (Async(_), Async(_))
            | (Sync(None), Sync(None))
            | (Sync(None), Async(_))
            | (Async(_), Sync(None)) => {}
        }
    }

    pub fn diff_node(
        &mut self,
        muts: &mut Mutations<'b>,
        left_template: &'b VNode<'b>,
        right_template: &'b VNode<'b>,
    ) {
        if left_template.template.id != right_template.template.id {
            // do a light diff of the roots nodes.
            return;
        }

        for (_idx, (left_attr, right_attr)) in left_template
            .dynamic_attrs
            .iter()
            .zip(right_template.dynamic_attrs.iter())
            .enumerate()
        {
            debug_assert!(left_attr.name == right_attr.name);
            debug_assert!(left_attr.value == right_attr.value);

            // Move over the ID from the old to the new
            right_attr
                .mounted_element
                .set(left_attr.mounted_element.get());

            if left_attr.value != right_attr.value {
                let value = "todo!()";
                muts.push(Mutation::SetAttribute {
                    id: left_attr.mounted_element.get(),
                    name: left_attr.name,
                    value,
                });
            }
        }

        for (idx, (left_node, right_node)) in left_template
            .dynamic_nodes
            .iter()
            .zip(right_template.dynamic_nodes.iter())
            .enumerate()
        {
            #[rustfmt::skip]
            match (left_node, right_node) {
                (DynamicNode::Component { props: lprops, .. }, DynamicNode::Component {  static_props: is_static , props: rprops, .. }) => {
                    let left_props = unsafe { &mut *lprops.get()};
                    let right_props = unsafe { &mut *rprops.get()};

                    // Ensure these two props are of the same component type
                    match left_props.as_ptr() == right_props.as_ptr()  {
                        true => {
                            //

                            if *is_static {
                                let props_are_same = unsafe { left_props.memoize(right_props)  };

                                if props_are_same{
                                    //
                                } else {
                                    //
                                }
                            } else {

                            }

                        },
                        false => todo!(),
                    }
                    //
                },

                // Make sure to drop the component properly
                (DynamicNode::Component { .. }, right) => {
                    // remove all the component roots except for the first
                    // replace the first with the new node
                    let m = self.create_dynamic_node(muts, right_template, right, idx);
                    todo!()
                },

                (DynamicNode::Text { id: lid, value: lvalue }, DynamicNode::Text { id: rid, value: rvalue }) => {
                    rid.set(lid.get());
                    if lvalue != rvalue {
                        muts.push(Mutation::SetText {
                            id: lid.get(),
                            value: rvalue,
                        });
                    }
                },

                (DynamicNode::Text { id: lid, .. }, right) => {
                    let m = self.create_dynamic_node(muts, right_template, right, idx);
                    muts.push(Mutation::Replace { id: lid.get(), m });
                }

                (DynamicNode::Placeholder(_), DynamicNode::Placeholder(_)) => todo!(),
                (DynamicNode::Placeholder(_), _) => todo!(),


                (DynamicNode::Fragment (l), DynamicNode::Fragment (r)) => {


                    // match (old, new) {
                    //     ([], []) => rp.set(lp.get()),
                    //     ([], _) => {
                    //         //
                    //         todo!()
                    //     },
                    //     (_, []) => {
                    //         todo!()
                    //     },
                    //     _ => {
                    //         let new_is_keyed = new[0].key.is_some();
                    //         let old_is_keyed = old[0].key.is_some();

                    //         debug_assert!(
                    //             new.iter().all(|n| n.key.is_some() == new_is_keyed),
                    //             "all siblings must be keyed or all siblings must be non-keyed"
                    //         );
                    //         debug_assert!(
                    //             old.iter().all(|o| o.key.is_some() == old_is_keyed),
                    //             "all siblings must be keyed or all siblings must be non-keyed"
                    //         );

                    //         if new_is_keyed && old_is_keyed {
                    //             self.diff_keyed_children(muts, old, new);
                    //         } else {
                    //             self.diff_non_keyed_children(muts, old, new);
                    //         }
                    //     }
                    // }
                },

                // Make sure to drop all the fragment children properly
                (DynamicNode::Fragment { .. }, right) => todo!(),
            };
        }
    }

    // Diff children that are not keyed.
    //
    // The parent must be on the top of the change list stack when entering this
    // function:
    //
    //     [... parent]
    //
    // the change list stack is in the same state when this function returns.
    fn diff_non_keyed_children(
        &mut self,
        muts: &mut Mutations<'b>,
        old: &'b [VNode<'b>],
        new: &'b [VNode<'b>],
    ) {
        use std::cmp::Ordering;

        // Handled these cases in `diff_children` before calling this function.
        debug_assert!(!new.is_empty());
        debug_assert!(!old.is_empty());

        match old.len().cmp(&new.len()) {
            Ordering::Greater => self.remove_nodes(muts, &old[new.len()..]),
            Ordering::Less => todo!(),
            // Ordering::Less => self.create_and_insert_after(&new[old.len()..], old.last().unwrap()),
            Ordering::Equal => {}
        }

        for (new, old) in new.iter().zip(old.iter()) {
            self.diff_node(muts, old, new);
        }
    }

    // Diffing "keyed" children.
    //
    // With keyed children, we care about whether we delete, move, or create nodes
    // versus mutate existing nodes in place. Presumably there is some sort of CSS
    // transition animation that makes the virtual DOM diffing algorithm
    // observable. By specifying keys for nodes, we know which virtual DOM nodes
    // must reuse (or not reuse) the same physical DOM nodes.
    //
    // This is loosely based on Inferno's keyed patching implementation. However, we
    // have to modify the algorithm since we are compiling the diff down into change
    // list instructions that will be executed later, rather than applying the
    // changes to the DOM directly as we compare virtual DOMs.
    //
    // https://github.com/infernojs/inferno/blob/36fd96/packages/inferno/src/DOM/patching.ts#L530-L739
    //
    // The stack is empty upon entry.
    fn diff_keyed_children(
        &mut self,
        muts: &mut Mutations<'b>,
        old: &'b [VNode<'b>],
        new: &'b [VNode<'b>],
    ) {
        // if cfg!(debug_assertions) {
        //     let mut keys = fxhash::FxHashSet::default();
        //     let mut assert_unique_keys = |children: &'b [VNode<'b>]| {
        //         keys.clear();
        //         for child in children {
        //             let key = child.key;
        //             debug_assert!(
        //                 key.is_some(),
        //                 "if any sibling is keyed, all siblings must be keyed"
        //             );
        //             keys.insert(key);
        //         }
        //         debug_assert_eq!(
        //             children.len(),
        //             keys.len(),
        //             "keyed siblings must each have a unique key"
        //         );
        //     };
        //     assert_unique_keys(old);
        //     assert_unique_keys(new);
        // }

        // // First up, we diff all the nodes with the same key at the beginning of the
        // // children.
        // //
        // // `shared_prefix_count` is the count of how many nodes at the start of
        // // `new` and `old` share the same keys.
        // let (left_offset, right_offset) = match self.diff_keyed_ends(muts, old, new) {
        //     Some(count) => count,
        //     None => return,
        // };

        // // Ok, we now hopefully have a smaller range of children in the middle
        // // within which to re-order nodes with the same keys, remove old nodes with
        // // now-unused keys, and create new nodes with fresh keys.

        // let old_middle = &old[left_offset..(old.len() - right_offset)];
        // let new_middle = &new[left_offset..(new.len() - right_offset)];

        // debug_assert!(
        //     !((old_middle.len() == new_middle.len()) && old_middle.is_empty()),
        //     "keyed children must have the same number of children"
        // );

        // if new_middle.is_empty() {
        //     // remove the old elements
        //     self.remove_nodes(muts, old_middle);
        // } else if old_middle.is_empty() {
        //     // there were no old elements, so just create the new elements
        //     // we need to find the right "foothold" though - we shouldn't use the "append" at all
        //     if left_offset == 0 {
        //         // insert at the beginning of the old list
        //         let foothold = &old[old.len() - right_offset];
        //         self.create_and_insert_before(new_middle, foothold);
        //     } else if right_offset == 0 {
        //         // insert at the end  the old list
        //         let foothold = old.last().unwrap();
        //         self.create_and_insert_after(new_middle, foothold);
        //     } else {
        //         // inserting in the middle
        //         let foothold = &old[left_offset - 1];
        //         self.create_and_insert_after(new_middle, foothold);
        //     }
        // } else {
        //     self.diff_keyed_middle(muts, old_middle, new_middle);
        // }
    }

    // /// Diff both ends of the children that share keys.
    // ///
    // /// Returns a left offset and right offset of that indicates a smaller section to pass onto the middle diffing.
    // ///
    // /// If there is no offset, then this function returns None and the diffing is complete.
    // fn diff_keyed_ends(
    //     &mut self,
    //     muts: &mut Renderer<'b>,
    //     old: &'b [VNode<'b>],
    //     new: &'b [VNode<'b>],
    // ) -> Option<(usize, usize)> {
    //     let mut left_offset = 0;

    //     for (old, new) in old.iter().zip(new.iter()) {
    //         // abort early if we finally run into nodes with different keys
    //         if old.key != new.key {
    //             break;
    //         }
    //         self.diff_node(muts, old, new);
    //         left_offset += 1;
    //     }

    //     // If that was all of the old children, then create and append the remaining
    //     // new children and we're finished.
    //     if left_offset == old.len() {
    //         self.create_and_insert_after(&new[left_offset..], old.last().unwrap());
    //         return None;
    //     }

    //     // And if that was all of the new children, then remove all of the remaining
    //     // old children and we're finished.
    //     if left_offset == new.len() {
    //         self.remove_nodes(muts, &old[left_offset..]);
    //         return None;
    //     }

    //     // if the shared prefix is less than either length, then we need to walk backwards
    //     let mut right_offset = 0;
    //     for (old, new) in old.iter().rev().zip(new.iter().rev()) {
    //         // abort early if we finally run into nodes with different keys
    //         if old.key != new.key {
    //             break;
    //         }
    //         self.diff_node(muts, old, new);
    //         right_offset += 1;
    //     }

    //     Some((left_offset, right_offset))
    // }

    // // The most-general, expensive code path for keyed children diffing.
    // //
    // // We find the longest subsequence within `old` of children that are relatively
    // // ordered the same way in `new` (via finding a longest-increasing-subsequence
    // // of the old child's index within `new`). The children that are elements of
    // // this subsequence will remain in place, minimizing the number of DOM moves we
    // // will have to do.
    // //
    // // Upon entry to this function, the change list stack must be empty.
    // //
    // // This function will load the appropriate nodes onto the stack and do diffing in place.
    // //
    // // Upon exit from this function, it will be restored to that same self.
    // #[allow(clippy::too_many_lines)]
    // fn diff_keyed_middle(
    //     &mut self,
    //     muts: &mut Renderer<'b>,
    //     old: &'b [VNode<'b>],
    //     new: &'b [VNode<'b>],
    // ) {
    //     /*
    //     1. Map the old keys into a numerical ordering based on indices.
    //     2. Create a map of old key to its index
    //     3. Map each new key to the old key, carrying over the old index.
    //         - IE if we have ABCD becomes BACD, our sequence would be 1,0,2,3
    //         - if we have ABCD to ABDE, our sequence would be 0,1,3,MAX because E doesn't exist

    //     now, we should have a list of integers that indicates where in the old list the new items map to.

    //     4. Compute the LIS of this list
    //         - this indicates the longest list of new children that won't need to be moved.

    //     5. Identify which nodes need to be removed
    //     6. Identify which nodes will need to be diffed

    //     7. Going along each item in the new list, create it and insert it before the next closest item in the LIS.
    //         - if the item already existed, just move it to the right place.

    //     8. Finally, generate instructions to remove any old children.
    //     9. Generate instructions to finally diff children that are the same between both
    //     */
    //     // 0. Debug sanity checks
    //     // Should have already diffed the shared-key prefixes and suffixes.
    //     debug_assert_ne!(new.first().map(|i| i.key), old.first().map(|i| i.key));
    //     debug_assert_ne!(new.last().map(|i| i.key), old.last().map(|i| i.key));

    //     // 1. Map the old keys into a numerical ordering based on indices.
    //     // 2. Create a map of old key to its index
    //     // IE if the keys were A B C, then we would have (A, 1) (B, 2) (C, 3).
    //     let old_key_to_old_index = old
    //         .iter()
    //         .enumerate()
    //         .map(|(i, o)| (o.key.unwrap(), i))
    //         .collect::<FxHashMap<_, _>>();

    //     let mut shared_keys = FxHashSet::default();

    //     // 3. Map each new key to the old key, carrying over the old index.
    //     let new_index_to_old_index = new
    //         .iter()
    //         .map(|node| {
    //             let key = node.key.unwrap();
    //             if let Some(&index) = old_key_to_old_index.get(&key) {
    //                 shared_keys.insert(key);
    //                 index
    //             } else {
    //                 u32::MAX as usize
    //             }
    //         })
    //         .collect::<Vec<_>>();

    //     // If none of the old keys are reused by the new children, then we remove all the remaining old children and
    //     // create the new children afresh.
    //     if shared_keys.is_empty() {
    //         if let Some(first_old) = old.get(0) {
    //             self.remove_nodes(muts, &old[1..]);
    //             let nodes_created = self.create_children(new);
    //             self.replace_inner(first_old, nodes_created);
    //         } else {
    //             // I think this is wrong - why are we appending?
    //             // only valid of the if there are no trailing elements
    //             self.create_and_append_children(new);
    //         }
    //         return;
    //     }

    //     // remove any old children that are not shared
    //     // todo: make this an iterator
    //     for child in old {
    //         let key = child.key.unwrap();
    //         if !shared_keys.contains(&key) {
    //             todo!("remove node");
    //             // self.remove_nodes(muts, [child]);
    //         }
    //     }

    //     // 4. Compute the LIS of this list
    //     let mut lis_sequence = Vec::default();
    //     lis_sequence.reserve(new_index_to_old_index.len());

    //     let mut predecessors = vec![0; new_index_to_old_index.len()];
    //     let mut starts = vec![0; new_index_to_old_index.len()];

    //     longest_increasing_subsequence::lis_with(
    //         &new_index_to_old_index,
    //         &mut lis_sequence,
    //         |a, b| a < b,
    //         &mut predecessors,
    //         &mut starts,
    //     );

    //     // the lis comes out backwards, I think. can't quite tell.
    //     lis_sequence.sort_unstable();

    //     // if a new node gets u32 max and is at the end, then it might be part of our LIS (because u32 max is a valid LIS)
    //     if lis_sequence.last().map(|f| new_index_to_old_index[*f]) == Some(u32::MAX as usize) {
    //         lis_sequence.pop();
    //     }

    //     for idx in &lis_sequence {
    //         self.diff_node(muts, &old[new_index_to_old_index[*idx]], &new[*idx]);
    //     }

    //     let mut nodes_created = 0;

    //     // add mount instruction for the first items not covered by the lis
    //     let last = *lis_sequence.last().unwrap();
    //     if last < (new.len() - 1) {
    //         for (idx, new_node) in new[(last + 1)..].iter().enumerate() {
    //             let new_idx = idx + last + 1;
    //             let old_index = new_index_to_old_index[new_idx];
    //             if old_index == u32::MAX as usize {
    //                 nodes_created += self.create(muts, new_node);
    //             } else {
    //                 self.diff_node(muts, &old[old_index], new_node);
    //                 nodes_created += self.push_all_real_nodes(new_node);
    //             }
    //         }

    //         self.mutations.insert_after(
    //             self.find_last_element(&new[last]).unwrap(),
    //             nodes_created as u32,
    //         );
    //         nodes_created = 0;
    //     }

    //     // for each spacing, generate a mount instruction
    //     let mut lis_iter = lis_sequence.iter().rev();
    //     let mut last = *lis_iter.next().unwrap();
    //     for next in lis_iter {
    //         if last - next > 1 {
    //             for (idx, new_node) in new[(next + 1)..last].iter().enumerate() {
    //                 let new_idx = idx + next + 1;
    //                 let old_index = new_index_to_old_index[new_idx];
    //                 if old_index == u32::MAX as usize {
    //                     nodes_created += self.create(muts, new_node);
    //                 } else {
    //                     self.diff_node(muts, &old[old_index], new_node);
    //                     nodes_created += self.push_all_real_nodes(new_node);
    //                 }
    //             }

    //             self.mutations.insert_before(
    //                 self.find_first_element(&new[last]).unwrap(),
    //                 nodes_created as u32,
    //             );

    //             nodes_created = 0;
    //         }
    //         last = *next;
    //     }

    //     // add mount instruction for the last items not covered by the lis
    //     let first_lis = *lis_sequence.first().unwrap();
    //     if first_lis > 0 {
    //         for (idx, new_node) in new[..first_lis].iter().enumerate() {
    //             let old_index = new_index_to_old_index[idx];
    //             if old_index == u32::MAX as usize {
    //                 nodes_created += self.create_node(new_node);
    //             } else {
    //                 self.diff_node(muts, &old[old_index], new_node);
    //                 nodes_created += self.push_all_real_nodes(new_node);
    //             }
    //         }

    //         self.mutations.insert_before(
    //             self.find_first_element(&new[first_lis]).unwrap(),
    //             nodes_created as u32,
    //         );
    //     }
    // }

    /// Remove these nodes from the dom
    /// Wont generate mutations for the inner nodes
    fn remove_nodes(&mut self, muts: &mut Mutations<'b>, nodes: &'b [VNode<'b>]) {
        //
    }
}

// /// Lightly diff the two templates and apply their edits to the dom
// fn light_diff_template_roots(
//     &'a mut self,
//     mutations: &mut Vec<Mutation<'a>>,
//     left: &VNode,
//     right: &VNode,
// ) {
//     match right.template.roots.len().cmp(&left.template.roots.len()) {
//         std::cmp::Ordering::Less => {
//             // remove the old nodes at the end
//         }
//         std::cmp::Ordering::Greater => {
//             // add the extra nodes.
//         }
//         std::cmp::Ordering::Equal => {}
//     }

//     for (left_node, right_node) in left.template.roots.iter().zip(right.template.roots.iter()) {
//         if let (TemplateNode::Dynamic(lidx), TemplateNode::Dynamic(ridx)) =
//             (left_node, right_node)
//         {
//             let left_node = &left.dynamic_nodes[*lidx];
//             let right_node = &right.dynamic_nodes[*ridx];

//             // match (left_node, right_node) {
//             //     (
//             //         DynamicNode::Component {
//             //             name,
//             //             can_memoize,
//             //             props,
//             //         },
//             //         DynamicNode::Component {
//             //             name,
//             //             can_memoize,
//             //             props,
//             //         },
//             //     ) => todo!(),
//             //     (
//             //         DynamicNode::Component {
//             //             name,
//             //             can_memoize,
//             //             props,
//             //         },
//             //         DynamicNode::Fragment { children },
//             //     ) => todo!(),
//             //     (
//             //         DynamicNode::Fragment { children },
//             //         DynamicNode::Component {
//             //             name,
//             //             can_memoize,
//             //             props,
//             //         },
//             //     ) => todo!(),
//             //     _ => {}
//             // }
//         }
//     }
// }
