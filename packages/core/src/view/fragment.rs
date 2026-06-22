//! The typed fragment view: an ordered group of children with no enclosing
//! element.

use crate::DynamicValues;
use dioxus_core_template::TemplateRawTree;

use super::{IntoViewChild, View, ViewTemplate};

/// A typed fragment view: an ordered group of children with no enclosing element.
///
/// Like [`ElementBuilder`](super::ElementBuilder) it collects children into a cons list via
/// [`FragmentBuilder::child`], but it contributes no node of its own — it lowers to exactly its
/// children, in order. That makes it the container for a template's roots and for grouping more
/// siblings than a single tuple can hold: `View`/`ViewTemplate` are only implemented for tuples up
/// to arity 64, so wider lists are split into several ≤64-wide tuples joined through `.child(..)`.
/// Nested fragments and tuples flatten transparently into the surrounding template, so the grouping
/// leaves no trace in the lowered ops or dynamic-slot order.
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

    /// Replace the children with an already-normalized typed view.
    #[inline]
    pub fn with_children<NewChildren>(self, children: NewChildren) -> FragmentBuilder<NewChildren> {
        FragmentBuilder { children }
    }
}

impl<Children: ViewTemplate> ViewTemplate for FragmentBuilder<Children> {
    const TEMPLATE_TREE: &'static TemplateRawTree = Children::TEMPLATE_TREE;
}

impl<Children: View> View for FragmentBuilder<Children> {
    #[inline]
    fn push(self, dynamic: &mut DynamicValues) {
        self.children.push(dynamic);
    }
}
