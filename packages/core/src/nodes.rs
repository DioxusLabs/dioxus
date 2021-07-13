//! Virtual Node Support
//! VNodes represent lazily-constructed VDom trees that support diffing and event handlers.
//!
//! These VNodes should be *very* cheap and *very* fast to construct - building a full tree should be insanely quick.

use crate::{
    events::VirtualEvent,
    innerlude::{Context, Properties, RealDom, RealDomNode, Scope, ScopeIdx, FC},
};
use std::{
    cell::{Cell, RefCell},
    fmt::{Arguments, Debug, Formatter},
    marker::PhantomData,
    rc::Rc,
};

pub struct VNode<'src> {
    pub kind: VNodeKind<'src>,
    pub dom_id: Cell<RealDomNode>,
    pub key: Option<&'src str>,
}

/// Tools for the base unit of the virtual dom - the VNode
/// VNodes are intended to be quickly-allocated, lightweight enum values.
///
/// Components will be generating a lot of these very quickly, so we want to
/// limit the amount of heap allocations / overly large enum sizes.
pub enum VNodeKind<'src> {
    Text(VText<'src>),
    Element(&'src VElement<'src>),
    Fragment(VFragment<'src>),
    Component(&'src VComponent<'src>),
    Suspended,
}

pub struct VText<'src> {
    pub text: &'src str,
    pub is_static: bool,
}

pub struct VFragment<'src> {
    pub children: &'src [VNode<'src>],
    pub is_static: bool,
}

pub trait DioxusElement {
    const TAG_NAME: &'static str;
    const NAME_SPACE: Option<&'static str>;
    #[inline]
    fn tag_name(&self) -> &'static str {
        Self::TAG_NAME
    }
    #[inline]
    fn namespace(&self) -> Option<&'static str> {
        Self::NAME_SPACE
    }
}
pub struct VElement<'a> {
    // tag is always static
    pub tag_name: &'static str,
    pub namespace: Option<&'static str>,

    pub static_listeners: bool,
    pub listeners: &'a [Listener<'a>],

    pub static_attrs: bool,
    pub attributes: &'a [Attribute<'a>],

    pub static_children: bool,
    pub children: &'a [VNode<'a>],
}

/// An attribute on a DOM node, such as `id="my-thing"` or
/// `href="https://example.com"`.
#[derive(Clone, Debug)]
pub struct Attribute<'a> {
    pub name: &'static str,
    pub value: &'a str,
    pub is_static: bool,
    pub is_volatile: bool,
    // Doesn't exist in the html spec, mostly used to denote "style" tags - could be for any type of group
    pub namespace: Option<&'static str>,
}

/// An event listener.
/// IE onclick, onkeydown, etc
pub struct Listener<'bump> {
    /// The type of event to listen for.
    pub(crate) event: &'static str,
    pub scope: ScopeIdx,
    pub mounted_node: &'bump Cell<RealDomNode>,
    pub(crate) callback: &'bump dyn FnMut(VirtualEvent),
}

/// Virtual Components for custom user-defined components
/// Only supports the functional syntax
pub struct VComponent<'src> {
    pub ass_scope: Cell<Option<ScopeIdx>>,
    pub(crate) caller: Rc<dyn Fn(&Scope) -> VNode>,
    pub(crate) children: &'src [VNode<'src>],
    pub(crate) comparator: Option<&'src dyn Fn(&VComponent) -> bool>,
    pub is_static: bool,

    // a pointer into the bump arena (given by the 'src lifetime)
    pub(crate) raw_props: *const (),

    // a pointer to the raw fn typ
    pub(crate) user_fc: *const (),
}

/// This struct provides an ergonomic API to quickly build VNodes.
///
/// NodeFactory is used to build VNodes in the component's memory space.
/// This struct adds metadata to the final VNode about listeners, attributes, and children
#[derive(Copy, Clone)]
pub struct NodeFactory<'a> {
    pub scope_ref: &'a Scope,
    pub listener_id: &'a Cell<usize>,
}

impl<'a> NodeFactory<'a> {
    #[inline]
    pub fn bump(&self) -> &'a bumpalo::Bump {
        &self.scope_ref.cur_frame().bump
    }

    pub const fn const_text(&self, text: &'static str) -> VNodeKind<'static> {
        VNodeKind::Text(VText {
            is_static: true,
            text,
        })
    }

    pub const fn const_fragment(&self, children: &'static [VNode<'static>]) -> VNodeKind<'static> {
        VNodeKind::Fragment(VFragment {
            children,
            is_static: true,
        })
    }

    pub fn static_text(text: &'static str) -> VNode {
        VNode {
            dom_id: RealDomNode::empty_cell(),
            key: None,
            kind: VNodeKind::Text(VText {
                text,
                is_static: true,
            }),
        }
    }

    pub fn raw_text(&self, args: Arguments) -> (&'a str, bool) {
        match args.as_str() {
            Some(static_str) => (static_str, true),
            None => {
                use bumpalo::core_alloc::fmt::Write;
                let mut s = bumpalo::collections::String::new_in(self.bump());
                s.write_fmt(args).unwrap();
                (s.into_bump_str(), false)
            }
        }
    }

    /// Create some text that's allocated along with the other vnodes
    pub fn text(&self, args: Arguments) -> VNode<'a> {
        let (text, is_static) = self.raw_text(args);
        VNode {
            dom_id: RealDomNode::empty_cell(),
            key: None,
            kind: VNodeKind::Text(VText { text, is_static }),
        }
    }

    pub fn raw_element(
        &self,
        _tag: &'static str,
        _listeners: &[Listener],
        _attributes: &[Attribute],
        _children: &'a [VNode<'a>],
    ) -> VNode<'a> {
        todo!()
    }

    pub fn element(
        &self,
        el: impl DioxusElement,
        listeners: &'a [Listener<'a>],
        attributes: &'a [Attribute<'a>],
        children: &'a [VNode<'a>],
        key: Option<&'a str>,
    ) -> VNode<'a> {
        VNode {
            dom_id: RealDomNode::empty_cell(),
            key,
            kind: VNodeKind::Element(self.bump().alloc(VElement {
                tag_name: el.tag_name(),
                namespace: el.namespace(),
                static_listeners: false,
                listeners,
                static_attrs: false,
                attributes,
                static_children: false,
                children,
            })),
        }
    }

    pub fn suspended() -> VNode<'static> {
        VNode {
            dom_id: RealDomNode::empty_cell(),
            key: None,
            kind: VNodeKind::Suspended,
        }
    }

    pub fn attr(
        &self,
        name: &'static str,
        val: Arguments,
        namespace: Option<&'static str>,
        is_volatile: bool,
    ) -> Attribute<'a> {
        let (value, is_static) = self.raw_text(val);
        Attribute {
            name,
            value,
            is_static,
            namespace,
            is_volatile,
        }
    }

    pub fn virtual_child<P>(
        &self,
        component: FC<P>,
        props: P,
        key: Option<&'a str>, // key: NodeKey<'a>,
        children: &'a [VNode<'a>],
    ) -> VNode<'a>
    where
        P: Properties + 'a,
    {
        // We don't want the fat part of the fat pointer
        // This function does static dispatch so we don't need any VTable stuff
        let props = self.bump().alloc(props);
        let raw_props = props as *const P as *const ();

        let user_fc = component as *const ();

        let comparator: Option<&dyn Fn(&VComponent) -> bool> = Some(self.bump().alloc_with(|| {
            move |other: &VComponent| {
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

        VNode {
            key,
            dom_id: Cell::new(RealDomNode::empty()),
            kind: VNodeKind::Component(self.bump().alloc_with(|| VComponent {
                user_fc,
                comparator,
                raw_props,
                children,
                caller: NodeFactory::create_component_caller(component, raw_props),
                is_static: children.len() == 0 && P::IS_STATIC && key.is_none(),
                ass_scope: Cell::new(None),
            })),
        }
    }

    pub fn create_component_caller<'g, P: 'g>(
        component: FC<P>,
        raw_props: *const (),
    ) -> Rc<dyn for<'r> Fn(&'r Scope) -> VNode<'r>> {
        type Captured<'a> = Rc<dyn for<'r> Fn(&'r Scope) -> VNode<'r> + 'a>;
        let caller: Captured = Rc::new(move |scp: &Scope| -> VNode {
            // cast back into the right lifetime
            let safe_props: &'_ P = unsafe { &*(raw_props as *const P) };
            let tasks = RefCell::new(Vec::new());
            let cx: Context<P> = Context {
                props: safe_props,
                scope: scp,
                tasks: &tasks,
            };

            let res = component(cx);

            // submit any async tasks to the scope
            for _task in tasks.borrow_mut().drain(..) {
                // scp.submit_task(task);
            }

            let g2 = unsafe { std::mem::transmute(res) };

            g2
        });
        unsafe { std::mem::transmute::<_, Captured<'static>>(caller) }
    }

    pub fn fragment_from_iter(
        self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a>>,
    ) -> VNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(self.bump());
        // TODO throw an error if there are nodes without keys
        for node in node_iter.into_iter() {
            nodes.push(node.into_vnode(self));
        }
        VNode {
            dom_id: RealDomNode::empty_cell(),
            key: None,
            kind: VNodeKind::Fragment(VFragment {
                children: nodes.into_bump_slice(),
                is_static: false,
            }),
        }
    }
}

impl<'a> IntoIterator for VNode<'a> {
    type Item = VNode<'a>;
    type IntoIter = std::iter::Once<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}
impl<'a> IntoVNode<'a> for VNode<'a> {
    fn into_vnode(self, _: NodeFactory<'a>) -> VNode<'a> {
        self
    }
}

impl<'a> IntoVNode<'a> for &VNode<'a> {
    fn into_vnode(self, _: NodeFactory<'a>) -> VNode<'a> {
        self.clone()
    }
}

pub trait IntoVNode<'a> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a>;
}

// Wrap the the node-builder closure in a concrete type.
// ---
// This is a bit of a hack to implement the IntoVNode trait for closure types.
pub struct LazyNodes<'a, G>
where
    G: FnOnce(NodeFactory<'a>) -> VNode<'a>,
{
    inner: G,
    _p: PhantomData<&'a ()>,
}

impl<'a, G> LazyNodes<'a, G>
where
    G: FnOnce(NodeFactory<'a>) -> VNode<'a>,
{
    pub fn new(f: G) -> Self {
        Self {
            inner: f,
            _p: PhantomData {},
        }
    }
}

// Cover the cases where nodes are used by macro.
// Likely used directly.
// ---
//  let nodes = rsx!{ ... };
//  rsx! { {nodes } }
impl<'a, G> IntoVNode<'a> for LazyNodes<'a, G>
where
    G: FnOnce(NodeFactory<'a>) -> VNode<'a>,
{
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        (self.inner)(cx)
    }
}

// Required because anything that enters brackets in the rsx! macro needs to implement IntoIterator
impl<'a, G> IntoIterator for LazyNodes<'a, G>
where
    G: FnOnce(NodeFactory<'a>) -> VNode<'a>,
{
    type Item = Self;
    type IntoIter = std::iter::Once<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

impl IntoVNode<'_> for () {
    fn into_vnode<'a>(self, cx: NodeFactory<'a>) -> VNode<'a> {
        cx.fragment_from_iter(None as Option<VNode>)
    }
}

impl IntoVNode<'_> for Option<()> {
    fn into_vnode<'a>(self, cx: NodeFactory<'a>) -> VNode<'a> {
        cx.fragment_from_iter(None as Option<VNode>)
    }
}

impl Debug for NodeFactory<'_> {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

// it's okay to clone because vnodes are just references to places into the bump
impl<'a> Clone for VNode<'a> {
    fn clone(&self) -> Self {
        let kind = match &self.kind {
            VNodeKind::Element(element) => VNodeKind::Element(element),
            VNodeKind::Text(old) => VNodeKind::Text(VText {
                text: old.text,
                is_static: old.is_static,
            }),
            VNodeKind::Fragment(fragment) => VNodeKind::Fragment(VFragment {
                children: fragment.children,
                is_static: fragment.is_static,
            }),
            VNodeKind::Component(component) => VNodeKind::Component(component),
            VNodeKind::Suspended => VNodeKind::Suspended,
        };
        VNode {
            kind,
            dom_id: self.dom_id.clone(),
            key: self.key.clone(),
        }
    }
}

impl Debug for VNode<'_> {
    fn fmt(&self, s: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match &self.kind {
            VNodeKind::Element(el) => write!(s, "element, {}", el.tag_name),
            VNodeKind::Text(t) => write!(s, "text, {}", t.text),
            VNodeKind::Fragment(_) => write!(s, "fragment"),
            VNodeKind::Suspended { .. } => write!(s, "suspended"),
            VNodeKind::Component(_) => write!(s, "component"),
        }
    }
}

mod tests {
    

    static B1: &str = "hello world!";
    static B2: &str = "hello world!";
    #[test]
    fn test() {
        dbg!("Hello world!" as *const _ as *const ());
        dbg!("Hello world!" as *const _ as *const ());
        // dbg!(B1 as *const _ as *const ());
        // dbg!(B2 as *const _ as *const ());
        // goal: elements as const

        // let b = A.clone();
        // A.dom_id.set(RealDomNode::new(10));

        // let p = &A;
        // p.dom_id.set(RealDomNode::new(10));
        // dbg!(p.dom_id.get());

        // dbg!(p as *const _ as *const ());
        // dbg!(&A as *const _ as *const ());

        // // dbg!(b.dom_id.get());
        // dbg!(A.dom_id.get());

        // A.dom_id.set(RealDomNode::empty());
        // let g = A.dom_id.get();
    }

    #[test]
    fn sizing() {
        dbg!(std::mem::size_of::<VElement>());
        dbg!(std::mem::align_of::<VElement>());
    }
}
