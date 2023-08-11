use dioxus::core_macro::render;
use dioxus::prelude::rsx;
use dioxus_core::{AttributeBox, Element, HasAttributesBox, Scope};
use dioxus_html::{ExtendedAudioMarker, ExtendedGlobalAttributesMarker};

#[test]
fn props_spread() {
    pub struct FooProps<'a> {
        pub open: Option<&'a str>,
        attributes: Vec<AttributeBox<'a>>,
    }

    // -----
    impl<'a> FooProps<'a> {
        #[doc = "\nCreate a builder for building `FooProps`.\nOn the builder, call `.open(...)`(optional) to set the values of the fields.\nFinally, call `.build()` to create the instance of `FooProps`.\n                    "]
        #[allow(dead_code)]
        pub fn builder() -> FooPropsBuilder<'a, ((), ), > {
            FooPropsBuilder { fields: ((), ), attributes: Vec::new(), _phantom: core::default::Default::default() }
        }
    }
    #[must_use]
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    pub struct FooPropsBuilder<'a, TypedBuilderFields, > {
        fields: TypedBuilderFields,
        attributes: Vec<AttributeBox<'a>>,
        _phantom: ( core::marker::PhantomData<&'a ()>   ),
    }
    //impl<'a, TypedBuilderFields, > Clone for FooPropsBuilder<'a, TypedBuilderFields, > where TypedBuilderFields: Clone { fn clone(&self) -> Self { Self { fields: self.fields.clone(), attributes: self.attributes, _phantom: Default::default() } } }
    impl<'a> dioxus::prelude::Properties for FooProps<'a> {
        type Builder = FooPropsBuilder<'a, ((), ), >;
        const IS_STATIC: bool = false;
        fn builder() -> Self::Builder { FooProps::builder() }
        unsafe fn memoize(&self, other: &Self) -> bool { false }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    pub trait FooPropsBuilder_Optional<T> { fn into_value<F: FnOnce() -> T>(self, default: F) -> T; }
    impl<T> FooPropsBuilder_Optional<T> for () { fn into_value<F: FnOnce() -> T>(self, default: F) -> T { default() } }
    impl<T> FooPropsBuilder_Optional<T> for (T, ) { fn into_value<F: FnOnce() -> T>(self, _: F) -> T { self.0 } }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    impl<'a> FooPropsBuilder<'a, ((), )> {
        pub fn open(self, open: &'a str) -> FooPropsBuilder<'a, ((Option<&'a str>,
                                                                  // pub attributes: Vec<Attribute<'a>>,
                                                                 ), )> {
            let open = (Some(open), );
            let (_, ) = self.fields;
            FooPropsBuilder { fields: (open, ), attributes: self.attributes, _phantom: self._phantom }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    pub enum FooPropsBuilder_Error_Repeated_field_open {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    impl<'a> FooPropsBuilder<'a, ((Option<&'a str>,
                                   // pub attributes: Vec<Attribute<'a>>,
                                  ), )> {
        #[deprecated(note = "Repeated field open")]
        pub fn open(self, _: FooPropsBuilder_Error_Repeated_field_open) -> FooPropsBuilder<'a, ((Option<&'a str>,
                                                                                                 // pub attributes: Vec<Attribute<'a>>,
                                                                                                ), )> { self }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    impl<'a, __open: FooPropsBuilder_Optional<Option<&'a str>>> FooPropsBuilder<'a, (__open, ), > {
        pub fn build(self) -> FooProps<'a> {
            let (open, ) = self.fields;
            let open = FooPropsBuilder_Optional::into_value(open, || Default::default());
            FooProps { open, attributes: self.attributes }
        }
    }
    // -----

    impl<'a, A> HasAttributesBox<'a, FooPropsBuilder<'a, (A, )>> for FooPropsBuilder<'a, (A, )> {
        fn push_attribute(self, attr: AttributeBox<'a>) -> FooPropsBuilder<'a, (A, )> {
            let mut attrs = Vec::from(self.attributes);
            attrs.push(attr);
            FooPropsBuilder { fields: self.fields, attributes: attrs, _phantom: self._phantom }
        }
    }
    impl<A,> ExtendedGlobalAttributesMarker for FooPropsBuilder<'_, (A,)> {}
    impl<A,> ExtendedAudioMarker for FooPropsBuilder<'_, (A,)> {}

    use dioxus::prelude::*;
    use dioxus_html::AudioExtension;

    #[allow(non_snake_case)]
    pub fn Foo<'a>(cx: Scope<'a, FooProps<'a>>) -> Element<'a> {
        let muted = false;
        let attributes = cx.props.attributes;
        render! {
            // rsx! {
            //     audio {
            //         muted: muted,
            //     }
            // }
            ::dioxus::core::LazyNodes::new(move |__cx: &::dioxus::core::ScopeState| -> ::dioxus::core::VNode   {
                static TEMPLATE: ::dioxus::core::Template = ::dioxus::core::Template { name: concat!(file!(), ":", line!(), ":", column!(), ":", "" ), roots: &[::dioxus::core::TemplateNode::Element { tag: dioxus_elements::audio::TAG_NAME, namespace: dioxus_elements::audio::NAME_SPACE, attrs: &[::dioxus::core::TemplateAttribute::Dynamic { id: 0usize }], children: &[] }], node_paths: &[], attr_paths: &[&[0u8]] };
                let mut attrs = vec![__cx.attr(dioxus_elements::audio::muted.0, muted, dioxus_elements::audio::muted.1, dioxus_elements::audio::muted.2)];
                for attr in attributes {
                    attrs.push(__cx.attr(attr.name, attr.value.into_value(__cx.bump()), attr.namespace, attr.volatile));
                };
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

    rsx! {
        Foo {
            autoplay: true,
            controls: true,
        }
    };
}