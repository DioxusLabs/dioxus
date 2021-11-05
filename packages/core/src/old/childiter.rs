use crate::innerlude::*;

/// This iterator iterates through a list of virtual children and only returns real children (Elements, Text, Anchors).
///
/// This iterator is useful when it's important to load the next real root onto the top of the stack for operations like
/// "InsertBefore".
pub(crate) struct RealChildIterator<'a> {
    scopes: &'a ResourcePool,

    // Heuristically we should never bleed into 4 completely nested fragments/components
    // Smallvec lets us stack allocate our little stack machine so the vast majority of cases are sane
    // TODO: use const generics instead of the 4 estimation
    stack: smallvec::SmallVec<[(u16, &'a VNode<'a>); 4]>,
}

impl<'a> RealChildIterator<'a> {
    pub fn new(starter: &'a VNode<'a>, scopes: &'a ResourcePool) -> Self {
        Self {
            scopes,
            stack: smallvec::smallvec![(0, starter)],
        }
    }
}

impl<'a> Iterator for RealChildIterator<'a> {
    type Item = &'a VNode<'a>;

    fn next(&mut self) -> Option<&'a VNode<'a>> {
        let mut should_pop = false;
        let mut returned_node: Option<&'a VNode<'a>> = None;
        let mut should_push = None;

        while returned_node.is_none() {
            if let Some((count, node)) = self.stack.last_mut() {
                match &node {
                    // We can only exit our looping when we get "real" nodes
                    // This includes fragments and components when they're empty (have a single root)
                    VNode::Element(_) | VNode::Text(_) | VNode::Suspended(_) | VNode::Anchor(_) => {
                        // We've recursed INTO an element/text
                        // We need to recurse *out* of it and move forward to the next
                        should_pop = true;
                        returned_node = Some(node);
                    }

                    // If we get a fragment we push the next child
                    VNode::Fragment(frag) => {
                        let subcount = *count as usize;

                        if frag.children.is_empty() {
                            should_pop = true;
                            returned_node = Some(node);
                        }

                        if subcount >= frag.children.len() {
                            should_pop = true;
                        } else {
                            should_push = Some(&frag.children[subcount]);
                        }
                    }

                    // For components, we load their root and push them onto the stack
                    VNode::Component(sc) => {
                        let scope = self
                            .scopes
                            .get_scope(&sc.associated_scope.get().unwrap())
                            .unwrap();

                        // Simply swap the current node on the stack with the root of the component
                        *node = scope.frames.fin_head();
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
