//! Virtual Node Support
//! VNodes represent lazily-constructed VDom trees that support diffing and event handlers.
//!
//! These VNodes should be *very* cheap and *very* fast to construct - building a full tree should be insanely quick.

use crate::{
    events::VirtualEvent,
    innerlude::{Context, Properties, Scope, ScopeIdx, FC},
    nodebuilder::{text3, NodeCtx},
    virtual_dom::RealDomNode,
};
use bumpalo::Bump;
use std::{
    cell::{Cell, RefCell},
    fmt::{Arguments, Debug, Formatter},
    rc::Rc,
};

/// Tools for the base unit of the virtual dom - the VNode
/// VNodes are intended to be quickly-allocated, lightweight enum values.
///
/// Components will be generating a lot of these very quickly, so we want to
/// limit the amount of heap allocations / overly large enum sizes.
pub enum VNode<'src> {
    /// An element node (node type `ELEMENT_NODE`).
    Element(&'src VElement<'src>),

    /// A text node (node type `TEXT_NODE`).
    Text(VText<'src>),

    /// A fragment is a "virtual position" in the DOM
    /// Fragments may have children and keys
    Fragment(&'src VFragment<'src>),

    /// A "suspended component"
    /// This is a masqeurade over an underlying future that needs to complete
    /// When the future is completed, the VNode will then trigger a render
    Suspended,

    /// A User-defined componen node (node type COMPONENT_NODE)
    Component(&'src VComponent<'src>),
}

// it's okay to clone because vnodes are just references to places into the bump
impl<'a> Clone for VNode<'a> {
    fn clone(&self) -> Self {
        match self {
            VNode::Element(element) => VNode::Element(element),
            VNode::Text(old) => VNode::Text(old.clone()),
            VNode::Fragment(fragment) => VNode::Fragment(fragment),
            VNode::Component(component) => VNode::Component(component),
            VNode::Suspended => VNode::Suspended,
        }
    }
}

impl<'old, 'new> VNode<'old> {
    // performs a somewhat costly clone of this vnode into another bump
    // this is used when you want to drag nodes from an old frame into a new frame
    // There is no way to safely drag listeners over (no way to clone a closure)
    //
    // This method will only be called if a component was once a real node and then becomes suspended
    fn deep_clone_to_new_bump(&self, new: &'new Bump) -> VNode<'new> {
        match self {
            VNode::Element(el) => {
                let new_el: VElement<'new> = VElement {
                    key: NodeKey::NONE,
                    // key: el.key.clone(),
                    tag_name: el.tag_name,
                    // wipe listeners on deep clone, there's no way to know what other bump material they might be referencing (nodes, etc)
                    listeners: &[],
                    attributes: {
                        let attr_vec = bumpalo::collections::Vec::new_in(new);
                        attr_vec.into_bump_slice()
                    },
                    children: {
                        let attr_vec = bumpalo::collections::Vec::new_in(new);
                        attr_vec.into_bump_slice()
                    },
                    namespace: el.namespace.clone(),
                    dom_id: el.dom_id.clone(),
                };

                VNode::Element(new.alloc_with(move || new_el))
            }
            VNode::Text(_) => todo!(),
            VNode::Fragment(_) => todo!(),
            VNode::Suspended => todo!(),
            VNode::Component(_) => todo!(),
        }
    }
}

impl<'a> VNode<'a> {
    /// Low-level constructor for making a new `Node` of type element with given
    /// parts.
    ///
    /// This is primarily intended for JSX and templating proc-macros to compile
    /// down into. If you are building nodes by-hand, prefer using the
    /// `dodrio::builder::*` APIs.
    #[inline]
    pub fn element(
        bump: &'a Bump,
        key: NodeKey<'a>,
        tag_name: &'static str,
        listeners: &'a [Listener<'a>],
        attributes: &'a [Attribute<'a>],
        children: &'a [VNode<'a>],
        namespace: Option<&'static str>,
    ) -> VNode<'a> {
        let element = bump.alloc_with(|| VElement {
            key,
            tag_name,
            listeners,
            attributes,
            children,
            namespace,
            dom_id: Cell::new(RealDomNode::empty()),
        });
        VNode::Element(element)
    }

    /// Construct a new text node with the given text.
    #[inline]
    pub fn text(text: &'a str) -> VNode<'a> {
        VNode::Text(VText {
            text,
            dom_id: Cell::new(RealDomNode::empty()),
        })
    }

    pub fn text_args(bump: &'a Bump, args: Arguments) -> VNode<'a> {
        text3(bump, args)
    }

    #[inline]
    pub(crate) fn key(&self) -> NodeKey {
        match &self {
            VNode::Text { .. } => NodeKey::NONE,
            VNode::Element(e) => e.key,
            VNode::Fragment(frag) => frag.key,
            VNode::Component(c) => c.key,

            // todo suspend should be allowed to have keys
            VNode::Suspended => NodeKey::NONE,
        }
    }

    fn get_child(&self, id: u32) -> Option<&'a VNode<'a>> {
        todo!()
    }

    pub fn is_real(&self) -> bool {
        match self {
            VNode::Element(_) => true,
            VNode::Text(_) => true,
            VNode::Fragment(_) => false,
            VNode::Suspended => false,
            VNode::Component(_) => false,
        }
    }

    pub fn get_mounted_id(&self) -> Option<RealDomNode> {
        match self {
            VNode::Element(el) => Some(el.dom_id.get()),
            VNode::Text(te) => Some(te.dom_id.get()),
            VNode::Fragment(_) => todo!(),
            VNode::Suspended => todo!(),
            VNode::Component(_) => todo!(),
        }
    }
}

impl Debug for VNode<'_> {
    fn fmt(&self, s: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            VNode::Element(el) => write!(s, "element, {}", el.tag_name),
            VNode::Text(t) => write!(s, "text, {}", t.text),
            VNode::Fragment(_) => write!(s, "fragment"),
            VNode::Suspended => write!(s, "suspended"),
            VNode::Component(_) => write!(s, "component"),
        }
    }
}

#[derive(Clone)]
pub struct VText<'src> {
    pub text: &'src str,
    pub dom_id: Cell<RealDomNode>,
}

// ========================================================
//   VElement (div, h1, etc), attrs, keys, listener handle
// ========================================================

#[derive(Clone)]
pub struct VElement<'a> {
    /// Elements have a tag name, zero or more attributes, and zero or more
    pub key: NodeKey<'a>,
    pub tag_name: &'static str,
    pub listeners: &'a [Listener<'a>],
    pub attributes: &'a [Attribute<'a>],
    pub children: &'a [VNode<'a>],
    pub namespace: Option<&'static str>,
    pub dom_id: Cell<RealDomNode>,
}

/// An attribute on a DOM node, such as `id="my-thing"` or
/// `href="https://example.com"`.
#[derive(Clone, Debug)]
pub struct Attribute<'a> {
    pub name: &'static str,
    pub value: &'a str,
}

impl<'a> Attribute<'a> {
    /// Get this attribute's name, such as `"id"` in `<div id="my-thing" />`.
    #[inline]
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// The attribute value, such as `"my-thing"` in `<div id="my-thing" />`.
    #[inline]
    pub fn value(&self) -> &'a str {
        self.value
    }

    /// Certain attributes are considered "volatile" and can change via user
    /// input that we can't see when diffing against the old virtual DOM. For
    /// these attributes, we want to always re-set the attribute on the physical
    /// DOM node, even if the old and new virtual DOM nodes have the same value.
    #[inline]
    pub(crate) fn is_volatile(&self) -> bool {
        match self.name {
            "value" | "checked" | "selected" => true,
            _ => false,
        }
    }
}

pub struct ListenerHandle {
    pub event: &'static str,
    pub scope: ScopeIdx,
    pub id: usize,
}

/// An event listener.
pub struct Listener<'bump> {
    /// The type of event to listen for.
    pub(crate) event: &'static str,

    /// Which scope?
    /// This might not actually be relevant
    pub scope: ScopeIdx,

    pub mounted_node: &'bump Cell<RealDomNode>,

    /// The callback to invoke when the event happens.
    pub(crate) callback: &'bump dyn Fn(VirtualEvent),
}

/// The key for keyed children.
///
/// Keys must be unique among siblings.
///
/// If any sibling is keyed, then they all must be keyed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeKey<'a>(pub(crate) Option<&'a str>);

impl<'a> Default for NodeKey<'a> {
    fn default() -> NodeKey<'a> {
        NodeKey::NONE
    }
}
impl<'a> NodeKey<'a> {
    /// The default, lack of a key.
    pub const NONE: NodeKey<'a> = NodeKey(None);

    /// Is this key `NodeKey::NONE`?
    #[inline]
    pub fn is_none(&self) -> bool {
        *self == Self::NONE
    }

    /// Is this key not `NodeKey::NONE`?
    #[inline]
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Create a new `NodeKey`.
    ///
    /// `key` must not be `u32::MAX`.
    #[inline]
    pub fn new(key: &'a str) -> Self {
        NodeKey(Some(key))
    }
}

// ==============================
//   Custom components
// ==============================

/// Virtual Components for custom user-defined components
/// Only supports the functional syntax
pub type StableScopeAddres = Option<u32>;
pub type VCompAssociatedScope = Option<ScopeIdx>;

pub struct VComponent<'src> {
    pub key: NodeKey<'src>,

    pub mounted_root: Cell<RealDomNode>,

    pub ass_scope: RefCell<VCompAssociatedScope>,

    // pub comparator: Rc<dyn Fn(&VComponent) -> bool + 'src>,
    pub caller: Rc<dyn Fn(&Scope) -> VNode>,

    pub children: &'src [VNode<'src>],

    pub comparator: Option<&'src dyn Fn(&VComponent) -> bool>,

    // a pointer into the bump arena (given by the 'src lifetime)
    // raw_props: Box<dyn Any>,
    raw_props: *const (),

    // a pointer to the raw fn typ
    pub user_fc: *const (),
}

impl<'a> VComponent<'a> {
    /// When the rsx! macro is called, it will check if the CanMemo flag is set to true (from the Props impl)
    /// If it is set to true, then this method will be called which implements automatic memoization.
    ///
    /// If the CanMemo is `false`, then the macro will call the backup method which always defaults to "false"
    pub fn new<P: Properties + 'a>(
        cx: &NodeCtx<'a>,
        component: FC<P>,
        props: P,
        key: Option<&'a str>,
        children: &'a [VNode<'a>],
    ) -> Self {
        let bump = cx.bump();
        let user_fc = component as *const ();

        let props = bump.alloc(props);
        let raw_props = props as *const P as *const ();

        let comparator: Option<&dyn Fn(&VComponent) -> bool> = {
            Some(bump.alloc(move |other: &VComponent| {
                // Safety:
                // ------
                //
                // Invariants:
                // - Component function pointers are the same
                // - Generic properties on the same function pointer are the same
                // - Lifetime of P borrows from its parent
                // - The parent scope still exists when method is called
                // - Casting from T to *const () is portable
                //
                // Explanation:
                //   We are guaranteed that the props will be of the same type because
                //   there is no way to create a VComponent other than this `new` method.
                //
                //   Therefore, if the render functions are identical (by address), then so will be
                //   props type paramter (because it is the same render function). Therefore, we can be
                //   sure that it is safe to interperet the previous props raw pointer as the same props
                //   type. From there, we can call the props' "memoize" method to see if we can
                //   avoid re-rendering the component.
                if user_fc == other.user_fc {
                    let real_other = unsafe { &*(other.raw_props as *const _ as *const P) };
                    let props_memoized = unsafe { props.memoize(&real_other) };
                    match (props_memoized, children.len() == 0) {
                        (true, true) => true,
                        _ => false,
                    }
                } else {
                    false
                }
            }))
        };

        Self {
            user_fc,
            comparator,
            raw_props,
            children,
            ass_scope: RefCell::new(None),
            key: match key {
                Some(key) => NodeKey::new(key),
                None => NodeKey(None),
            },
            caller: create_closure(component, raw_props),
            mounted_root: Cell::new(RealDomNode::empty()),
        }
    }
}

type Captured<'a> = Rc<dyn for<'r> Fn(&'r Scope) -> VNode<'r> + 'a>;

fn create_closure<'a, P: 'a>(
    component: FC<P>,
    raw_props: *const (),
) -> Rc<dyn for<'r> Fn(&'r Scope) -> VNode<'r>> {
    // ) -> impl for<'r> Fn(&'r Scope) -> VNode<'r> {
    let g: Captured = Rc::new(move |scp: &Scope| -> VNode {
        // cast back into the right lifetime
        let safe_props: &'_ P = unsafe { &*(raw_props as *const P) };
        // let cx: Context<P2> = todo!();
        let cx: Context<P> = Context {
            props: safe_props,
            scope: scp,
        };

        let g = component(cx);
        let g2 = unsafe { std::mem::transmute(g) };
        g2
    });
    let r: Captured<'static> = unsafe { std::mem::transmute(g) };
    r
}

pub struct VFragment<'src> {
    pub key: NodeKey<'src>,
    pub children: &'src [VNode<'src>],
}

impl<'a> VFragment<'a> {
    pub fn new(key: Option<&'a str>, children: &'a [VNode<'a>]) -> Self {
        let key = match key {
            Some(key) => NodeKey::new(key),
            None => NodeKey(None),
        };

        Self { key, children }
    }
}

/// This method converts a list of nested real/virtual nodes into a stream of nodes that are definitely associated
/// with the real dom. The only types of nodes that may be returned are text, elemets, and components.
///
/// Components *are* considered virtual, but this iterator can't necessarily handle them without the scope arena.
///
/// Why?
/// ---
/// Fragments are seen as virtual nodes but are actually a list of possibly-real nodes.
/// JS implementations normalize their node lists when fragments are present. Here, we just create a new iterator
/// that iterates through the recursive nesting of fragments.
///
/// Fragments are stupid and I wish we didn't need to support them.
///
/// This iterator only supports 3 levels of nested fragments
///
pub fn iterate_real_nodes<'a>(nodes: &'a [VNode<'a>]) -> RealNodeIterator<'a> {
    RealNodeIterator::new(nodes)
}

pub struct RealNodeIterator<'a> {
    nodes: &'a [VNode<'a>],

    // this node is always a "real" node
    // the index is "what sibling # is it"
    // IE in a list of children on a fragment, the node will be a text node that's the 5th sibling
    node_stack: Vec<(&'a VNode<'a>, u32)>,
}

impl<'a> RealNodeIterator<'a> {
    // We immediately descend to the first real node we can find
    fn new(nodes: &'a [VNode<'a>]) -> Self {
        let mut node_stack = Vec::new();
        if nodes.len() > 0 {
            let mut cur_node = nodes.get(0).unwrap();
            loop {
                node_stack.push((cur_node, 0_u32));
                if !cur_node.is_real() {
                    cur_node = cur_node.get_child(0).unwrap();
                } else {
                    break;
                }
            }
        }

        Self { nodes, node_stack }
    }

    // // advances the cursor to the next element, panicing if we're on the 3rd level and still finding fragments
    // fn advance_cursor(&mut self) {
    //     let (mut cur_node, mut cur_id) = self.node_stack.last().unwrap();

    //     while !cur_node.is_real() {
    //         match cur_node {
    //             VNode::Element(_) | VNode::Text(_) => todo!(),
    //             VNode::Suspended => todo!(),
    //             VNode::Component(_) => todo!(),
    //             VNode::Fragment(frag) => {
    //                 let p = frag.children;
    //             }
    //         }
    //     }
    // }

    fn next_node(&mut self) -> bool {
        let (mut cur_node, cur_id) = self.node_stack.last_mut().unwrap();

        match cur_node {
            VNode::Fragment(frag) => {
                //
                if *cur_id + 1 > frag.children.len() as u32 {
                    self.node_stack.pop();
                    let next = self.node_stack.last_mut();
                    return false;
                }
                *cur_id += 1;
                true
            }

            VNode::Element(_) => todo!(),
            VNode::Text(_) => todo!(),
            VNode::Suspended => todo!(),
            VNode::Component(_) => todo!(),
        }
    }

    fn get_current_node(&self) -> Option<&VNode<'a>> {
        self.node_stack.last().map(|(node, id)| match node {
            VNode::Element(_) => todo!(),
            VNode::Text(_) => todo!(),
            VNode::Fragment(_) => todo!(),
            VNode::Suspended => todo!(),
            VNode::Component(_) => todo!(),
        })
    }
}

impl<'a> Iterator for RealNodeIterator<'a> {
    type Item = &'a VNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
        // let top_idx = self.nesting_idxs.get_mut(0).unwrap();
        // let node = &self.nodes.get_mut(*top_idx as usize);

        // if node.is_none() {
        //     return None;
        // }
        // let node = node.unwrap();

        // match node {
        //     VNode::Element(_) | VNode::Text(_) => {
        //         *top_idx += 1;
        //         return Some(node);
        //     }
        //     VNode::Suspended => todo!(),
        //     // we need access over the scope map
        //     VNode::Component(_) => todo!(),

        //     VNode::Fragment(frag) => {
        //         let nest_idx = self.nesting_idxs.get_mut(1).unwrap();
        //         let node = &frag.children.get_mut(*nest_idx as usize);
        //         match node {
        //             VNode::Element(_) | VNode::Text(_) => {
        //                 *nest_idx += 1;
        //                 return Some(node);
        //             }
        //             VNode::Fragment(_) => todo!(),
        //             VNode::Suspended => todo!(),
        //             VNode::Component(_) => todo!(),
        //         }
        //     }
        // }
    }
}

mod tests {
    use crate::debug_renderer::DebugRenderer;
    use crate::nodebuilder::LazyNodes;

    use crate as dioxus;
    use dioxus::prelude::*;
    #[test]
    fn iterate_nodes() {
        let rs = rsx! {
            Fragment {
                Fragment {
                    Fragment {
                        Fragment {
                            h1 {"abc1"}
                        }
                        h2 {"abc2"}
                    }
                    h3 {"abc3"}
                }
                h4 {"abc4"}
            }
        };
    }
}
