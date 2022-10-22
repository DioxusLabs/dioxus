use dioxus::prelude::*;
use dioxus_core::{Attribute, TemplateAttribute};

fn basic_syntax_is_a_template(cx: Scope) -> Element {
    let asd = 123;

    let g = rsx! {
        div {
            class: "asd",
            // class: "{asd}",
            // onclick: move |_| {},
            // div { "{var}" }
        }
    };

    let __cx = NodeFactory::new(&cx);

    static attrs: &'static [TemplateAttribute<'static>] =
        &[::dioxus::core::TemplateAttribute::Static(
            ::dioxus::core::Attribute {
                name: "class",
                namespace: None,
                volatile: false,
                mounted_node: Default::default(),
                value: ::dioxus::core::AttributeValue::Text("asd"),
            },
        )];

    __cx . template_ref (
        || :: dioxus :: core :: Template {
            id : "packages/dioxus/tests/rsx_syntax.rs:7:13:/Users/jonkelley/Development/dioxus/packages/dioxus" ,
            roots : &[
                :: dioxus :: core :: TemplateNode :: Element {
                    tag : dioxus_elements :: div :: TAG_NAME ,
                    attrs : attrs,
                    children : & [] ,
            }] ,
            } ,
         __cx . bump () . alloc ([]) , __cx . bump () . alloc ([]) , __cx . bump () . alloc ([]) ,
        None
    );

    // let static_attr = ::dioxus::core::TemplateAttribute::Static(::dioxus::core::Attribute {
    //     name: "class",
    //     namespace: None,
    //     volatile: false,
    //     mounted_node: Default::default(),
    //     value: ::dioxus::core::AttributeValue::Text("asd"),
    // });

    // __cx . template_ref (|| :: dioxus :: core :: Template { id : "packages/dioxus/tests/rsx_syntax.rs:7:13:/Users/jonkelley/Development/dioxus/packages/dioxus" , roots : & [:: dioxus :: core :: TemplateNode :: Element { tag : dioxus_elements :: div :: TAG_NAME , attrs : & [static_attr , :: dioxus :: core :: TemplateAttribute :: Dynamic (0usize)] , children : & [] , }] , } , __cx . bump () . alloc ([]) , __cx . bump () . alloc ([__cx . attr (dioxus_elements :: div :: class . 0 , :: core :: fmt :: Arguments :: new_v1 (& [""] , & [:: core :: fmt :: ArgumentV1 :: new_display (& asd)]) , None , false)]) , __cx . bump () . alloc ([]) , None);

    cx.render(g)

    // let __cx = NodeFactory::new(&cx);

    // let t = __cx.template_ref (
    //         || :: dioxus :: core :: Template {
    //             id : "packages/dioxus/tests/rsx_syntax.rs:8:13:/Users/jonkelley/Development/dioxus/packages/dioxus" ,
    //             roots : & [
    //                 :: dioxus :: core :: TemplateNode :: Element {
    //                     tag : dioxus_elements :: div :: TAG_NAME ,
    //                     attrs : & [:: dioxus :: core :: TemplateAttribute :: Dynamic (0usize)] ,
    //                     children : & [] ,
    //                 }
    //             ],
    //         },
    //         &[] ,
    //         {
    //             let mut arr = dioxus_core::exports::bumpalo::vec![in __cx.bump()];
    //             arr.push(Attribute {
    //                 name: "asd",
    //                 namespace: None,
    //                 volatile: false,
    //                 mounted_node: Default::default(),
    //                 value: dioxus_core::AttributeValue::Text(
    //                     __cx.raw_text(format_args!("{asd}")).0
    //                 ),
    //             });
    //             arr.into_bump_slice() as &[::dioxus::core::Attribute]
    //         },
    //         & [] ,
    //         None
    //     );

    // Some(t)
}
