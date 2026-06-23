//! Static template storage for Dioxus views.
//!
//! The `rsx!` macro lowers each view into two pieces:
//!
//! - a static [`Template`] describing the stable element, text, and static
//!   attribute structure
//! - runtime dynamic node and dynamic attribute arrays for one render
//!
//! The template uses compact paths and anchors to connect those runtime values
//! back to stable locations in the static tree.
//!
//! ## Annotated RSX lowering
//!
//! Consider this `rsx!` block:
//!
//! ```rust,ignore
//! rsx! {
//!     {before}
//!     div { class: "{class_name}",
//!         "Hello "
//!         strong { "{name}" }
//!         {badge}
//!         "{count}"
//!         span { "!" }
//!     }
//!     {after}
//! }
//! ```
//!
//! Dynamic nodes do not receive their own paths. Paths are only stored when an
//! emitted anchor needs to point to a static node. The path
//! notation below is a tree walk: `0` means "next child", and `1` means "next
//! sibling". The empty path is the vnode render-parent site; it is not a real
//! static node.
//!
//! This table includes only paths that are actually emitted in anchor slot
//! targets for the example above:
//!
//! ```text
//! Path walk    Last static node?    Why it is emitted
//! 0            false                root anchor for div / node before div
//! 0            false                dynamic class attrs on div
//! 001          true                 dynamic text inside strong
//! 0011         false                badge and count before span
//! 0            true                 trailing root dynamic node after div
//! ```
//!
//! There is no emitted path for `"Hello "` because no dynamic slot is anchored
//! to that static text node. The same path can appear with different parent
//! ownership: a root-level anchor at `0` can insert before `div`, while an
//! element-owned anchor at `0` applies dynamic attributes to `div`.
//!
//! The low tag bit on an anchor path records whether the path is the last
//! static node before the dynamic slot. When it is false, dynamic nodes insert
//! before the static node at `path`. When it is true, dynamic nodes insert
//! after that static node, or append to it when the path is also the anchor's
//! owning element.
//!
//! Anchors are created for dynamic node or attribute groups, not for every static node.
//! A static element appears as an anchor parent only when it owns dynamic
//! attributes or a direct dynamic child insertion position. Root-level dynamic
//! nodes use `None` as their parent instead.
//!
//! Dynamic nodes and dynamic attributes are pushed into separate runtime arrays
//! as the typed view renders. An element's dynamic attributes are pushed before
//! its dynamic children, so the example above produces values like this:
//!
//! ```text
//! Node index    Runtime node
//! 0             before
//! 1             text from "{name}"
//! 2             badge
//! 3             text from "{count}"
//! 4             after
//!
//! Attr index    Runtime attrs
//! 0             class from "{class_name}"
//! ```
//!
//! Anchors map ranges from those arrays to static paths plus the last-static
//! flag. A root static node always has an anchor, even when it owns no dynamic
//! values, so renderers can use the same anchor list for hydration and template
//! cloning.
//!
//! ```text
//! Nodes    Attrs    Parent element op    Path    Last?    Meaning
//! 0..1     0..0     None                 0       false    root node before div
//! 1..1     0..1     div                  0       false    dynamic class attrs for div
//! 1..2     1..1     strong               001     true     "{name}" inside strong
//! 2..4     1..1     div                  0011    false    badge and count before span
//! 4..5     1..1     None                 0       true     trailing root node
//! ```
//!
//! Adjacent dynamic nodes at the same insertion position share one anchor. In
//! the example, `{badge}` and `"{count}"` are both before `span`, so they are
//! represented by the single `2..4` node range.
//!
//! The stored anchor slice is in source/template order.
//!
mod anchor;
mod data;
mod op;
mod path;
mod raw;
#[cfg(feature = "serialize")]
mod serialization;
mod storage;

pub use anchor::TemplateAnchor;
pub use data::{
    StaticTemplateAttribute, StaticTemplateAttributeIter, StaticTemplateElement,
    StaticTemplateNode, StaticTemplateNodeIter, StaticTemplateText, Template,
};
pub use op::DecodedTemplateOp;
pub(crate) use path::TemplateSlotPath;
pub use path::{TemplatePath, TemplateSlotTarget};
pub use raw::TemplateRawTree;
#[cfg(feature = "serialize")]
pub use serialization::{deserialize_option_leaky, deserialize_string_leaky};
pub use storage::TemplateStorageStats;
pub use storage::{
    RuntimeTemplateBuilder, TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_MAX_CAP,
    TEMPLATE_STORAGE_OPS_CAP, TEMPLATE_STORAGE_STRING_CAP, TemplateStatsBuilder, TemplateStorage,
};
#[cfg(debug_assertions)]
pub use storage::build_runtime_template;
