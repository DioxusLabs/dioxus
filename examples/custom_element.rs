//! This example shows to wrap a webcomponent / custom element with a component.
//!
//! Oftentimes, a third party library will provide a webcomponent that you want
//! to use in your application. This example shows how to create that custom element
//! directly with the raw_element method on NodeFactory.
#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types)]

use std::marker::PhantomData;

// use dioxus::prelude::*;
use dioxus::core::{Element, Scope, ScopeState, VirtualDom};

fn main() {}

// pub struct StringlyAttr {
//     name: &'static str,
//     namespace: Option<&'static str>,
//     is_boolean: bool,
// }

// impl StringlyAttr {
//     pub const fn new(
//         name: &'static str,
//         namespace: Option<&'static str>,
//         is_boolean: bool,
//     ) -> Self {
//         Self {
//             name,
//             namespace,
//             is_boolean,
//         }
//     }
// }

// pub trait AttributeTransform {}
// impl AttributeTransform for () {}

// macro_rules! custom_elements {
//     (
//         // doc comment
//         $( #[doc = $doc:expr] )*
//         $trait_def:ident;

//         $(
//             $element:ident {
//                 $(
//                     $attr:ident: $attr_type:ty,
//                 )*
//             },
//         )*
//     ) => {
//         $( #[doc = $doc:expr] )*
//         pub trait $trait_def {
//             $(
//                 const $element: elements::$element = elements::$element;
//             )*
//         }

//         mod elements {
//             use super::*;
//             $(
//                 pub struct $element;

//                 impl $element {
//                     pub const fn name(&self) -> &'static str {
//                         stringify!($element)
//                     }

//                     pub const fn namespace(&self) -> Option<&'static str> {
//                         None
//                     }

//                     pub const fn volatile(&self) -> bool {
//                         false
//                     }

//                     $(
//                         pub const fn $attr(&self) -> $attr_type {
//                             todo!()
//                             // $attr_type::new(stringify!($attr), None, false)
//                         }
//                     )*
//                 }
//             )*
//         }
//     };
// }

// impl HtmlElements for ScopeState {}

// custom_elements! {
//     HtmlElements;

//     // link {
//     //     crossorigin: StringlyAttr,
//     //     href: StringlyAttr,
//     //     hreflang: StringlyAttr,
//     //     integrity: StringlyAttr,
//     // },
// }

// struct RawElement<N, E = ()> {
//     tag: &'static str,
//     namespace: Option<&'static str>,
//     volatile: bool,
//     _t: PhantomData<(N, E)>,
// }

// struct HtmlNamespace;
// type HtmlElement<T = ()> = RawElement<HtmlNamespace, T>;

// impl<N, E> RawElement<N, E> {
//     const fn new(tag: &'static str, namespace: Option<&'static str>, volatile: bool) -> Self {
//         Self {
//             tag,
//             namespace,
//             volatile,
//             _t: PhantomData,
//         }
//     }
// }

// impl<T> HtmlElement<T> {
//     const fn class(&self) -> StringlyAttr {
//         StringlyAttr::new("class", None, false)
//     }

//     const fn name(&self) -> &'static str {
//         self.tag
//     }
// }

// struct link;
// impl RawElement<link> {
//     const fn div_id(&self) -> StringlyAttr {
//         StringlyAttr::new("id", None, false)
//     }
// }

// fn component(cx: Scope) -> Element {
//     // ScopeState::link.name();
//     // ScopeState::link.namespace();
//     // ScopeState::link.crossorigin();

//     todo!()
// }

// impl CustomElements for ScopeState {}
// trait CustomElements {
//     const div: HtmlElement = HtmlElement::new("div", None, false);
//     const link: HtmlElement<link> = HtmlElement::new("link", None, false);
// }

// use dioxus::core::{TemplateAttribute, TemplateNode};

// static TEMPLATE_EL: TemplateNode = TemplateNode::Element {
//     tag: ScopeState::link.tag,
//     namespace: ScopeState::link.namespace,
//     attrs: &[
//         TemplateAttribute::Static {
//             name: ScopeState::link.class().name,
//             namespace: ScopeState::link.class().namespace,
//             value: "asd",
//         },
//         TemplateAttribute::Dynamic { id: 0 },
//     ],
//     children: &[],
// };
