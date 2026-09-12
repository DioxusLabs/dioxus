//! The typed fragment view: an ordered group of children with no enclosing
//! element.

use crate::DynamicValues;
use dioxus_core_template::TemplateRawTree;

use super::{IntoViewChild, View, ViewTemplate};

/// A typed fragment view: an ordered group of children with no enclosing element.
///
/// Like [`ElementBuilder`](super::ElementBuilder) it collects children into a cons list via
/// [`FragmentBuilder::child`], but it contributes no node of its own - it lowers to exactly its
/// children, in order. Nested fragments and tuples flatten transparently into the surrounding
/// template, so grouping leaves no trace in the lowered ops or dynamic-slot order.
pub struct FragmentBuilder<Children> {
    children: Children,
}

/// Create an empty typed fragment.
#[inline]
pub const fn fragment() -> FragmentBuilder<()> {
    FragmentBuilder { children: () }
}

impl<Children> FragmentBuilder<Children> {
    /// Append one child.
    #[inline]
    pub fn child<Child, Marker>(
        self,
        child: Child,
    ) -> FragmentBuilder<(Children, <Child as IntoViewChild<Marker>>::Output)>
    where
        Child: IntoViewChild<Marker>,
    {
        FragmentBuilder {
            children: (self.children, child.into_child()),
        }
    }
}

impl<Children: ViewTemplate> ViewTemplate for FragmentBuilder<Children> {
    const TEMPLATE_TREE: &'static TemplateRawTree = Children::TEMPLATE_TREE;
    const HAS_DYNAMIC: bool = Children::HAS_DYNAMIC;
}

impl<Children: View> View for FragmentBuilder<Children> {
    #[inline]
    fn push(self, dynamic: &mut DynamicValues) {
        if Children::HAS_DYNAMIC {
            self.children.push(dynamic);
        }
    }
}
