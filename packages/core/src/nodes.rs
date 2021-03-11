//! Virtual Node Support
//! VNodes represent lazily-constructed VDom trees that support diffing and event handlers.
//!
//! These VNodes should be *very* cheap and *very* fast to construct - building a full tree should be insanely quick.

use crate::{
    events::VirtualEvent,
    innerlude::{Context, Properties, ScopeIdx, FC},
};

use bumpalo::Bump;
use std::fmt::Debug;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    marker::PhantomData,
    rc::Rc,
};

/// A domtree represents the result of "Viewing" the context
/// It's a placeholder over vnodes, to make working with lifetimes easier
pub struct DomTree;

// ==============================
//   VNODES
// ==============================

/// Tools for the base unit of the virtual dom - the VNode
/// VNodes are intended to be quickly-allocated, lightweight enum values.
///
/// Components will be generating a lot of these very quickly, so we want to
/// limit the amount of heap allocations / overly large enum sizes.
#[derive(Debug)]
pub enum VNode<'src> {
    /// An element node (node type `ELEMENT_NODE`).
    Element(&'src VElement<'src>),

    /// A text node (node type `TEXT_NODE`).
    ///
    /// Note: This wraps a `VText` instead of a plain `String` in
    /// order to enable custom methods like `create_text_node()` on the
    /// wrapped type.
    Text(VText<'src>),

    /// A "suspended component"
    /// This is a masqeurade over an underlying future that needs to complete
    /// When the future is completed, the VNode will then trigger a render
    Suspended,

    /// A User-defined componen node (node type COMPONENT_NODE)
    Component(VComponent<'src>),
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
        key: NodeKey,
        tag_name: &'a str,
        listeners: &'a [Listener<'a>],
        attributes: &'a [Attribute<'a>],
        children: &'a [VNode<'a>],
        namespace: Option<&'a str>,
    ) -> VNode<'a> {
        let element = bump.alloc_with(|| VElement {
            key,
            tag_name,
            listeners,
            attributes,
            children,
            namespace,
        });
        VNode::Element(element)
    }

    /// Construct a new text node with the given text.
    #[inline]
    pub fn text(text: &'a str) -> VNode<'a> {
        VNode::Text(VText { text })
    }

    #[inline]
    pub(crate) fn key(&self) -> NodeKey {
        match &self {
            VNode::Text(_) => NodeKey::NONE,
            VNode::Element(e) => e.key,
            VNode::Suspended => {
                todo!()
            }
            VNode::Component(_) => {
                todo!()
            }
        }
    }
}

// ========================================================
//   VElement (div, h1, etc), attrs, keys, listener handle
// ========================================================
#[derive(Debug)]
pub struct VElement<'a> {
    /// Elements have a tag name, zero or more attributes, and zero or more
    pub key: NodeKey,
    pub tag_name: &'a str,
    pub listeners: &'a [Listener<'a>],
    pub attributes: &'a [Attribute<'a>],
    pub children: &'a [VNode<'a>],
    pub namespace: Option<&'a str>,
    // The HTML tag, such as "div"
    // pub tag: &'a str,

    // pub tag_name: &'a str,
    // pub attributes: &'a [Attribute<'a>],
    // todo: hook up listeners
    // pub listeners: &'a [Listener<'a>],
    // / HTML attributes such as id, class, style, etc
    // pub attrs: HashMap<String, String>,
    // TODO: @JON Get this to not heap allocate, but rather borrow
    // pub attrs: HashMap<&'static str, &'static str>,

    // TODO @Jon, re-enable "events"
    //
    // /// Events that will get added to your real DOM element via `.addEventListener`
    // pub events: Events,
    // pub events: HashMap<String, ()>,

    // /// The children of this `VNode`. So a <div> <em></em> </div> structure would
    // /// have a parent div and one child, em.
    // pub children: Vec<VNode>,
}

impl<'a> VElement<'a> {
    // The tag of a component MUST be known at compile time
    pub fn new(_tag: &'a str) -> Self {
        todo!()
        // VElement {
        //     tag,
        //     attrs: HashMap::new(),
        //     events: HashMap::new(),
        //     // events: Events(HashMap::new()),
        //     children: vec![],
        // }
    }
}

/// An attribute on a DOM node, such as `id="my-thing"` or
/// `href="https://example.com"`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

    pub scope: ScopeIdx,
    pub id: usize,

    // pub(crate) _i: &'bump str,
    // #[serde(skip_serializing, skip_deserializing, default="")]
    // /// The callback to invoke when the event happens.
    pub(crate) callback: &'bump (dyn Fn(VirtualEvent)),
}
impl Debug for Listener<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Listener")
            .field("event", &self.event)
            .finish()
    }
}

/// The key for keyed children.
///
/// Keys must be unique among siblings.
///
/// If any sibling is keyed, then they all must be keyed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeKey(pub(crate) u32);

impl Default for NodeKey {
    fn default() -> NodeKey {
        NodeKey::NONE
    }
}
impl NodeKey {
    /// The default, lack of a key.
    pub const NONE: NodeKey = NodeKey(u32::MAX);

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
    pub fn new(key: u32) -> Self {
        debug_assert_ne!(key, u32::MAX);
        NodeKey(key)
    }
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VText<'bump> {
    pub text: &'bump str,
}

impl<'a> VText<'a> {
    // / Create an new `VText` instance with the specified text.
    pub fn new(text: &'a str) -> Self
// pub fn new(text: Into<str>) -> Self
    {
        VText { text: text.into() }
    }
}

// ==============================
//   Custom components
// ==============================

/// Virtual Components for custom user-defined components
/// Only supports the functional syntax
pub type StableScopeAddres = Rc<RefCell<Option<usize>>>;

#[derive(Debug)]
pub struct VComponent<'src> {
    pub stable_addr: StableScopeAddres,
    pub comparator: Comparator,
    pub caller: Caller,
    _p: PhantomData<&'src ()>,
}
pub struct Comparator(Box<dyn Fn(&dyn Any) -> bool>);
impl std::fmt::Debug for Comparator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

pub struct Caller(Box<dyn Fn(Context) -> DomTree>);
impl std::fmt::Debug for Caller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl<'a> VComponent<'a> {
    // use the type parameter on props creation and move it into a portable context
    // this lets us keep scope generic *and* downcast its props when we need to:
    // - perform comparisons when diffing (memoization)
    // TODO: lift the requirement that props need to be static
    // we want them to borrow references... maybe force implementing a "to_static_unsafe" trait
    pub fn new<P: Properties + 'static>(caller: FC<P>, props: P) -> Self {
        let props = Rc::new(props);

        // used for memoization
        let p1 = props.clone();
        let props_comparator = move |new_props: &dyn Any| -> bool {
            new_props
                .downcast_ref::<P>()
                .map(|new_p| p1.as_ref() == new_p)
                .unwrap_or_else(|| {
                    log::debug!("downcasting failed, this receovered but SHOULD NOT FAIL");
                    false
                })
        };

        // used for actually rendering the custom component
        let p2 = props.clone();
        let caller = move |ctx: Context| -> DomTree { caller(ctx, p2.as_ref()) };

        Self {
            _p: PhantomData,
            comparator: Comparator(Box::new(props_comparator)),
            caller: Caller(Box::new(caller)),
            stable_addr: Rc::new(RefCell::new(None)),
        }
    }
}
