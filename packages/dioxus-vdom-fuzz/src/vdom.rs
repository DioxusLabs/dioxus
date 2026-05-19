#![allow(non_snake_case)]

use crate::{
    model::*,
    ops::{SuspenseReadyFuture, read_model},
};
use dioxus::prelude::*;
use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, Task, Template, TemplateAttribute, TemplateNode,
    VComponent, VNode, VText,
};
use std::{
    collections::HashMap,
    future::pending,
    sync::{Mutex, OnceLock},
};

// ---------- VNode construction --------------------------------------------------------------

pub(crate) fn App() -> Element {
    Ok(build_vnode(&read_model().root))
}

#[derive(Clone, PartialEq, Props)]
struct GeneratedProps {
    node: VNodeSpec,
}

#[derive(Clone, PartialEq, Props)]
struct GeneratedSuspenseProps {
    id: u64,
    ready_generation: u64,
    mode: SuspenseMode,
    wake_mutation: WakeMutationSpec,
    wake_applied: bool,
    child: VNodeSpec,
}

fn GeneratedComponent(props: GeneratedProps) -> Element {
    Ok(build_vnode(&props.node))
}

fn OtherGeneratedComponent(props: GeneratedProps) -> Element {
    Ok(build_vnode(&props.node))
}

fn GeneratedSuspenseBoundary(props: GeneratedSuspenseProps) -> Element {
    let id = props.id;
    let ready_generation = props.ready_generation;
    let mode = props.mode;
    let wake_mutation = props.wake_mutation;
    let wake_applied = props.wake_applied;
    let child = props.child;
    rsx! {
        SuspenseBoundary {
            fallback: |_| rsx! { "suspense-fallback" },
            GeneratedSuspenseChild {
                id,
                ready_generation,
                mode,
                wake_mutation,
                wake_applied,
                child,
            }
        }
    }
}

fn GeneratedSuspenseChild(props: GeneratedSuspenseProps) -> Element {
    let mut task: Signal<Option<Task>> = use_signal(|| None);
    let mut task_key: Signal<Option<SuspenseTaskKey>> = use_signal(|| None);
    let mut ready_resolved = use_signal(|| false);
    let mut applied_wake_mutation = use_signal(|| {
        if props.wake_applied {
            props.wake_mutation
        } else {
            WakeMutationSpec::None
        }
    });

    let next_task_key = match props.mode {
        SuspenseMode::Resolved => None,
        SuspenseMode::Pending => Some(SuspenseTaskKey::Pending(props.id)),
        SuspenseMode::Ready => Some(SuspenseTaskKey::Ready(SuspenseReadyKey {
            id: props.id,
            generation: props.ready_generation,
        })),
    };

    if task_key.cloned() != next_task_key {
        if let Some(task) = task.take() {
            task.cancel();
        }
        task_key.set(None);
        ready_resolved.set(false);
        applied_wake_mutation.set(if props.wake_applied {
            props.wake_mutation
        } else {
            WakeMutationSpec::None
        });
    } else if props.wake_applied {
        if applied_wake_mutation() != props.wake_mutation {
            applied_wake_mutation.set(props.wake_mutation);
        }
    } else if props.mode == SuspenseMode::Resolved
        && applied_wake_mutation() != WakeMutationSpec::None
    {
        applied_wake_mutation.set(WakeMutationSpec::None);
    }

    match props.mode {
        SuspenseMode::Resolved => {
            if let Some(task) = task.take() {
                task.cancel();
            }
        }
        SuspenseMode::Pending => {
            let running = task.cloned().unwrap_or_else(|| {
                let new_task = spawn(async { pending::<()>().await });
                task.set(Some(new_task));
                task_key.set(next_task_key);
                new_task
            });
            suspend(running)?;
        }
        SuspenseMode::Ready => {
            if !ready_resolved() {
                let running = task.cloned().unwrap_or_else(|| {
                    let Some(SuspenseTaskKey::Ready(key)) = next_task_key else {
                        unreachable!();
                    };
                    let new_task = spawn(async move {
                        SuspenseReadyFuture { key }.await;
                        let wake_mutation = read_model().wake_mutation_for_ready_key(key);
                        if wake_mutation != WakeMutationSpec::None {
                            applied_wake_mutation.set(wake_mutation);
                        }
                        ready_resolved.set(true);
                    });
                    task.set(Some(new_task));
                    task_key.set(next_task_key);
                    new_task
                });
                suspend(running)?;
            }
        }
    }

    let local_wake_mutation = applied_wake_mutation();
    let wake_mutation = if local_wake_mutation != WakeMutationSpec::None {
        local_wake_mutation
    } else {
        props.wake_mutation
    };
    Ok(build_suspense_child_vnode(
        &props.child,
        wake_mutation,
        props.wake_applied || local_wake_mutation != WakeMutationSpec::None,
    ))
}

fn build_suspense_child_vnode(
    child: &VNodeSpec,
    wake_mutation: WakeMutationSpec,
    wake_applied: bool,
) -> VNode {
    let child = build_vnode(child);
    let WakeMutationSpec::PrependStaticRoot { tag } = wake_mutation else {
        return child;
    };
    if !wake_applied {
        return child;
    }

    let template = compile_template(&TemplateSpec {
        roots: vec![
            TemplateNodeSpec::Element {
                tag,
                namespace: None,
                attrs: Vec::new(),
                children: Vec::new(),
            },
            TemplateNodeSpec::Dynamic,
        ],
    });

    VNode::new(
        None,
        template,
        Box::new([DynamicNode::Fragment(vec![child])]),
        Vec::<Box<[Attribute]>>::new().into_boxed_slice(),
    )
}

fn build_vnode(spec: &VNodeSpec) -> VNode {
    let spec = spec.clone().normalize();
    VNode::new(
        spec.key.map(|key| format!("k{key}")),
        compile_template(&spec.template),
        spec.dynamics.iter().map(build_dynamic).collect(),
        spec.attrs
            .iter()
            .enumerate()
            .map(|(slot, attrs)| attrs.iter().map(|attr| build_attr(slot, attr)).collect())
            .collect(),
    )
}

fn build_dynamic(spec: &DynamicSpec) -> DynamicNode {
    match spec {
        DynamicSpec::Empty => DynamicNode::Fragment(Vec::new()),
        DynamicSpec::Text(value) => DynamicNode::Text(VText::new(format!("text-{value}"))),
        DynamicSpec::Fragment(nodes) => {
            DynamicNode::Fragment(nodes.iter().map(build_vnode).collect())
        }
        DynamicSpec::ComponentA(node) => DynamicNode::Component(VComponent::new(
            GeneratedComponent,
            GeneratedProps {
                node: node.as_ref().clone(),
            },
            "GeneratedComponent",
        )),
        DynamicSpec::ComponentB(node) => DynamicNode::Component(VComponent::new(
            OtherGeneratedComponent,
            GeneratedProps {
                node: node.as_ref().clone(),
            },
            "OtherGeneratedComponent",
        )),
        DynamicSpec::Portal(spec) => DynamicNode::Component(VComponent::new(
            GeneratedComponent,
            GeneratedProps {
                node: spec.child.as_ref().clone(),
            },
            "GeneratedPortal",
        )),
        DynamicSpec::Suspense(spec) => DynamicNode::Component(VComponent::new(
            GeneratedSuspenseBoundary,
            GeneratedSuspenseProps {
                id: spec.id,
                ready_generation: spec.ready_generation,
                mode: spec.mode,
                wake_mutation: spec.wake_mutation,
                wake_applied: spec.wake_applied,
                child: spec.child.as_ref().clone(),
            },
            "GeneratedSuspenseBoundary",
        )),
    }
}

fn build_attr(slot: usize, spec: &AttrSpec) -> Attribute {
    let namespace = spec.namespace.map(namespace_name);
    match spec.value {
        AttrValueSpec::Text(value) => Attribute::new(
            attr_name(spec.name),
            format!("attr-value-{value}"),
            namespace,
            spec.volatile,
        ),
        AttrValueSpec::Float(value) => Attribute::new(
            attr_name(spec.name),
            f64::from(value) / 10.0,
            namespace,
            spec.volatile,
        ),
        AttrValueSpec::Int(value) => {
            Attribute::new(attr_name(spec.name), value as i64, namespace, spec.volatile)
        }
        AttrValueSpec::Bool(value) => {
            Attribute::new(attr_name(spec.name), value, namespace, spec.volatile)
        }
        AttrValueSpec::Any(value) => Attribute::new(
            attr_name(spec.name),
            AttributeValue::any_value(value),
            namespace,
            spec.volatile,
        ),
        AttrValueSpec::None => Attribute::new(
            attr_name(spec.name),
            AttributeValue::None,
            namespace,
            spec.volatile,
        ),
        AttrValueSpec::Listener => Attribute::new(
            listener_name(slot, spec.name),
            AttributeValue::listener(|_: Event<String>| {}),
            None,
            spec.volatile,
        ),
    }
}

fn compile_template(spec: &TemplateSpec) -> Template {
    static CACHE: OnceLock<Mutex<HashMap<TemplateSpec, Template>>> = OnceLock::new();

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache.lock().unwrap();
    if let Some(template) = cache.get(spec) {
        return *template;
    }

    let template = compile_template_uncached(spec);
    cache.insert(spec.clone(), template);
    template
}

fn compile_template_uncached(spec: &TemplateSpec) -> Template {
    let mut compiler = TemplateCompiler::default();
    let roots: Vec<_> = spec
        .roots
        .iter()
        .enumerate()
        .map(|(index, root)| compiler.compile_node(root, &[index as u8]))
        .collect();
    Template::new(
        leak_slice(roots),
        leak_path_list(compiler.node_paths),
        leak_path_list(compiler.attr_paths),
    )
}

#[derive(Default)]
struct TemplateCompiler {
    next_dynamic: usize,
    next_attr: usize,
    node_paths: Vec<Vec<u8>>,
    attr_paths: Vec<Vec<u8>>,
}

impl TemplateCompiler {
    fn compile_node(&mut self, spec: &TemplateNodeSpec, path: &[u8]) -> TemplateNode {
        match spec {
            TemplateNodeSpec::Element {
                tag,
                namespace,
                attrs,
                children,
            } => {
                let attrs = attrs
                    .iter()
                    .map(|attr| self.compile_attr(attr, path))
                    .collect();
                let children = children
                    .iter()
                    .enumerate()
                    .map(|(index, child)| {
                        let mut child_path = path.to_vec();
                        child_path.push(index as u8);
                        self.compile_node(child, &child_path)
                    })
                    .collect();
                TemplateNode::Element {
                    tag: tag_name(*tag),
                    namespace: namespace.map(namespace_name),
                    attrs: leak_slice(attrs),
                    children: leak_slice(children),
                }
            }
            TemplateNodeSpec::Text(value) => TemplateNode::Text {
                text: text_value(*value),
            },
            TemplateNodeSpec::Dynamic => {
                let id = self.next_dynamic;
                self.next_dynamic += 1;
                self.node_paths.push(path.to_vec());
                TemplateNode::Dynamic { id }
            }
        }
    }

    fn compile_attr(&mut self, spec: &TemplateAttrSpec, path: &[u8]) -> TemplateAttribute {
        match spec {
            TemplateAttrSpec::Static {
                name,
                value,
                namespace,
            } => TemplateAttribute::Static {
                name: attr_name(*name),
                value: attr_static_value(*value),
                namespace: namespace.map(namespace_name),
            },
            TemplateAttrSpec::Dynamic => {
                let id = self.next_attr;
                self.next_attr += 1;
                self.attr_paths.push(path.to_vec());
                TemplateAttribute::Dynamic { id }
            }
        }
    }
}

fn leak_slice<T: 'static>(value: Vec<T>) -> &'static [T] {
    if value.is_empty() {
        &[]
    } else {
        Box::leak(value.into_boxed_slice())
    }
}

fn leak_path_list(paths: Vec<Vec<u8>>) -> &'static [&'static [u8]] {
    if paths.is_empty() {
        return &[];
    }

    let paths = paths
        .into_iter()
        .map(|path| {
            let path: &'static mut [u8] = Box::leak(path.into_boxed_slice());
            &*path as &'static [u8]
        })
        .collect();
    leak_slice(paths)
}

fn leak_str(value: String) -> &'static str {
    static CACHE: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache.lock().unwrap();
    if let Some(interned) = cache.get(value.as_str()) {
        return *interned;
    }

    let interned: &'static str = Box::leak(value.clone().into_boxed_str());
    cache.insert(value, interned);
    interned
}

fn tag_name(value: u8) -> &'static str {
    leak_str(format!("tag{value}"))
}

fn namespace_name(value: u8) -> &'static str {
    leak_str(format!("ns{value}"))
}

fn attr_name(value: u8) -> &'static str {
    leak_str(format!("attr{value}"))
}

fn listener_name(slot: usize, value: u8) -> &'static str {
    leak_str(format!("onevent{slot}_{value}"))
}

fn attr_static_value(value: u8) -> &'static str {
    leak_str(format!("static{value}"))
}

fn text_value(value: u8) -> &'static str {
    leak_str(format!("static-text-{value}"))
}
