use std::cell::Cell;

use dioxus::prelude::Props;
use dioxus_core::*;
use dioxus_native_core::prelude::*;
use dioxus_native_core_macro::partial_derive_state;
use shipyard::Component;

fn random_ns() -> Option<&'static str> {
    let namespace = rand::random::<u8>() % 2;
    match namespace {
        0 => None,
        1 => Some(Box::leak(
            format!("ns{}", rand::random::<usize>()).into_boxed_str(),
        )),
        _ => unreachable!(),
    }
}

fn create_random_attribute(attr_idx: &mut usize) -> TemplateAttribute<'static> {
    match rand::random::<u8>() % 2 {
        0 => TemplateAttribute::Static {
            name: Box::leak(format!("attr{}", rand::random::<usize>()).into_boxed_str()),
            value: Box::leak(format!("value{}", rand::random::<usize>()).into_boxed_str()),
            namespace: random_ns(),
        },
        1 => TemplateAttribute::Dynamic {
            id: {
                let old_idx = *attr_idx;
                *attr_idx += 1;
                old_idx
            },
        },
        _ => unreachable!(),
    }
}

fn create_random_template_node(
    dynamic_node_types: &mut Vec<DynamicNodeType>,
    template_idx: &mut usize,
    attr_idx: &mut usize,
    depth: usize,
) -> TemplateNode {
    match rand::random::<u8>() % 4 {
        0 => {
            let attrs = {
                let attrs: Vec<_> = (0..(rand::random::<usize>() % 10))
                    .map(|_| create_random_attribute(attr_idx))
                    .collect();
                Box::leak(attrs.into_boxed_slice())
            };
            TemplateNode::Element {
                tag: Box::leak(format!("tag{}", rand::random::<usize>()).into_boxed_str()),
                namespace: random_ns(),
                attrs,
                children: {
                    if depth > 4 {
                        &[]
                    } else {
                        let children: Vec<_> = (0..(rand::random::<usize>() % 3))
                            .map(|_| {
                                create_random_template_node(
                                    dynamic_node_types,
                                    template_idx,
                                    attr_idx,
                                    depth + 1,
                                )
                            })
                            .collect();
                        Box::leak(children.into_boxed_slice())
                    }
                },
            }
        }
        1 => TemplateNode::Text {
            text: Box::leak(format!("{}", rand::random::<usize>()).into_boxed_str()),
        },
        2 => TemplateNode::DynamicText {
            id: {
                let old_idx = *template_idx;
                *template_idx += 1;
                dynamic_node_types.push(DynamicNodeType::Text);
                old_idx
            },
        },
        3 => TemplateNode::Dynamic {
            id: {
                let old_idx = *template_idx;
                *template_idx += 1;
                dynamic_node_types.push(DynamicNodeType::Other);
                old_idx
            },
        },
        _ => unreachable!(),
    }
}

fn generate_paths(
    node: &TemplateNode,
    current_path: &[u8],
    node_paths: &mut Vec<Vec<u8>>,
    attr_paths: &mut Vec<Vec<u8>>,
) {
    match node {
        TemplateNode::Element {
            children, attrs, ..
        } => {
            for attr in *attrs {
                match attr {
                    TemplateAttribute::Static { .. } => {}
                    TemplateAttribute::Dynamic { .. } => {
                        attr_paths.push(current_path.to_vec());
                    }
                }
            }
            for (i, child) in children.iter().enumerate() {
                let mut current_path = current_path.to_vec();
                current_path.push(i as u8);
                generate_paths(child, &current_path, node_paths, attr_paths);
            }
        }
        TemplateNode::Text { .. } => {}
        TemplateNode::DynamicText { .. } => {
            node_paths.push(current_path.to_vec());
        }
        TemplateNode::Dynamic { .. } => {
            node_paths.push(current_path.to_vec());
        }
    }
}

enum DynamicNodeType {
    Text,
    Other,
}

fn create_random_template(name: &'static str) -> (Template, Vec<DynamicNodeType>) {
    let mut dynamic_node_type = Vec::new();
    let mut template_idx = 0;
    let mut attr_idx = 0;
    let roots = (0..(1 + rand::random::<usize>() % 5))
        .map(|_| {
            create_random_template_node(&mut dynamic_node_type, &mut template_idx, &mut attr_idx, 0)
        })
        .collect::<Vec<_>>();
    assert!(!roots.is_empty());
    let roots = Box::leak(roots.into_boxed_slice());
    let mut node_paths = Vec::new();
    let mut attr_paths = Vec::new();
    for (i, root) in roots.iter().enumerate() {
        generate_paths(root, &[i as u8], &mut node_paths, &mut attr_paths);
    }
    let node_paths = Box::leak(
        node_paths
            .into_iter()
            .map(|v| &*Box::leak(v.into_boxed_slice()))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );
    let attr_paths = Box::leak(
        attr_paths
            .into_iter()
            .map(|v| &*Box::leak(v.into_boxed_slice()))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );
    (
        Template {
            name,
            roots,
            node_paths,
            attr_paths,
        },
        dynamic_node_type,
    )
}

fn create_random_dynamic_node(depth: usize) -> DynamicNode {
    let range = if depth > 3 { 1 } else { 3 };
    match rand::random::<u8>() % range {
        0 => DynamicNode::Placeholder(Default::default()),
        1 => cx.make_node((0..(rand::random::<u8>() % 5)).map(|_| {
            cx.vnode(
                None.into(),
                Default::default(),
                Cell::new(Template {
                    name: concat!(file!(), ":", line!(), ":", column!(), ":0"),
                    roots: &[TemplateNode::Dynamic { id: 0 }],
                    node_paths: &[&[0]],
                    attr_paths: &[],
                }),
                dioxus::dioxus_core::exports::bumpalo::collections::Vec::new_in(cx.bump()).into(),
                cx.bump().alloc([cx.component(
                    create_random_element,
                    DepthProps { depth, root: false },
                    "create_random_element",
                )]),
                &[],
            )
        })),
        2 => cx.component(
            create_random_element,
            DepthProps { depth, root: false },
            "create_random_element",
        ),
        _ => unreachable!(),
    }
}

fn create_random_dynamic_attr() -> Attribute {
    let value = match rand::random::<u8>() % 6 {
        0 => AttributeValue::Text(Box::leak(
            format!("{}", rand::random::<usize>()).into_boxed_str(),
        )),
        1 => AttributeValue::Float(rand::random()),
        2 => AttributeValue::Int(rand::random()),
        3 => AttributeValue::Bool(rand::random()),
        4 => cx.any_value(rand::random::<usize>()),
        5 => AttributeValue::None,
        _ => unreachable!(),
    };

    Attribute::new(
        Box::leak(format!("attr{}", rand::random::<usize>()).into_boxed_str()),
        value,
        random_ns(),
        rand::random(),
    )
}

static mut TEMPLATE_COUNT: usize = 0;

#[derive(PartialEq, Props, Component)]
struct DepthProps {
    depth: usize,
    root: bool,
}

fn create_random_element(cx: Scope<DepthProps>) -> Element {
    cx.needs_update();
    let range = if cx.props.root { 2 } else { 3 };
    let node = match rand::random::<usize>() % range {
        0 | 1 => {
            let (template, dynamic_node_types) = create_random_template(Box::leak(
                format!(
                    "{}{}",
                    concat!(file!(), ":", line!(), ":", column!(), ":"),
                    {
                        unsafe {
                            let old = TEMPLATE_COUNT;
                            TEMPLATE_COUNT += 1;
                            old
                        }
                    }
                )
                .into_boxed_str(),
            ));
            println!("{template:#?}");
            let node = cx.vnode(
                None.into(),
                None,
                Cell::new(template),
                dioxus::dioxus_core::exports::bumpalo::collections::Vec::new_in(cx.bump()).into(),
                {
                    let dynamic_nodes: Vec<_> = dynamic_node_types
                        .iter()
                        .map(|ty| match ty {
                            DynamicNodeType::Text => DynamicNode::Text(VText::new(Box::leak(
                                format!("{}", rand::random::<usize>()).into_boxed_str(),
                            ))),
                            DynamicNodeType::Other => {
                                create_random_dynamic_node(cx.props.depth + 1)
                            }
                        })
                        .collect();
                    cx.bump().alloc(dynamic_nodes)
                },
                cx.bump()
                    .alloc(
                        (0..template.attr_paths.len())
                            .map(|_| create_random_dynamic_attr(cx).into())
                            .collect::<Vec<_>>(),
                    )
                    .as_slice(),
            );
            Some(node)
        }
        _ => None,
    };
    println!("{node:#?}");
    node
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Component)]
pub struct BlablaState {
    count: usize,
}

#[partial_derive_state]
impl State for BlablaState {
    type ParentDependencies = (Self,);
    type ChildDependencies = ();
    type NodeDependencies = ();

    const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new()
        .with_attrs(AttributeMaskBuilder::Some(&["blabla"]))
        .with_element();

    fn update<'a>(
        &mut self,
        _: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: &SendAnyMap,
    ) -> bool {
        if let Some((parent,)) = parent {
            if parent.count != 0 {
                self.count += 1;
            }
        }
        true
    }

    fn create<'a>(
        node_view: NodeView<()>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        context: &SendAnyMap,
    ) -> Self {
        let mut myself = Self::default();
        myself.update(node_view, node, parent, children, context);
        myself
    }
}

// test for panics when creating random nodes and templates
#[test]
fn create() {
    for _ in 0..100 {
        let mut vdom = VirtualDom::new_with_props(
            create_random_element,
            DepthProps {
                depth: 0,
                root: true,
            },
        );
        let mutations = vdom.rebuild();
        let mut rdom: RealDom = RealDom::new([BlablaState::to_type_erased()]);
        let mut dioxus_state = DioxusState::create(&mut rdom);
        dioxus_state.apply_mutations(&mut rdom, mutations);

        let ctx = SendAnyMap::new();
        rdom.update_state(ctx);
    }
}

// test for panics when diffing random nodes
// This test will change the template every render which is not very realistic, but it helps stress the system
#[test]
fn diff() {
    for _ in 0..10 {
        let mut vdom = VirtualDom::new_with_props(
            create_random_element,
            DepthProps {
                depth: 0,
                root: true,
            },
        );
        let mutations = vdom.rebuild();
        let mut rdom: RealDom = RealDom::new([BlablaState::to_type_erased()]);
        let mut dioxus_state = DioxusState::create(&mut rdom);
        dioxus_state.apply_mutations(&mut rdom, mutations);

        let ctx = SendAnyMap::new();
        rdom.update_state(ctx);
        for _ in 0..10 {
            let mutations = vdom.render_immediate();
            dioxus_state.apply_mutations(&mut rdom, mutations);

            let ctx = SendAnyMap::new();
            rdom.update_state(ctx);
        }
    }
}
