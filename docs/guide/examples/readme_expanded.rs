use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let mut count = use_state(cx, || 0);

    cx.render(
        // rsx expands to LazyNodes::new
        ::dioxus::core::LazyNodes::new(
            move |__cx: &::dioxus::core::ScopeState| -> ::dioxus::core::VNode {
                // The template is every static part of the rsx
                static TEMPLATE: ::dioxus::core::Template = ::dioxus::core::Template {
                    // This is the source location of the rsx that generated this template. This is used to make hot rsx reloading work. Hot rsx reloading just replaces the template with a new one generated from the rsx by the CLI.
                    name: "examples\\readme.rs:14:15:250",
                    // The root nodes are the top level nodes of the rsx
                    roots: &[
                        // The h1 node
                        ::dioxus::core::TemplateNode::Element {
                            // Find the built in h1 tag in the dioxus_elements crate exported by the dioxus html crate
                            tag: dioxus_elements::h1::TAG_NAME,
                            namespace: dioxus_elements::h1::NAME_SPACE,
                            attrs: &[],
                            // The children of the h1 node
                            children: &[
                                // The dynamic count text node
                                // Any nodes that are dynamic have a dynamic placeholder with a unique index
                                ::dioxus::core::TemplateNode::DynamicText {
                                    // This index is used to find what element in `dynamic_nodes` to use instead of the placeholder
                                    id: 0usize,
                                },
                            ],
                        },
                        // The up high button node
                        ::dioxus::core::TemplateNode::Element {
                            tag: dioxus_elements::button::TAG_NAME,
                            namespace: dioxus_elements::button::NAME_SPACE,
                            attrs: &[
                                // The dynamic onclick listener attribute
                                // Any attributes that are dynamic have a dynamic placeholder with a unique index.
                                ::dioxus::core::TemplateAttribute::Dynamic {
                                    // Similar to dynamic nodes, dynamic attributes have a unique index used to find the attribute in `dynamic_attrs` to use instead of the placeholder
                                    id: 0usize,
                                },
                            ],
                            children: &[::dioxus::core::TemplateNode::Text { text: "Up high!" }],
                        },
                        // The down low button node
                        ::dioxus::core::TemplateNode::Element {
                            tag: dioxus_elements::button::TAG_NAME,
                            namespace: dioxus_elements::button::NAME_SPACE,
                            attrs: &[
                                // The dynamic onclick listener attribute
                                ::dioxus::core::TemplateAttribute::Dynamic { id: 1usize },
                            ],
                            children: &[::dioxus::core::TemplateNode::Text { text: "Down low!" }],
                        },
                    ],
                    // Node paths is a list of paths to every dynamic node in the rsx
                    node_paths: &[
                        // The first node path is the path to the dynamic node with an id of 0 (the count text node)
                        &[
                            // Go to the index 0 root node
                            0u8,
                            //
                            // Go to the first child of the root node
                            0u8,
                        ],
                    ],
                    // Attr paths is a list of paths to every dynamic attribute in the rsx
                    attr_paths: &[
                        // The first attr path is the path to the dynamic attribute with an id of 0 (the up high button onclick listener)
                        &[
                            // Go to the index 1 root node
                            1u8,
                        ],
                        // The second attr path is the path to the dynamic attribute with an id of 1 (the down low button onclick listener)
                        &[
                            // Go to the index 2 root node
                            2u8,
                        ],
                    ],
                };
                // The VNode is a reference to the template with the dynamic parts of the rsx
                ::dioxus::core::VNode {
                    parent: None,
                    key: None,
                    // The static template this node will use. The template is stored in a Cell so it can be replaced with a new template when hot rsx reloading is enabled
                    template: std::cell::Cell::new(TEMPLATE),
                    root_ids: dioxus::core::exports::bumpalo::collections::Vec::new_in(__cx.bump())
                        .into(),
                    dynamic_nodes: __cx.bump().alloc([
                        // The dynamic count text node (dynamic node id 0)
                        __cx.text_node(format_args!("High-Five counter: {0}", count)),
                    ]),
                    dynamic_attrs: __cx.bump().alloc([
                        // The dynamic up high button onclick listener (dynamic attribute id 0)
                        dioxus_elements::events::onclick(__cx, move |_| count += 1),
                        // The dynamic down low button onclick listener (dynamic attribute id 1)
                        dioxus_elements::events::onclick(__cx, move |_| count -= 1),
                    ]),
                }
            },
        ),
    )
}
