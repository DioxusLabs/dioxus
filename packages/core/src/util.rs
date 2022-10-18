use crate::innerlude::{VNode, VirtualDom};

/// An iterator that only yields "real" [`Element`]s. IE only Elements that are
/// not [`VNode::Component`] or [`VNode::Fragment`], .
pub struct ElementIdIterator<'a> {
    vdom: &'a VirtualDom,

    // Heuristically we should never bleed into 5 completely nested fragments/components
    // Smallvec lets us stack allocate our little stack machine so the vast majority of cases are sane
    stack: smallvec::SmallVec<[(u16, &'a VNode<'a>); 5]>,
}

impl<'a> ElementIdIterator<'a> {
    /// Create a new iterator from the given [`VirtualDom`] and [`VNode`]
    ///
    /// This will allow you to iterate through all the real childrne of the [`VNode`].
    pub fn new(vdom: &'a VirtualDom, node: &'a VNode<'a>) -> Self {
        Self {
            vdom,
            stack: smallvec::smallvec![(0, node)],
        }
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
                    VNode::Element(_)
                    | VNode::Text(_)
                    | VNode::Placeholder(_)
                    | VNode::TemplateRef(_) => {
                        // We've recursed INTO an element/text
                        // We need to recurse *out* of it and move forward to the next
                        // println!("Found element! Returning it!");
                        should_pop = true;
                        returned_node = Some(&**node);
                    }

                    // If we get a fragment we push the next child
                    VNode::Fragment(frag) => {
                        let count = *count as usize;
                        if count >= frag.children.len() {
                            should_pop = true;
                        } else {
                            should_push = Some(&frag.children[count]);
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

/// This intentionally leaks once per element name to allow more flexability when hot reloding templetes
#[cfg(all(any(feature = "hot-reload", debug_assertions), feature = "serde"))]
mod leaky {
    use std::sync::Mutex;

    use once_cell::sync::Lazy;
    use rustc_hash::FxHashSet;
    static STATIC_CACHE: Lazy<Mutex<FxHashSet<&'static str>>> =
        Lazy::new(|| Mutex::new(FxHashSet::default()));

    pub fn deserialize_static_leaky<'de, D>(d: D) -> Result<&'static str, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;
        let s = <&str>::deserialize(d)?;
        Ok(if let Some(stat) = STATIC_CACHE.lock().unwrap().get(s) {
            *stat
        } else {
            Box::leak(s.into())
        })
    }

    pub fn deserialize_static_leaky_ns<'de, D>(d: D) -> Result<Option<&'static str>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;
        Ok(<Option<&str>>::deserialize(d)?.map(|s| {
            if let Some(stat) = STATIC_CACHE.lock().unwrap().get(s) {
                *stat
            } else {
                Box::leak(s.into())
            }
        }))
    }
}

#[cfg(all(any(feature = "hot-reload", debug_assertions), feature = "serde"))]
pub use leaky::*;
