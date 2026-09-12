//! The core view abstraction: [`ViewTemplate`], [`View`], and the extension
//! traits and helpers that turn a typed view into a [`VNode`].

use crate::{DynamicValues, Template, VNode};
use dioxus_core_template::{
    TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_OPS_CAP, TEMPLATE_STORAGE_STRING_CAP,
    TemplateRawTree, TemplateStorage,
};
#[cfg(debug_assertions)]
use std::collections::HashMap;
use std::marker::PhantomData;
#[cfg(debug_assertions)]
use std::sync::RwLock;

/// A type that contributes static template structure.
pub trait ViewTemplate {
    /// The static tree for this view type.
    const TEMPLATE_TREE: &'static TemplateRawTree;

    /// Whether [`View::push`] on this view contributes any runtime dynamic nodes or attributes.
    ///
    /// Composite views only call [`View::push`] on parts where this is `true`, so fully static
    /// subtrees never instantiate `push` at all. Defaults to `true`, which is always sound.
    const HAS_DYNAMIC: bool = true;
}

struct StaticViewTemplate<
    V,
    const OPS_CAP: usize,
    const STRING_CAP: usize,
    const DYNAMIC_CAP: usize,
>(PhantomData<fn() -> V>);

impl<V: ViewTemplate, const OPS_CAP: usize, const STRING_CAP: usize, const DYNAMIC_CAP: usize>
    StaticViewTemplate<V, OPS_CAP, STRING_CAP, DYNAMIC_CAP>
{
    const TEMPLATE: &'static Template =
        &TemplateStorage::<OPS_CAP, STRING_CAP, DYNAMIC_CAP>::build_from_tree(V::TEMPLATE_TREE)
            .as_template();
}

impl ViewTemplate for () {
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::Empty;
    const HAS_DYNAMIC: bool = false;
}

/// A typed view that can collect runtime dynamic nodes and attributes.
pub trait View: ViewTemplate + Sized {
    /// Push runtime dynamic nodes and attributes in template order.
    #[inline]
    fn push(self, _: &mut DynamicValues) {}
}

/// A typed view with a root key.
pub struct KeyedView<V> {
    key: Option<String>,
    view: V,
}

impl<V: ViewTemplate> ViewTemplate for KeyedView<V> {
    const TEMPLATE_TREE: &'static TemplateRawTree = V::TEMPLATE_TREE;
}

impl<V: View> View for KeyedView<V> {
    #[inline]
    fn push(self, dynamic: &mut DynamicValues) {
        dynamic.set_key(self.key);
        if V::HAS_DYNAMIC {
            self.view.push(dynamic);
        }
    }
}

/// Extension methods for assigning a root key to typed views.
pub trait ViewKeyExt: View {
    /// Assign a root key to this view.
    fn key(self, key: Option<String>) -> KeyedView<Self>;
}

impl<V: View> ViewKeyExt for V {
    #[inline]
    fn key(self, key: Option<String>) -> KeyedView<Self> {
        KeyedView { key, view: self }
    }
}

/// Extension methods for typed views.
pub trait ViewExt: View {
    /// Convert this view into a [`VNode`].
    fn into_vnode(self) -> VNode;
}

impl<V: View> ViewExt for V {
    #[inline]
    fn into_vnode(self) -> VNode {
        into_vnode_with_template(
            self,
            StaticViewTemplate::<
                V,
                TEMPLATE_STORAGE_OPS_CAP,
                TEMPLATE_STORAGE_STRING_CAP,
                TEMPLATE_STORAGE_DYNAMIC_CAP,
            >::TEMPLATE,
        )
    }
}

/// Convert a view into a [`VNode`] using a prepared template.
#[inline]
fn into_vnode_with_template<V: View>(view: V, template: &Template) -> VNode {
    into_vnode_with_template_and_values(view, DynamicValues::new(), template)
}

/// Convert a view into a [`VNode`] using a prepared template, pushing its runtime values onto
/// `dynamic` (which already holds any values the caller filled in ahead of the view).
#[inline]
fn into_vnode_with_template_and_values<V: View>(
    view: V,
    mut dynamic: DynamicValues,
    template: &Template,
) -> VNode {
    if V::HAS_DYNAMIC {
        view.push(&mut dynamic);
    }
    VNode::new(*template, dynamic)
}

/// Runtime values for an `rsx!` body, sized for its dynamic node and attribute counts.
///
/// `rsx!` pushes the body's dynamic node and attribute values into this (see
/// [`push_dyn_node`](super::push_dyn_node) and [`push_dyn_attrs`](super::push_dyn_attrs)) before
/// building the view, then hands both to [`vnode_from_tree`] / [`vnode_with_capacity`].
#[doc(hidden)]
#[inline]
pub fn dynamic_values(dynamic_nodes: usize, dynamic_attributes: usize) -> DynamicValues {
    DynamicValues::with_capacity(dynamic_nodes, dynamic_attributes)
}

/// Set the root key of an `rsx!` body whose values are pushed ahead of the view.
#[doc(hidden)]
#[inline]
pub fn set_key(dynamic: &mut DynamicValues, key: Option<String>) {
    dynamic.set_key(key);
}

/// The static template tree of a view value's type.
///
/// This is the only per-site generic code the debug `rsx!` expansion instantiates; everything
/// else runs through the non-generic [`vnode_from_tree`].
#[doc(hidden)]
#[inline]
pub fn template_tree<V: ViewTemplate>(_: &V) -> &'static TemplateRawTree {
    V::TEMPLATE_TREE
}

/// Build a [`VNode`] for a view type whose runtime values were all pushed onto `dynamic` ahead
/// of the view, using template capacities resolved at the call site.
///
/// The view value only fixes `V`; nothing is pushed from it, so no [`View::push`] is
/// instantiated. Release `rsx!` expansions use this; debug ones use [`vnode_from_tree`].
#[doc(hidden)]
#[inline]
pub fn vnode_with_capacity<
    const OPS_CAP: usize,
    const STRING_CAP: usize,
    const DYNAMIC_CAP: usize,
    V: ViewTemplate,
>(
    _: &V,
    dynamic: DynamicValues,
) -> VNode {
    VNode::new(
        *StaticViewTemplate::<V, OPS_CAP, STRING_CAP, DYNAMIC_CAP>::TEMPLATE,
        dynamic,
    )
}

/// Build a [`VNode`] from a debug-only lazy template cached per raw tree.
///
/// In dev builds the optimized template is lowered once at runtime from the view's
/// [`ViewTemplate::TEMPLATE_TREE`] (skipping the per-`rsx!`-site const evaluation that dominates
/// debug compile time) and cached by the tree's address, so a site needs no `static` of its own.
/// Release builds use [`vnode_with_capacity`] and its const template instead.
#[cfg(debug_assertions)]
#[doc(hidden)]
#[inline(never)]
pub fn vnode_from_tree(tree: &'static TemplateRawTree, dynamic: DynamicValues) -> VNode {
    VNode::new(runtime_template(tree), dynamic)
}

/// The runtime-lowered template for `tree`, lowered on first use and cached by address.
///
/// Distinct trees never share an address, and lowering an identical tree reached through two
/// addresses only costs a duplicate cache entry, so the address is a sound key.
#[cfg(debug_assertions)]
fn runtime_template(tree: &'static TemplateRawTree) -> Template {
    static TEMPLATES: RwLock<Option<HashMap<usize, Template>>> = RwLock::new(None);

    let key = tree as *const TemplateRawTree as usize;
    let cached = TEMPLATES
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .and_then(|templates| templates.get(&key).copied());
    if let Some(template) = cached {
        return template;
    }

    let mut templates = TEMPLATES
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *templates
        .get_or_insert_default()
        .entry(key)
        .or_insert_with(|| dioxus_core_template::build_runtime_template(tree))
}

impl View for () {}
