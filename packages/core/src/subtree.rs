/*
This is a WIP module

Subtrees allow the virtualdom to split up the mutation stream into smaller chunks which can be directed to different parts of the dom.
It's core to implementing multiwindow desktop support, portals, and alternative inline renderers like react-three-fiber.

The primary idea is to give each renderer a linear element tree managed by Dioxus to maximize performance and minimize memory usage.
This can't be done if two renderers need to share the same native tree.
With subtrees, we have an entirely different slab of elements

*/

use std::borrow::Cow;

use slab::Slab;

use crate::{ElementPath, ScopeId};

/// A collection of elements confined to a single scope under a chunk of the tree
///
/// All elements in this collection are guaranteed to be in the same scope and share the same numbering
///
/// This unit can be multithreaded
/// Whenever multiple subtrees are present, we can perform **parallel diffing**
pub struct Subtree {
    id: usize,
    namespace: Cow<'static, str>,
    root: ScopeId,
    elements: Slab<ElementPath>,
}
