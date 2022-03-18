// use dioxus_core::{Element, ElementId, Mutations, VNode, VirtualDom, DomEdit};

// /// The focus system needs a iterator that can persist through changes in the [VirtualDom]. Iterate through it with [ElementIter::next], and update it with [ElementIter::update] (with data from [`VirtualDom::work_with_deadline`]).
// pub(crate) struct ElementIter {
//     // stack of elements and fragments
//     stack: smallvec::SmallVec<[(ElementId, usize); 5]>,
// }

// impl ElementIter {
//     pub(crate) fn new(initial: ElementId) -> Self {
//         ElementIter {
//             stack: smallvec::smallvec![(initial, 0)],
//         }
//     }
//     /// remove stale element refreneces
//     pub(crate) fn update(&mut self, mutations: &Mutations, vdom: &VirtualDom) {
//         let ids_removed: Vec<_> = mutations.edits.iter().filter_map(|e| if let DomEdit::Remove{root: }).collect();
//         for node in self.stack {

//             match node.0 {
//                 VNode::Fragment(f) => {

//                 }

//                 VNode::Element(_) => {}

//                 _ => unreachable!(),
//             }
//         }
//     }
//     pub(crate) fn next<'a>(&mut self, vdom: &'a VirtualDom) -> Option<&'a VNode<'a>> {
//         let last = self.stack.last()?.0;
//         let node = vdom.get_element(last)?;
//         match node {
//             VNode::Fragment(f) => {
//                 let mut last_mut = self.stack.last_mut()?;
//                 if last_mut.1 + 1 >= f.children.len() {
//                     self.stack.pop();
//                     self.next(vdom)
//                 } else {
//                     last_mut.1 += 1;
//                     let new_node = &f.children[last_mut.1];
//                     if matches!(new_node, VNode::Fragment(_) | VNode::Element(_)) {
//                         self.stack.push((new_node.mounted_id(), 0));
//                     }
//                     self.next(vdom)
//                 }
//             }

//             VNode::Component(vcomp) => {
//                 let idx = vcomp.scope.get().unwrap();
//                 let new_node = vdom.get_scope(idx).unwrap().root_node();
//                 *self.stack.last_mut()? = (new_node.mounted_id(), 0);
//                 self.next(vdom)
//             }

//             VNode::Placeholder(_) | VNode::Text(_) => {
//                 self.stack.pop();
//                 self.next(vdom)
//             }

//             VNode::Element(e) => {
//                 let mut last_mut = self.stack.last_mut()?;
//                 if last_mut.1 + 1 >= e.children.len() {
//                     self.stack.pop();
//                     self.next(vdom);
//                 } else {
//                     last_mut.1 += 1;
//                     let new_node = &e.children[last_mut.1];
//                     if matches!(new_node, VNode::Fragment(_) | VNode::Element(_)) {
//                         self.stack.push((new_node.mounted_id(), 0));
//                     }
//                     self.next(vdom);
//                 }
//                 Some(node)
//             }
//         }
//     }

//     pub(crate) fn peak(&self) -> Option<&ElementId> {
//         self.stack.last().map(|(e, c)| e)
//     }
// }
