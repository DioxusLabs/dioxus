//! Example: SSR
//!
//! This example shows how we can render the Dioxus Virtualdom using SSR.

use dioxus::core::{Attribute, HasAttributesBox};
use dioxus::html::{AudioExtension, ExtendedAudioMarker, ExtendedGlobalAttributesMarker};
use dioxus::prelude::*;

fn main() {
    // We can render VirtualDoms
    let mut vdom = VirtualDom::new(app);
    let _ = vdom.rebuild();
    println!("{}", dioxus_ssr::render(&vdom));

    // Or we can render rsx! calls themselves
    println!(
        "{}",
        dioxus_ssr::render_lazy(rsx! {
            div {
                h1 { "Hello, world!" }
            }
        })
    );

    // We can configure the SSR rendering to add ids for rehydration
    println!("{}", dioxus_ssr::pre_render(&vdom));

    // We can render to a buf directly too
    let mut file = String::new();
    let mut renderer = dioxus_ssr::Renderer::default();
    renderer.render_to(&mut file, &vdom).unwrap();
    println!("{file}");
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(Foo {
        autoplay: true,
        controls: true,
    }))
}

pub struct FooProps<'a> {
    pub open: Option<&'a str>,
    attributes: Vec<Attribute<'a>>,
}

// -----
impl<'a> FooProps<'a> {
    #[doc = "\nCreate a builder for building `FooProps`.\nOn the builder, call `.open(...)`(optional) to set the values of the fields.\nFinally, call `.build()` to create the instance of `FooProps`.\n                    "]
    #[allow(dead_code)]
    pub fn builder(cx: &'a ScopeState) -> FooPropsBuilder<'a, ((),)> {
        FooPropsBuilder {
            bump: cx.bump(),
            fields: ((),),
            attributes: Vec::new(),
            _phantom: core::default::Default::default(),
        }
    }
}
#[must_use]
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub struct FooPropsBuilder<'a, TypedBuilderFields> {
    bump: &'a ::dioxus::core::exports::bumpalo::Bump,
    fields: TypedBuilderFields,
    attributes: Vec<Attribute<'a>>,
    _phantom: (core::marker::PhantomData<&'a ()>),
}
//impl<'a, TypedBuilderFields, > Clone for FooPropsBuilder<'a, TypedBuilderFields, > where TypedBuilderFields: Clone { fn clone(&self) -> Self { Self { fields: self.fields.clone(), attributes: self.attributes, _phantom: Default::default() } } }
impl<'a> dioxus::prelude::Properties<'a> for FooProps<'a> {
    type Builder = FooPropsBuilder<'a, ((),)>;
    const IS_STATIC: bool = false;
    fn builder(cx: &'a ScopeState) -> Self::Builder {
        FooProps::builder(cx)
    }
    unsafe fn memoize(&self, other: &Self) -> bool {
        false
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub trait FooPropsBuilder_Optional<T> {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T;
}
impl<T> FooPropsBuilder_Optional<T> for () {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T {
        default()
    }
}
impl<T> FooPropsBuilder_Optional<T> for (T,) {
    fn into_value<F: FnOnce() -> T>(self, _: F) -> T {
        self.0
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<'a> FooPropsBuilder<'a, ((),)> {
    pub fn open(
        self,
        open: &'a str,
    ) -> FooPropsBuilder<
        'a,
        ((
            Option<&'a str>,
            // pub attributes: Vec<Attribute<'a>>,
        ),),
    > {
        let open = (Some(open),);
        let (_,) = self.fields;
        FooPropsBuilder {
            bump: self.bump,
            fields: (open,),
            attributes: self.attributes,
            _phantom: self._phantom,
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum FooPropsBuilder_Error_Repeated_field_open {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<'a>
    FooPropsBuilder<
        'a,
        ((
            Option<&'a str>,
            // pub attributes: Vec<Attribute<'a>>,
        ),),
    >
{
    #[deprecated(note = "Repeated field open")]
    pub fn open(
        self,
        _: FooPropsBuilder_Error_Repeated_field_open,
    ) -> FooPropsBuilder<
        'a,
        ((
            Option<&'a str>,
            // pub attributes: Vec<Attribute<'a>>,
        ),),
    > {
        self
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<'a, __open: FooPropsBuilder_Optional<Option<&'a str>>> FooPropsBuilder<'a, (__open,)> {
    pub fn build(self) -> FooProps<'a> {
        let (open,) = self.fields;
        let open = FooPropsBuilder_Optional::into_value(open, || Default::default());
        FooProps {
            open,
            attributes: self.attributes,
        }
    }
}
// -----

impl<'a, A> HasAttributesBox<'a, FooPropsBuilder<'a, (A,)>> for FooPropsBuilder<'a, (A,)> {
    fn push_attribute(
        self,
        name: &'a str,
        ns: Option<&'static str>,
        attr: impl IntoAttributeValue<'a>,
        volatile: bool,
    ) -> Self {
        let mut attrs = self.attributes;
        // We insert attributes so that the list is binary-searchable
        if let Err(index) = attrs.binary_search_by(|probe| probe.name.cmp(name)) {
            attrs.insert(
                index,
                Attribute::new(name, attr.into_value(self.bump), ns, volatile),
            );
        }
        FooPropsBuilder {
            bump: self.bump,
            fields: self.fields,
            attributes: attrs,
            _phantom: self._phantom,
        }
    }
}
impl<A> ExtendedGlobalAttributesMarker for FooPropsBuilder<'_, (A,)> {}
impl<A> ExtendedAudioMarker for FooPropsBuilder<'_, (A,)> {}

#[allow(non_snake_case)]
pub fn Foo<'a>(cx: Scope<'a, FooProps<'a>>) -> Element<'a> {
    let muted = false;
    let attributes = &cx.props.attributes;
    render! {
        // rsx! {
        //     audio {
        //         muted: muted,
        //     }
        // }
        ::dioxus::core::LazyNodes::new(move |__cx: &::dioxus::core::ScopeState| -> ::dioxus::core::VNode   {
            static TEMPLATE: ::dioxus::core::Template = ::dioxus::core::Template { name: concat!(file!(), ":", line!(), ":", column!(), ":", "123" ), roots: &[::dioxus::core::TemplateNode::Element { tag: dioxus_elements::audio::TAG_NAME, namespace: dioxus_elements::audio::NAME_SPACE, attrs: &[::dioxus::core::TemplateAttribute::Dynamic { id: 0usize }], children: &[] }], node_paths: &[], attr_paths: &[&[0u8]] };
            let mut attrs = vec![__cx.attr(dioxus_elements::audio::muted.0, muted, dioxus_elements::audio::muted.1, dioxus_elements::audio::muted.2)];
            attrs.push((&**attributes).into());
            ::dioxus::core::VNode {
                parent: None,
                key: None,
                template: std::cell::Cell::new(TEMPLATE),
                root_ids: Default::default(),
                dynamic_nodes: __cx.bump().alloc([]),
                dynamic_attrs: __cx.bump().alloc(attrs),
            }
        })
    }
}
