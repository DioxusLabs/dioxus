use crate::innerlude::*;

pub struct ElementIdIterator<'a> {
    vdom: &'a VirtualDom,

    // Heuristically we should never bleed into 5 completely nested fragments/components
    // Smallvec lets us stack allocate our little stack machine so the vast majority of cases are sane
    stack: smallvec::SmallVec<[(u16, &'a VNode<'a>); 5]>,
}

impl<'a> ElementIdIterator<'a> {
    pub fn new(vdom: &'a VirtualDom, node: &'a VNode<'a>) -> Self {
        Self { vdom, stack: smallvec::smallvec![(0, node)] }
    }
}

impl<'a> Iterator for ElementIdIterator<'a> {
    type Item = &'a VNode<'a>;

    fn next(&mut self) -> Option<&'a VNode<'a>> {
        let mut should_pop = false;
        let mut returned_node = None;
        let mut should_push = None;

        while returned_node.is_none() {
            if let Some((count, node)) = self.stack.last_mut() {
                match node {
                    // We can only exit our looping when we get "real" nodes
                    VNode::Element(_) | VNode::Text(_) | VNode::Placeholder(_) => {
                        // We've recursed INTO an element/text
                        // We need to recurse *out* of it and move forward to the next
                        // println!("Found element! Returning it!");
                        should_pop = true;
                        returned_node = Some(&**node);
                    }

                    // If we get a fragment we push the next child
                    VNode::Fragment(frag) => {
                        let _count = *count as usize;
                        if _count >= frag.children.len() {
                            should_pop = true;
                        } else {
                            should_push = Some(&frag.children[_count]);
                        }
                    }

                    // For components, we load their root and push them onto the stack
                    VNode::Component(sc) => {
                        let scope = self.vdom.get_scope(sc.scope.get().unwrap()).unwrap();
                        // Simply swap the current node on the stack with the root of the component
                        *node = scope.root_node();
                    }
                }
            } else {
                // If there's no more items on the stack, we're done!
                return None;
            }

            if should_pop {
                self.stack.pop();
                if let Some((id, _)) = self.stack.last_mut() {
                    *id += 1;
                }
                should_pop = false;
            }

            if let Some(push) = should_push {
                self.stack.push((0, push));
                should_push = None;
            }
        }

        returned_node
    }
}
