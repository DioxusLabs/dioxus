//! The core view abstraction: [`ViewTemplate`], [`View`], and the extension
//! traits and helpers that turn a typed view into a [`VNode`].

use crate::{DynamicValues, Template, VNode};
use dioxus_core_template::{
    TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_OPS_CAP, TEMPLATE_STORAGE_STRING_CAP,
    TemplateRawTree, TemplateStorage,
};

/// A type that contributes static template structure.
pub trait ViewTemplate {
    /// The static tree for this view type.
    const TEMPLATE_TREE: &'static TemplateRawTree;
}

/// Builds the static [`Template`] for a view using template storage capacities resolved at the
/// call site.
pub trait StaticViewTemplateWithCapacity<
    const OPS_CAP: usize,
    const STRING_CAP: usize,
    const DYNAMIC_CAP: usize,
>: ViewTemplate
{
    /// The static template for this view type with the given storage capacities.
    const TEMPLATE: &'static Template;
}

impl<T: ViewTemplate, const OPS_CAP: usize, const STRING_CAP: usize, const DYNAMIC_CAP: usize>
    StaticViewTemplateWithCapacity<OPS_CAP, STRING_CAP, DYNAMIC_CAP> for T
{
    const TEMPLATE: &'static Template =
        &TemplateStorage::<OPS_CAP, STRING_CAP, DYNAMIC_CAP>::build_from_tree(T::TEMPLATE_TREE)
            .as_template();
}

impl ViewTemplate for () {
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::Empty;
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
        self.view.push(dynamic);
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
            <V as StaticViewTemplateWithCapacity<
                TEMPLATE_STORAGE_OPS_CAP,
                TEMPLATE_STORAGE_STRING_CAP,
                TEMPLATE_STORAGE_DYNAMIC_CAP,
            >>::TEMPLATE,
        )
    }
}

/// Convert a view into a [`VNode`] using a prepared template.
#[inline]
fn into_vnode_with_template<V: View>(view: V, template: &Template) -> VNode {
    let mut dynamic = DynamicValues::new();
    view.push(&mut dynamic);
    VNode::new(*template, dynamic)
}

/// Convert a view into a [`VNode`] using a debug-only lazy template cached per call site.
///
/// In dev builds the optimized template is lowered once at runtime from the view's
/// [`ViewTemplate::TEMPLATE_TREE`] (skipping the per-`rsx!`-site const evaluation that dominates
/// debug compile time) and cached in `cache`. Release builds use [`into_vnode_with_capacity`] and
/// its const template instead.
#[cfg(debug_assertions)]
#[doc(hidden)]
pub fn into_vnode_cached<V: View>(view: V, cache: &std::sync::OnceLock<Template>) -> VNode {
    let template =
        *cache.get_or_init(|| dioxus_core_template::build_runtime_template(V::TEMPLATE_TREE));
    into_vnode_with_template(view, &template)
}

/// Convert a view into a [`VNode`] using template capacities resolved at the call site.
pub fn into_vnode_with_capacity<
    const OPS_CAP: usize,
    const STRING_CAP: usize,
    const DYNAMIC_CAP: usize,
    V: View + StaticViewTemplateWithCapacity<OPS_CAP, STRING_CAP, DYNAMIC_CAP>,
>(
    view: V,
) -> VNode {
    into_vnode_with_template(
        view,
        <V as StaticViewTemplateWithCapacity<OPS_CAP, STRING_CAP, DYNAMIC_CAP>>::TEMPLATE,
    )
}

impl View for () {}
