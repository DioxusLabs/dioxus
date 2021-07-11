/// This iterator iterates through a list of virtual children and only returns real children (Elements or Text).
///
/// This iterator is useful when it's important to load the next real root onto the top of the stack for operations like
/// "InsertBefore".
struct RealChildIterator<'a> {
    scopes: &'a SharedArena,

    // Heuristcally we should never bleed into 5 completely nested fragments/components
    // Smallvec lets us stack allocate our little stack machine so the vast majority of cases are sane
    stack: smallvec::SmallVec<[(u16, &'a VNode<'a>); 5]>,
}

impl<'a> RealChildIterator<'a> {
    fn new(starter: &'a VNode<'a>, scopes: &'a SharedArena) -> Self {
        Self {
            scopes,
            stack: smallvec::smallvec![(0, starter)],
        }
    }
}

// impl<'a> DoubleEndedIterator for ChildIterator<'a> {
//     fn next_back(&mut self) -> Option<Self::Item> {
//         todo!()
//     }
// }

impl<'a> Iterator for RealChildIterator<'a> {
    type Item = &'a VNode<'a>;

    fn next(&mut self) -> Option<&'a VNode<'a>> {
        let mut should_pop = false;
        let mut returned_node = None;
        let mut should_push = None;

        while returned_node.is_none() {
            if let Some((count, node)) = self.stack.last_mut() {
                match node {
                    // We can only exit our looping when we get "real" nodes
                    VNode::Element(_) | VNode::Text(_) => {
                        // We've recursed INTO an element/text
                        // We need to recurse *out* of it and move forward to the next
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

                    // Immediately abort suspended nodes - can't do anything with them yet
                    // VNode::Suspended => should_pop = true,
                    VNode::Suspended { real } => todo!(),

                    // For components, we load their root and push them onto the stack
                    VNode::Component(sc) => {
                        let scope = self.scopes.try_get(sc.ass_scope.get().unwrap()).unwrap();

                        // Simply swap the current node on the stack with the root of the component
                        *node = scope.root();
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

mod tests {
    use super::*;
    use crate as dioxus;
    use crate::innerlude::*;
    use crate::util::DebugDom;
    use dioxus_core_macro::*;

    // #[test]
    // fn test_child_iterator() {
    //     static App: FC<()> = |cx| {
    //         cx.render(rsx! {
    //             Fragment {
    //                 div {}
    //                 h1 {}
    //                 h2 {}
    //                 h3 {}
    //                 Fragment {
    //                     "internal node"
    //                     div {
    //                         "baller text shouldn't show up"
    //                     }
    //                     p {

    //                     }
    //                     Fragment {
    //                         Fragment {
    //                             "wow you really like framgents"
    //                             Fragment {
    //                                 "why are you like this"
    //                                 Fragment {
    //                                     "just stop now please"
    //                                     Fragment {
    //                                         "this hurts"
    //                                         Fragment {
    //                                             "who needs this many fragments?????"
    //                                             Fragment {
    //                                                 "just... fine..."
    //                                                 Fragment {
    //                                                     "no"
    //                                                 }
    //                                             }
    //                                         }
    //                                     }
    //                                 }
    //                             }
    //                         }
    //                     }
    //                 }
    //                 "my text node 1"
    //                 "my text node 2"
    //                 "my text node 3"
    //                 "my text node 4"
    //             }
    //         })
    //     };
    //     let mut dom = VirtualDom::new(App);
    //     let mut renderer = DebugDom::new();
    //     dom.rebuild(&mut renderer).unwrap();
    //     let starter = dom.base_scope().root();
    //     let ite = RealChildIterator::new(starter, &dom.components);
    //     for child in ite {
    //         match child {
    //             VNode::Element(el) => println!("Found: Element {}", el.tag_name),
    //             VNode::Text(t) => println!("Found: Text {:?}", t.text),

    //             // These would represent failing cases.
    //             VNode::Fragment(_) => panic!("Found: Fragment"),
    //             VNode::Suspended { real } => panic!("Found: Suspended"),
    //             VNode::Component(_) => panic!("Found: Component"),
    //         }
    //     }
    // }
}
