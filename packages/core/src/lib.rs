#![allow(non_snake_case)]
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

pub(crate) mod diff;
pub(crate) mod events;
pub(crate) mod lazynodes;
pub(crate) mod mutations;
pub(crate) mod nodes;
pub(crate) mod properties;
pub(crate) mod scopes;
pub(crate) mod virtual_dom;

pub(crate) mod innerlude {
    pub use crate::events::*;
    pub use crate::lazynodes::*;
    pub use crate::mutations::*;
    pub use crate::nodes::*;
    pub use crate::properties::*;
    pub use crate::scopes::*;
    pub use crate::virtual_dom::*;

    /// An [`Element`] is a possibly-none [`VNode`] created by calling `render` on [`Scope`] or [`ScopeState`].
    ///
    /// Any [`None`] [`Element`] will automatically be coerced into a placeholder [`VNode`] with the [`VNode::Placeholder`] variant.
    pub type Element<'a> = Option<VNode<'a>>;

    /// A [`Component`] is a function that takes a [`Scope`] and returns an [`Element`].
    ///
    /// Components can be used in other components with two syntax options:
    /// - lowercase as a function call with named arguments (rust style)
    /// - uppercase as an element (react style)
    ///
    /// ## Rust-Style
    ///
    /// ```rust, ignore
    /// fn example(cx: Scope<Props>) -> Element {
    ///     // ...
    /// }
    ///
    /// rsx!(
    ///     example()
    /// )
    /// ```
    /// ## React-Style
    /// ```rust, ignore
    /// fn Example(cx: Scope<Props>) -> Element {
    ///     // ...
    /// }
    ///
    /// rsx!(
    ///     Example {}
    /// )
    /// ```
    pub type Component<P = ()> = fn(Scope<P>) -> Element;

    /// A list of attributes
    pub type Attributes<'a> = Option<&'a [Attribute<'a>]>;
}

pub use crate::innerlude::{
    AnyEvent, ArbitraryAttributeValue, Attribute, AttributeValue, Component, Element, ElementId,
    EventHandler, EventPriority, IntoVNode, LazyNodes, Listener, NodeFactory, Properties, Renderer,
    SchedulerMsg, Scope, ScopeId, ScopeState, TaskId, Template, TemplateAttribute, TemplateNode,
    UiEvent, UserEvent, VComponent, VElement, VNode, VTemplate, VText, VirtualDom,
};

/// The purpose of this module is to alleviate imports of many common types
///
/// This includes types like [`Scope`], [`Element`], and [`Component`].
pub mod prelude {
    pub use crate::innerlude::{
        fc_to_builder, Attributes, Component, Element, EventHandler, LazyNodes, NodeFactory,
        Properties, Scope, ScopeId, ScopeState, Template, VNode, VirtualDom,
    };
}

pub mod exports {
    //! Important dependencies that are used by the rest of the library
    //! Feel free to just add the dependencies in your own Crates.toml
    pub use bumpalo;
    pub use futures_channel;
}

/// Functions that wrap unsafe functionality to prevent us from misusing it at the callsite
pub(crate) mod unsafe_utils {
    use crate::VNode;

    pub(crate) unsafe fn extend_vnode<'a, 'b>(node: &'a VNode<'a>) -> &'b VNode<'b> {
        std::mem::transmute(node)
    }
}

#[macro_export]
/// A helper macro for using hooks in async environements.
///
/// # Usage
///
///
/// ```ignore
/// let (data) = use_ref(&cx, || {});
///
/// let handle_thing = move |_| {
///     to_owned![data]
///     cx.spawn(async move {
///         // do stuff
///     });
/// }
/// ```
macro_rules! to_owned {
    ($($es:ident),+) => {$(
        #[allow(unused_mut)]
        let mut $es = $es.to_owned();
    )*}
}

/// get the code location of the code that called this function
#[macro_export]
macro_rules! get_line_num {
    () => {
        concat!(
            file!(),
            ":",
            line!(),
            ":",
            column!(),
            ":",
            env!("CARGO_MANIFEST_DIR")
        )
    };
}
