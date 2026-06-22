//! Normalizing values into typed child views: the [`IntoViewChild`] trait and
//! the [`dynamic_node`] slot it lowers dynamic nodes through.

use crate::IntoDynNode;

use super::View;

/// Marker for child values that are already typed views.
#[doc(hidden)]
pub struct ViewChildMarker;

#[doc(hidden)]
pub mod dynamic_node {
    use std::marker::PhantomData;

    use crate::view::{View, ViewTemplate};
    use crate::{DynamicValues, IntoDynNode};
    use dioxus_core_template::TemplateRawTree;

    /// Marker for child values that should become dynamic node slots.
    pub struct DynamicViewChildMarker<Marker>(PhantomData<Marker>);

    /// A dynamic node slot.
    pub struct DynamicNodeBuilder<N, Marker = ()> {
        node: N,
        _marker: PhantomData<Marker>,
    }

    /// Create a dynamic node slot from any [`IntoDynNode`] value.
    #[inline]
    pub fn dynamic_node_builder<N, Marker>(node: N) -> DynamicNodeBuilder<N, Marker>
    where
        N: IntoDynNode<Marker>,
    {
        DynamicNodeBuilder {
            node,
            _marker: PhantomData,
        }
    }

    impl<N, Marker> ViewTemplate for DynamicNodeBuilder<N, Marker> {
        const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::DynamicNode;
    }

    impl<N, Marker> View for DynamicNodeBuilder<N, Marker>
    where
        N: IntoDynNode<Marker>,
    {
        fn push(self, dynamic: &mut DynamicValues) {
            dynamic.push_node(self.node.into_dyn_node());
        }
    }
}

/// Convert a value passed to [`ElementBuilder::child`](super::ElementBuilder::child) into a typed
/// child view.
pub trait IntoViewChild<Marker = ViewChildMarker> {
    /// The typed view contributed by this child.
    type Output: View;

    /// Convert into the child view.
    fn into_child(self) -> Self::Output;
}

impl<V: View> IntoViewChild<ViewChildMarker> for V {
    type Output = V;

    #[inline]
    fn into_child(self) -> Self::Output {
        self
    }
}

impl<N, Marker> IntoViewChild<dynamic_node::DynamicViewChildMarker<Marker>> for N
where
    N: IntoDynNode<Marker>,
{
    type Output = dynamic_node::DynamicNodeBuilder<N, Marker>;

    #[inline]
    fn into_child(self) -> Self::Output {
        dynamic_node::dynamic_node_builder(self)
    }
}
