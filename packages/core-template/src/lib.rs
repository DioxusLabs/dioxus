//! Static template storage for Dioxus views.
//!
//! The `rsx!` macro lowers each view into two pieces:
//!
//! - a static [`Template`] describing the stable element, text, and static
//!   attribute structure
//! - a runtime `DynamicValue` array containing dynamic nodes and dynamic
//!   attribute groups for one render
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
//! emitted anchor needs to point to a static node or static parent. The path
//! notation below is a tree walk: `0` means "next child", and `1` means "next
//! sibling". The empty path is the vnode render-parent site; it is not a real
//! static node.
//!
//! This table includes only paths that are actually emitted in anchor slot
//! targets for the example above:
//!
//! ```text
//! Path walk    Slot target          Why it is emitted
//! 0            BeforeStatic(0)      root dynamic node before div
//! 001          AppendChildren(001)  dynamic text inside strong
//! 0011         BeforeStatic(0011)   badge and count before span
//! 0            AppendChildren(0)    dynamic class attrs on div
//! []           AppendChildren([])   trailing root dynamic node
//! ```
//!
//! There is no emitted path for `"Hello "` because no dynamic slot is anchored
//! to that static text node. The same path can appear with different target
//! kinds: `BeforeStatic(0)` inserts before `div`, while `AppendChildren(0)`
//! appends dynamic attributes to `div`.
//!
//! `BeforeStatic(path)` means the dynamic slot is inserted before the static
//! node at `path`. The diff uses that path to find the parent and insertion
//! index. Dynamic slots at the end of a parent use
//! `AppendChildren(parent_path)`.
//!
//! Anchors are created for dynamic value groups, not for every static node.
//! A static element appears as an anchor parent only when it owns dynamic
//! attributes or a direct dynamic child insertion position. Root-level dynamic
//! nodes use `None` as their parent instead.
//!
//! Dynamic nodes and dynamic attributes are pushed into one flat runtime value
//! array as the typed view renders. Element children are pushed before that
//! element's dynamic attributes, so the example above produces values like this:
//!
//! ```text
//! Value index    Runtime value
//! 0              Node(before)
//! 1              Node(text from "{name}")
//! 2              Node(badge)
//! 3              Node(text from "{count}")
//! 4              Attrs(class from "{class_name}")
//! 5              Node(after)
//! ```
//!
//! Anchors map ranges from that value array to insertion targets. A target is
//! either `BeforeStatic(path)` for values that should be inserted before a
//! following static node, or `AppendChildren(path)` for values that should be
//! appended to a static parent. `AppendChildren([])` means append at the vnode
//! root site.
//!
//! ```text
//! Values    Parent element op    Slot target          Meaning
//! 0..1      None                 BeforeStatic(0)      root node before div
//! 1..2      strong               AppendChildren(001)  "{name}" inside strong
//! 2..4      div                  BeforeStatic(0011)   badge and count before span
//! 4..5      div                  AppendChildren(0)    dynamic class attrs for div
//! 5..6      None                 AppendChildren([])   trailing root node
//! ```
//!
//! Adjacent dynamic nodes at the same insertion position share one anchor. In
//! the example, `{badge}` and `"{count}"` are both before `span`, so they are
//! represented by the single `2..4` range.
//!
//! The stored anchor slice is sorted for native fill order rather than source
//! order: deeper anchors are filled first, and ties are filled from later
//! dynamic values to earlier dynamic values. Code that needs source/value order
//! can use [`Template::anchors_in_document_order`].
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
pub use data::Template;
pub use op::{DecodedTemplateAttrNamespace, DecodedTemplateOp, TemplateOp};
pub use path::{TemplatePath, TemplateSlotPath, TemplateSlotTarget};
pub use raw::TemplateRawTree;
#[cfg(feature = "serialize")]
#[doc(hidden)]
pub use serialization::{
    deserialize_leaky, deserialize_option_leaky, deserialize_string_leaky,
    deserialize_strings_leaky,
};
pub use storage::TemplateStorageStats;
#[doc(hidden)]
pub use storage::{
    RuntimeTemplateBuilder, TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_MAX_CAP,
    TEMPLATE_STORAGE_OPS_CAP, TEMPLATE_STORAGE_STRING_CAP, TemplateStatsBuilder, TemplateStorage,
};
