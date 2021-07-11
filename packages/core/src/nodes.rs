//! Virtual Node Support
//! VNodes represent lazily-constructed VDom trees that support diffing and event handlers.
//!
//! These VNodes should be *very* cheap and *very* fast to construct - building a full tree should be insanely quick.

use crate::{
    arena::SharedArena,
    events::VirtualEvent,
    innerlude::{Context, Properties, RealDom, RealDomNode, Scope, ScopeIdx, FC},
    nodebuilder::NodeFactory,
};
use appendlist::AppendList;
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

    /// A fragment is a list of elements that might have a dynamic order.
    /// Normally, children will have a fixed order. However, Fragments allow a dynamic order and must be diffed differently.
    ///
    /// Fragments don't have a single mount into the dom, so their position is characterized by the head and tail nodes.
    ///
    /// Fragments may have children and keys
    Fragment(&'src VFragment<'src>),

    /// A "suspended component"
    /// This is a masqeurade over an underlying future that needs to complete
    /// When the future is completed, the VNode will then trigger a render and the `real` field gets populated
    Suspended { real: Cell<RealDomNode> },

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
            VNode::Suspended { real } => VNode::Suspended { real: real.clone() },
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
                    is_static: el.is_static.clone(),
                };

                VNode::Element(new.alloc_with(move || new_el))
            }
            VNode::Text(_) => todo!(),
            VNode::Fragment(_) => todo!(),
            VNode::Suspended { real } => todo!(),
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
            is_static: Cell::new(false),
        });
        VNode::Element(element)
    }

    pub fn static_text(text: &'static str) -> VNode {
        VNode::Text(VText {
            text,
            is_static: true,
            dom_id: Cell::new(RealDomNode::empty()),
        })
    }
    /// Construct a new text node with the given text.
    pub fn text(bump: &'a Bump, args: Arguments) -> VNode<'a> {
        match args.as_str() {
            Some(text) => VNode::static_text(text),
            None => {
                use bumpalo::core_alloc::fmt::Write;
                let mut s = bumpalo::collections::String::new_in(bump);
                s.write_fmt(args).unwrap();
                VNode::Text(VText {
                    text: s.into_bump_str(),
                    is_static: false,
                    dom_id: Cell::new(RealDomNode::empty()),
                })
            }
        }
    }

    #[inline]
    pub(crate) fn key(&self) -> NodeKey {
        match &self {
            VNode::Text { .. } => NodeKey::NONE,
            VNode::Element(e) => e.key,
            VNode::Fragment(frag) => frag.key,
            VNode::Component(c) => c.key,

            // todo suspend should be allowed to have keys
            VNode::Suspended { .. } => NodeKey::NONE,
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
            VNode::Suspended { .. } => false,
            VNode::Component(_) => false,
        }
    }

    pub fn get_mounted_id(&self, components: &SharedArena) -> Option<RealDomNode> {
        match self {
            VNode::Element(el) => Some(el.dom_id.get()),
            VNode::Text(te) => Some(te.dom_id.get()),
            VNode::Fragment(_) => todo!(),
            VNode::Suspended { .. } => todo!(),
            VNode::Component(el) => Some(el.mounted_root.get()),
        }
    }
}

impl Debug for VNode<'_> {
    fn fmt(&self, s: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            VNode::Element(el) => write!(s, "element, {}", el.tag_name),
            VNode::Text(t) => write!(s, "text, {}", t.text),
            VNode::Fragment(_) => write!(s, "fragment"),
            VNode::Suspended { .. } => write!(s, "suspended"),
            VNode::Component(_) => write!(s, "component"),
        }
    }
}

#[derive(Clone)]
pub struct VText<'src> {
    pub text: &'src str,
    pub is_static: bool,
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
    pub is_static: Cell<bool>,
}

/// An attribute on a DOM node, such as `id="my-thing"` or
/// `href="https://example.com"`.
#[derive(Clone, Debug)]
pub struct Attribute<'a> {
    pub name: &'static str,
    pub value: &'a str,
    pub is_static: bool,

    /// If an attribute is "namespaced", then it belongs to a group
    /// The most common namespace is the "style" namespace
    // pub is_dynamic: bool,
    pub namespace: Option<&'static str>,
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
    pub(crate) callback: &'bump dyn FnMut(VirtualEvent),
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

    #[inline]
    pub fn new_opt(key: Option<&'a str>) -> Self {
        NodeKey(key)
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

    pub ass_scope: Cell<VCompAssociatedScope>,

    // todo: swap the RC out with
    pub caller: Rc<dyn Fn(&Scope) -> VNode>,

    pub children: &'src [VNode<'src>],

    pub comparator: Option<&'src dyn Fn(&VComponent) -> bool>,

    pub is_static: bool,

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
        cx: &NodeFactory<'a>,
        component: FC<P>,
        props: P,
        key: Option<&'a str>,
        children: &'a [VNode<'a>],
    ) -> Self {
        let bump = cx.bump();
        let user_fc = component as *const ();

        let props = bump.alloc(props);
        let raw_props = props as *const P as *const ();

        let comparator: Option<&dyn Fn(&VComponent) -> bool> = Some(bump.alloc_with(|| {
            move |other: &VComponent| {
                // Safety:
                // ------
                //
                // Invariants:
                // - Component function pointers are the same
                // - Generic properties on the same function pointer are the same
                // - Lifetime of P borrows from its parent
                // - The parent scope still exists when method is called
                // - Casting from T to *const () is portable
                // - Casting raw props to P can only happen when P is static
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
            }
        }));

        let key = match key {
            Some(key) => NodeKey::new(key),
            None => NodeKey(None),
        };

        let caller = create_component_caller(component, raw_props);

        // If the component does not have children, has no props (we can't memoize props), and has no no key, then we don't
        // need to bother diffing it in the future
        //
        // This is more of an optimization to prevent unnecessary descending through the tree during diffing, rather than
        // actually speeding up the diff process itself
        let is_static = children.len() == 0 && P::IS_STATIC && key.is_none();

        Self {
            user_fc,
            comparator,
            raw_props,
            children,
            ass_scope: Cell::new(None),
            key,
            caller,
            is_static,
            mounted_root: Cell::new(RealDomNode::empty()),
        }
    }
}

type Captured<'a> = Rc<dyn for<'r> Fn(&'r Scope) -> VNode<'r> + 'a>;

pub fn create_component_caller<'a, P: 'a>(
    user_component: FC<P>,
    raw_props: *const (),
) -> Rc<dyn for<'r> Fn(&'r Scope) -> VNode<'r>> {
    let g: Captured = Rc::new(move |scp: &Scope| -> VNode {
        // cast back into the right lifetime
        let safe_props: &'_ P = unsafe { &*(raw_props as *const P) };
        let tasks = RefCell::new(Vec::new());
        let cx: Context<P> = Context {
            props: safe_props,
            scope: scp,
            tasks: &tasks,
        };

        let g = user_component(cx);

        for task in tasks.borrow_mut().drain(..) {
            scp.submit_task(task);
        }

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
