#![cfg(not(miri))]

use dioxus::prelude::*;
use dioxus_core::{AttributeValue, DynamicNode, NoOpMutations, Template, VComponent, VNode, *};
use std::{any::Any, cell::RefCell, cfg, collections::HashSet, default::Default, rc::Rc};

fn random_ns() -> Option<&'static str> {
    let namespace = rand::random::<u8>() % 2;
    match namespace {
        0 => None,
        1 => Some(Box::leak(
            format!("ns{}", rand::random::<u8>()).into_boxed_str(),
        )),
        _ => unreachable!(),
    }
}

fn create_random_attribute(attr_idx: &mut usize) -> TemplateAttribute {
    match rand::random::<u8>() % 2 {
        0 => TemplateAttribute::Static {
            name: Box::leak(format!("attr{}", rand::random::<u8>()).into_boxed_str()),
            value: Box::leak(format!("value{}", rand::random::<u8>()).into_boxed_str()),
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
                let attrs: Vec<_> = (0..(rand::random::<u8>() % 10))
                    .map(|_| create_random_attribute(attr_idx))
                    .collect();
                Box::leak(attrs.into_boxed_slice())
            };
            TemplateNode::Element {
                tag: Box::leak(format!("tag{}", rand::random::<u8>()).into_boxed_str()),
                namespace: random_ns(),
                attrs,
                children: {
                    if depth > 4 {
                        &[]
                    } else {
                        let children: Vec<_> = (0..(rand::random::<u8>() % 3))
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
            text: Box::leak(format!("{}", rand::random::<u8>()).into_boxed_str()),
        },
        2 => TemplateNode::Dynamic {
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
        TemplateNode::Element { children, attrs, .. } => {
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
        TemplateNode::Dynamic { .. } => {
            node_paths.push(current_path.to_vec());
        }
    }
}

enum DynamicNodeType {
    Text,
    Other,
}

fn create_random_template(depth: u8) -> (Template, Box<[DynamicNode]>) {
    let mut dynamic_node_types = Vec::new();
    let mut template_idx = 0;
    let mut attr_idx = 0;
    let roots = (0..(1 + rand::random::<u8>() % 5))
        .map(|_| {
            create_random_template_node(
                &mut dynamic_node_types,
                &mut template_idx,
                &mut attr_idx,
                0,
            )
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
    let dynamic_nodes = dynamic_node_types
        .iter()
        .map(|ty| match ty {
            DynamicNodeType::Text => {
                DynamicNode::Text(VText::new(format!("{}", rand::random::<u8>())))
            }
            DynamicNodeType::Other => create_random_dynamic_node(depth + 1),
        })
        .collect();
    (Template::new(roots, node_paths, attr_paths), dynamic_nodes)
}

fn create_random_dynamic_node(depth: u8) -> DynamicNode {
    let range = if depth > 5 { 1 } else { 3 };
    match rand::random::<u8>() % range {
        0 => DynamicNode::Placeholder(Default::default()),
        1 => (0..(rand::random::<u8>() % 5))
            .map(|_| {
                VNode::new(
                    None,
                    Template::new(&[TemplateNode::Dynamic { id: 0 }], &[&[0]], &[]),
                    Box::new([DynamicNode::Component(VComponent::new(
                        create_random_element,
                        DepthProps { depth, root: false },
                        "create_random_element",
                    ))]),
                    Box::new([]),
                )
            })
            .into_dyn_node(),
        2 => DynamicNode::Component(VComponent::new(
            create_random_element,
            DepthProps { depth, root: false },
            "create_random_element",
        )),
        _ => unreachable!(),
    }
}

fn create_random_dynamic_attr() -> Attribute {
    let value = match rand::random::<u8>() % 7 {
        0 => AttributeValue::Text(format!("{}", rand::random::<u8>())),
        1 => AttributeValue::Float(rand::random()),
        2 => AttributeValue::Int(rand::random()),
        3 => AttributeValue::Bool(rand::random()),
        4 => AttributeValue::any_value(rand::random::<u8>()),
        5 => AttributeValue::None,
        6 => {
            let value = AttributeValue::listener(|e: Event<String>| println!("{:?}", e));
            return Attribute::new("ondata", value, None, false);
        }
        _ => unreachable!(),
    };
    Attribute::new(
        Box::leak(format!("attr{}", rand::random::<u8>()).into_boxed_str()),
        value,
        random_ns(),
        rand::random(),
    )
}

#[derive(PartialEq, Props, Clone)]
struct DepthProps {
    depth: u8,
    root: bool,
}

fn create_random_element(cx: DepthProps) -> Element {
    let last_template = use_hook(|| Rc::new(RefCell::new(None)));
    if rand::random::<u8>() % 10 == 0 {
        needs_update();
    }
    let range = if cx.root { 2 } else { 3 };
    let node = match rand::random::<u8>() % range {
        // Change both the template and the dynamic nodes
        0 => {
            let (template, dynamic_nodes) = create_random_template(cx.depth + 1);
            last_template.replace(Some(template));
            VNode::new(
                None,
                template,
                dynamic_nodes,
                (0..template.attr_paths().len())
                    .map(|_| Box::new([create_random_dynamic_attr()]) as Box<[Attribute]>)
                    .collect(),
            )
        }
        // Change just the dynamic nodes
        1 => {
            let (template, dynamic_nodes) = match *last_template.borrow() {
                Some(template) => (
                    template,
                    (0..template.node_paths().len())
                        .map(|_| create_random_dynamic_node(cx.depth + 1))
                        .collect(),
                ),
                None => create_random_template(cx.depth + 1),
            };
            VNode::new(
                None,
                template,
                dynamic_nodes,
                (0..template.attr_paths().len())
                    .map(|_| Box::new([create_random_dynamic_attr()]) as Box<[Attribute]>)
                    .collect(),
            )
        }
        // Remove the template
        _ => VNode::default(),
    };
    Element::Ok(node)
}

// test for panics when creating random nodes and templates
#[test]
fn create() {
    let repeat_count = if cfg!(miri) { 100 } else { 1000 };
    for _ in 0..repeat_count {
        let mut vdom =
            VirtualDom::new_with_props(create_random_element, DepthProps { depth: 0, root: true });
        vdom.rebuild(&mut NoOpMutations);
    }
}

// test for panics when diffing random nodes
// This test will change the template every render which is not very realistic, but it helps stress the system
#[test]
fn diff() {
    let repeat_count = if cfg!(miri) { 100 } else { 1000 };
    for _ in 0..repeat_count {
        let mut vdom =
            VirtualDom::new_with_props(create_random_element, DepthProps { depth: 0, root: true });
        vdom.rebuild(&mut NoOpMutations);
        // A list of all elements that have had event listeners
        // This is intentionally never cleared, so that we can test that calling event listeners that are removed doesn't cause a panic
        let mut event_listeners = HashSet::new();
        for _ in 0..100 {
            for &id in &event_listeners {
                println!("firing event on {:?}", id);
                let event = Event::new(
                    std::rc::Rc::new(String::from("hello world")) as Rc<dyn Any>,
                    true,
                );
                vdom.runtime().handle_event("data", event, id);
            }
            {
                vdom.render_immediate(&mut InsertEventListenerMutationHandler(
                    &mut event_listeners,
                ));
            }
        }
    }
}

struct InsertEventListenerMutationHandler<'a>(&'a mut HashSet<ElementId>);

impl WriteMutations for InsertEventListenerMutationHandler<'_> {
    fn append_children(&mut self, _: ElementId, _: usize) {}

    fn assign_node_id(&mut self, _: &'static [u8], _: ElementId) {}

    fn create_placeholder(&mut self, _: ElementId) {}

    fn create_text_node(&mut self, _: &str, _: ElementId) {}

    fn load_template(&mut self, _: Template, _: usize, _: ElementId) {}

    fn replace_node_with(&mut self, _: ElementId, _: usize) {}

    fn replace_placeholder_with_nodes(&mut self, _: &'static [u8], _: usize) {}

    fn insert_nodes_after(&mut self, _: ElementId, _: usize) {}

    fn insert_nodes_before(&mut self, _: ElementId, _: usize) {}

    fn set_attribute(
        &mut self,
        _: &'static str,
        _: Option<&'static str>,
        _: &AttributeValue,
        _: ElementId,
    ) {
    }

    fn set_node_text(&mut self, _: &str, _: ElementId) {}

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        println!("new event listener on {:?} for {:?}", id, name);
        self.0.insert(id);
    }

    fn remove_event_listener(&mut self, _: &'static str, _: ElementId) {}

    fn remove_node(&mut self, _: ElementId) {}

    fn push_root(&mut self, _: ElementId) {}
}
