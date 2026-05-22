#![allow(non_snake_case)]

use crate::{
    cache::InternSet,
    lifecycle::{self, LifecycleRole},
    model::*,
    ops::{SuspenseReadyFuture, read_model},
};
use dioxus::prelude::*;
use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, Task, Template, TemplateAttribute, TemplateNode,
    VComponent, VNode, VText,
};
use std::{
    borrow::Borrow,
    future::pending,
    hash::{Hash, Hasher},
};

// ---------- VNode construction --------------------------------------------------------------

pub(crate) fn App() -> Element {
    Ok(build_vnode(&read_model().root))
}

#[derive(Clone, PartialEq, Props)]
struct GeneratedProps {
    id: u64,
    suspense_ancestors: Vec<u64>,
    node: VNodeSpec,
}

#[derive(Clone, PartialEq, Props)]
struct GeneratedSuspenseProps {
    id: u64,
    ready_generation: u64,
    required_ready_wake_count: usize,
    mode: SuspenseMode,
    wake_mutation: WakeMutationSpec,
    wake_applied: bool,
    suspense_ancestors: Vec<u64>,
    child: VNodeSpec,
}

fn GeneratedComponent(props: GeneratedProps) -> Element {
    track_lifecycle(
        LifecycleRole::ComponentA,
        props.id,
        &props.suspense_ancestors,
    );
    Ok(build_vnode_with_suspense(
        &props.node,
        &props.suspense_ancestors,
    ))
}

fn OtherGeneratedComponent(props: GeneratedProps) -> Element {
    track_lifecycle(
        LifecycleRole::ComponentB,
        props.id,
        &props.suspense_ancestors,
    );
    Ok(build_vnode_with_suspense(
        &props.node,
        &props.suspense_ancestors,
    ))
}

fn GeneratedSuspenseBoundary(props: GeneratedSuspenseProps) -> Element {
    track_lifecycle(
        LifecycleRole::SuspenseBoundary,
        props.id,
        &props.suspense_ancestors,
    );
    let id = props.id;
    let ready_generation = props.ready_generation;
    let required_ready_wake_count = props.required_ready_wake_count;
    let mode = props.mode;
    let wake_mutation = props.wake_mutation;
    let wake_applied = props.wake_applied;
    let suspense_ancestors = props.suspense_ancestors;
    let child_spec = props.child;

    if vnode_contains_suspense(&child_spec) {
        return rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "suspense-fallback" },
                GeneratedSuspenseChild {
                    id,
                    ready_generation,
                    required_ready_wake_count,
                    mode,
                    wake_mutation,
                    wake_applied,
                    suspense_ancestors,
                    child: child_spec,
                }
            }
        };
    }

    let mut child_suspense_ancestors = suspense_ancestors.clone();
    child_suspense_ancestors.push(id);
    let child = build_suspense_child_vnode(&child_spec, &child_suspense_ancestors, wake_mutation, wake_applied);
    rsx! {
        SuspenseBoundary {
            fallback: |_| rsx! { "suspense-fallback" },
            GeneratedSuspenseChild {
                id,
                ready_generation,
                required_ready_wake_count,
                mode,
                wake_mutation: WakeMutationSpec::None,
                wake_applied: false,
                suspense_ancestors,
                child: VNodeSpec::minimal(),
            }
            {child}
        }
    }
}

fn GeneratedSuspenseChild(props: GeneratedSuspenseProps) -> Element {
    track_lifecycle(
        LifecycleRole::SuspenseChild,
        props.id,
        &props.suspense_ancestors,
    );
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
        SuspenseMode::Ready { .. } => Some(SuspenseTaskKey::Ready(SuspenseReadyKey {
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
        SuspenseMode::Ready { .. } => {
            if !ready_resolved() {
                if let Some(running) = task.cloned() {
                    suspend(running)?;
                } else {
                    let Some(SuspenseTaskKey::Ready(key)) = next_task_key else {
                        unreachable!();
                    };
                    let required_wakes = props.required_ready_wake_count;
                    let new_task = spawn(async move {
                        SuspenseReadyFuture {
                            key,
                            required_wakes,
                        }
                        .await;
                        let wake_mutation = read_model().wake_mutation_for_ready_key(key);
                        if wake_mutation != WakeMutationSpec::None {
                            applied_wake_mutation.set(wake_mutation);
                        }
                        ready_resolved.set(true);
                    });
                    task_key.set(next_task_key);
                    if new_task.poll_now().is_pending() {
                        task.set(Some(new_task));
                        suspend(new_task)?;
                    }
                }
            }
        }
    }

    let local_wake_mutation = applied_wake_mutation();
    let wake_mutation = if local_wake_mutation != WakeMutationSpec::None {
        local_wake_mutation
    } else {
        props.wake_mutation
    };
    let mut child_suspense_ancestors = props.suspense_ancestors.clone();
    child_suspense_ancestors.push(props.id);
    Ok(build_suspense_child_vnode(
        &props.child,
        &child_suspense_ancestors,
        wake_mutation,
        props.wake_applied || local_wake_mutation != WakeMutationSpec::None,
    ))
}

fn track_lifecycle(role: LifecycleRole, id: u64, suspense_ancestors: &[u64]) {
    let suspense_ancestors = suspense_ancestors.to_vec();
    let guard = use_hook({
        let suspense_ancestors = suspense_ancestors.clone();
        move || lifecycle::track(role, id, &suspense_ancestors)
    });
    guard.update(role, id, &suspense_ancestors);
}

fn build_suspense_child_vnode(
    child: &VNodeSpec,
    suspense_ancestors: &[u64],
    wake_mutation: WakeMutationSpec,
    wake_applied: bool,
) -> VNode {
    let child = build_vnode_with_suspense(child, suspense_ancestors);
    let WakeMutationSpec::PrependStaticRoot { tag } = wake_mutation else {
        return child;
    };
    if !wake_applied {
        return child;
    }

    let template = compile_template(&TemplateSpec {
        cache_key: None,
        roots: vec![
            TemplateNodeSpec::Element {
                tag,
                namespace: None,
                attrs: Vec::new(),
                children: Vec::new(),
            },
            TemplateNodeSpec::Dynamic(DynamicSpec::Empty),
        ],
    });

    VNode::new(
        None,
        template,
        Box::new([DynamicNode::Fragment(vec![child])]),
        Vec::<Box<[Attribute]>>::new().into_boxed_slice(),
    )
}

fn vnode_contains_suspense(spec: &VNodeSpec) -> bool { spec.template.roots.iter().any(template_node_contains_suspense) }

fn template_node_contains_suspense(spec: &TemplateNodeSpec) -> bool {
    match spec {
        TemplateNodeSpec::Element { children, .. } => children.iter().any(template_node_contains_suspense),
        TemplateNodeSpec::Dynamic(DynamicSpec::Fragment(nodes)) => nodes.iter().any(vnode_contains_suspense),
        TemplateNodeSpec::Dynamic(DynamicSpec::ComponentA(component) | DynamicSpec::ComponentB(component)) => vnode_contains_suspense(&component.child),
        TemplateNodeSpec::Dynamic(DynamicSpec::Suspense(_)) => true,
        TemplateNodeSpec::Text(_) | TemplateNodeSpec::Dynamic(_) => false,
    }
}

fn build_vnode(spec: &VNodeSpec) -> VNode {
    build_vnode_with_suspense(spec, &[])
}

fn build_vnode_with_suspense(spec: &VNodeSpec, suspense_ancestors: &[u64]) -> VNode {
    let spec = spec.clone().normalize();
    let mut dynamics = Vec::new();
    collect_dynamic_specs(&spec.template.roots, &mut dynamics);
    let mut attrs = Vec::new();
    collect_dynamic_attr_specs(&spec.template.roots, &mut attrs);
    VNode::new(
        spec.key.map(|key| format!("k{key}")),
        compile_template(&spec.template),
        dynamics
            .iter()
            .map(|dynamic| build_dynamic(dynamic, suspense_ancestors))
            .collect(),
        attrs
            .iter()
            .enumerate()
            .map(|(slot, attrs)| attrs.iter().map(|attr| build_attr(slot, attr)).collect())
            .collect(),
    )
}

fn collect_dynamic_specs<'a>(nodes: &'a [TemplateNodeSpec], out: &mut Vec<&'a DynamicSpec>) {
    for node in nodes {
        match node {
            TemplateNodeSpec::Element { children, .. } => collect_dynamic_specs(children, out),
            TemplateNodeSpec::Text(_) => {}
            TemplateNodeSpec::Dynamic(dynamic) => out.push(dynamic),
        }
    }
}

fn collect_dynamic_attr_specs<'a>(nodes: &'a [TemplateNodeSpec], out: &mut Vec<&'a [AttrSpec]>) {
    for node in nodes {
        let TemplateNodeSpec::Element {
            attrs, children, ..
        } = node
        else {
            continue;
        };

        for attr in attrs {
            if let TemplateAttrSpec::Dynamic(attrs) = attr {
                out.push(attrs);
            }
        }

        collect_dynamic_attr_specs(children, out);
    }
}

fn build_dynamic(spec: &DynamicSpec, suspense_ancestors: &[u64]) -> DynamicNode {
    match spec {
        DynamicSpec::Empty => DynamicNode::Fragment(Vec::new()),
        DynamicSpec::Text(value) => DynamicNode::Text(VText::new(format!("text-{value}"))),
        DynamicSpec::Placeholder => DynamicNode::Placeholder(Default::default()),
        DynamicSpec::Fragment(nodes) => DynamicNode::Fragment(
            nodes
                .iter()
                .map(|node| build_vnode_with_suspense(node, suspense_ancestors))
                .collect(),
        ),
        DynamicSpec::ComponentA(component) => DynamicNode::Component(VComponent::new(
            GeneratedComponent,
            GeneratedProps {
                id: component.id,
                suspense_ancestors: suspense_ancestors.to_vec(),
                node: component.child.as_ref().clone(),
            },
            "GeneratedComponent",
        )),
        DynamicSpec::ComponentB(component) => DynamicNode::Component(VComponent::new(
            OtherGeneratedComponent,
            GeneratedProps {
                id: component.id,
                suspense_ancestors: suspense_ancestors.to_vec(),
                node: component.child.as_ref().clone(),
            },
            "OtherGeneratedComponent",
        )),
        DynamicSpec::Suspense(spec) => DynamicNode::Component(VComponent::new(
            GeneratedSuspenseBoundary,
            GeneratedSuspenseProps {
                id: spec.id,
                ready_generation: spec.ready_generation,
                required_ready_wake_count: spec.mode.required_ready_wake_count().unwrap_or(1)
                    as usize,
                mode: spec.mode,
                wake_mutation: spec.wake_mutation,
                wake_applied: spec.wake_applied,
                suspense_ancestors: suspense_ancestors.to_vec(),
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
            dynamic_attr_name(slot, spec.name),
            format!("attr-value-{value}"),
            namespace,
            spec.volatile,
        ),
        AttrValueSpec::Float(value) => Attribute::new(
            dynamic_attr_name(slot, spec.name),
            f64::from(value) / 10.0,
            namespace,
            spec.volatile,
        ),
        AttrValueSpec::Int(value) => {
            Attribute::new(
                dynamic_attr_name(slot, spec.name),
                value as i64,
                namespace,
                spec.volatile,
            )
        }
        AttrValueSpec::Bool(value) => {
            Attribute::new(
                dynamic_attr_name(slot, spec.name),
                value,
                namespace,
                spec.volatile,
            )
        }
        AttrValueSpec::Any(value) => Attribute::new(
            dynamic_attr_name(slot, spec.name),
            AttributeValue::any_value(value),
            namespace,
            spec.volatile,
        ),
        AttrValueSpec::None => Attribute::new(
            dynamic_attr_name(slot, spec.name),
            AttributeValue::None,
            namespace,
            spec.volatile,
        ),
        AttrValueSpec::Listener => Attribute::new(
            listener_name(slot, spec.name),
            AttributeValue::listener(|_: Event<String>| crate::event::handle_listener_event()),
            None,
            spec.volatile,
        ),
    }
}

fn compile_template(spec: &TemplateSpec) -> Template {
    static CACHE: InternSet<CompiledTemplate> = InternSet::new();

    let key = spec.cache_key();
    CACHE
        .get_or_insert_with(&key, || CompiledTemplate {
            key: key.clone(),
            template: compile_template_uncached(spec),
        })
        .template
}

fn compile_template_uncached(spec: &TemplateSpec) -> Template {
    Template::new(
        intern_template_node_slice(&spec.roots, 0, 0),
        intern_path_list(collect_node_paths(&spec.roots)),
        intern_path_list(collect_attr_paths(&spec.roots)),
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TemplateNodeCacheKey {
    spec: TemplateNodeShape,
    dynamic_base: usize,
    attr_base: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TemplateNodeSliceCacheKey {
    specs: Vec<TemplateNodeShape>,
    dynamic_base: usize,
    attr_base: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TemplateAttrSliceCacheKey {
    attrs: Vec<TemplateAttrShape>,
    attr_base: usize,
}

#[derive(Clone)]
struct CompiledTemplate {
    key: TemplateCacheKey,
    template: Template,
}

impl Borrow<TemplateCacheKey> for CompiledTemplate {
    fn borrow(&self) -> &TemplateCacheKey {
        &self.key
    }
}

impl PartialEq for CompiledTemplate {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for CompiledTemplate {}

impl Hash for CompiledTemplate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

#[derive(Clone)]
struct TemplateNodeSliceEntry {
    key: TemplateNodeSliceCacheKey,
    nodes: &'static [TemplateNode],
}

impl Borrow<TemplateNodeSliceCacheKey> for TemplateNodeSliceEntry {
    fn borrow(&self) -> &TemplateNodeSliceCacheKey {
        &self.key
    }
}

impl PartialEq for TemplateNodeSliceEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for TemplateNodeSliceEntry {}

impl Hash for TemplateNodeSliceEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

#[derive(Clone)]
struct TemplateNodeEntry {
    key: TemplateNodeCacheKey,
    node: TemplateNode,
}

impl Borrow<TemplateNodeCacheKey> for TemplateNodeEntry {
    fn borrow(&self) -> &TemplateNodeCacheKey {
        &self.key
    }
}

impl PartialEq for TemplateNodeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for TemplateNodeEntry {}

impl Hash for TemplateNodeEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

#[derive(Clone)]
struct TemplateAttrSliceEntry {
    key: TemplateAttrSliceCacheKey,
    attrs: &'static [TemplateAttribute],
}

impl Borrow<TemplateAttrSliceCacheKey> for TemplateAttrSliceEntry {
    fn borrow(&self) -> &TemplateAttrSliceCacheKey {
        &self.key
    }
}

impl PartialEq for TemplateAttrSliceEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for TemplateAttrSliceEntry {}

impl Hash for TemplateAttrSliceEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

#[derive(Clone)]
struct PathListEntry {
    paths: Vec<Vec<u8>>,
    leaked: &'static [&'static [u8]],
}

impl Borrow<[Vec<u8>]> for PathListEntry {
    fn borrow(&self) -> &[Vec<u8>] {
        &self.paths
    }
}

impl PartialEq for PathListEntry {
    fn eq(&self, other: &Self) -> bool {
        self.paths == other.paths
    }
}

impl Eq for PathListEntry {}

impl Hash for PathListEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.paths.hash(state);
    }
}

#[derive(Clone)]
struct PathEntry {
    path: Vec<u8>,
    leaked: &'static [u8],
}

impl Borrow<[u8]> for PathEntry {
    fn borrow(&self) -> &[u8] {
        &self.path
    }
}

impl PartialEq for PathEntry {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for PathEntry {}

impl Hash for PathEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

#[derive(Clone)]
struct StaticString {
    text: String,
    leaked: &'static str,
}

impl Borrow<str> for StaticString {
    fn borrow(&self) -> &str {
        &self.text
    }
}

impl PartialEq for StaticString {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Eq for StaticString {}

impl Hash for StaticString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

fn intern_template_node_slice(
    specs: &[TemplateNodeSpec],
    dynamic_base: usize,
    attr_base: usize,
) -> &'static [TemplateNode] {
    if specs.is_empty() {
        return &[];
    }

    static CACHE: InternSet<TemplateNodeSliceEntry> = InternSet::new();
    let key = TemplateNodeSliceCacheKey {
        specs: specs.iter().map(TemplateNodeSpec::shape).collect(),
        dynamic_base,
        attr_base,
    };
    CACHE
        .get_or_insert_with(&key, || {
            let mut dynamic_base = key.dynamic_base;
            let mut attr_base = key.attr_base;
            let mut nodes = Vec::with_capacity(key.specs.len());
            for spec in &key.specs {
                nodes.push(intern_template_node(spec, dynamic_base, attr_base));
                dynamic_base += spec.dynamic_count();
                attr_base += spec.attr_count();
            }
            TemplateNodeSliceEntry {
                key: key.clone(),
                nodes: Box::leak(nodes.into_boxed_slice()),
            }
        })
        .nodes
}

fn intern_template_node(
    spec: &TemplateNodeShape,
    dynamic_base: usize,
    attr_base: usize,
) -> TemplateNode {
    static CACHE: InternSet<TemplateNodeEntry> = InternSet::new();
    let key = TemplateNodeCacheKey {
        spec: spec.clone(),
        dynamic_base,
        attr_base,
    };
    CACHE
        .get_or_insert_with(&key, || TemplateNodeEntry {
            node: compile_template_node(&key),
            key: key.clone(),
        })
        .node
}

fn compile_template_node(key: &TemplateNodeCacheKey) -> TemplateNode {
    match &key.spec {
        TemplateNodeShape::Element {
            tag,
            namespace,
            attrs,
            children,
        } => {
            let static_attrs = intern_template_attr_shape_slice(attrs, key.attr_base);
            let children_attr_base = key.attr_base + dynamic_attr_count(attrs);
            TemplateNode::Element {
                tag: tag_name(*tag),
                namespace: namespace.map(namespace_name),
                attrs: static_attrs,
                children: intern_template_node_shape_slice(
                    children,
                    key.dynamic_base,
                    children_attr_base,
                ),
            }
        }
        TemplateNodeShape::Text(value) => TemplateNode::Text {
            text: text_value(*value),
        },
        TemplateNodeShape::Dynamic => TemplateNode::Dynamic {
            id: key.dynamic_base,
        },
    }
}

fn intern_template_node_shape_slice(
    specs: &[TemplateNodeShape],
    dynamic_base: usize,
    attr_base: usize,
) -> &'static [TemplateNode] {
    if specs.is_empty() {
        return &[];
    }

    static CACHE: InternSet<TemplateNodeSliceEntry> = InternSet::new();
    let key = TemplateNodeSliceCacheKey {
        specs: specs.to_vec(),
        dynamic_base,
        attr_base,
    };
    CACHE
        .get_or_insert_with(&key, || {
            let mut dynamic_base = key.dynamic_base;
            let mut attr_base = key.attr_base;
            let mut nodes = Vec::with_capacity(key.specs.len());
            for spec in &key.specs {
                nodes.push(intern_template_node(spec, dynamic_base, attr_base));
                dynamic_base += spec.dynamic_count();
                attr_base += spec.attr_count();
            }
            TemplateNodeSliceEntry {
                key: key.clone(),
                nodes: Box::leak(nodes.into_boxed_slice()),
            }
        })
        .nodes
}

#[cfg(test)]
fn intern_template_attr_slice(
    attrs: &[TemplateAttrSpec],
    attr_base: usize,
) -> &'static [TemplateAttribute] {
    let attrs = attrs
        .iter()
        .map(TemplateAttrSpec::shape)
        .collect::<Vec<_>>();
    intern_template_attr_shape_slice(&attrs, attr_base)
}

fn intern_template_attr_shape_slice(
    attrs: &[TemplateAttrShape],
    attr_base: usize,
) -> &'static [TemplateAttribute] {
    if attrs.is_empty() {
        return &[];
    }

    static CACHE: InternSet<TemplateAttrSliceEntry> = InternSet::new();
    let key = TemplateAttrSliceCacheKey {
        attrs: attrs.to_vec(),
        attr_base,
    };
    CACHE
        .get_or_insert_with(&key, || {
            let mut next_attr = key.attr_base;
            let mut static_attrs = Vec::new();
            let mut dynamic_attrs = Vec::new();
            for attr in &key.attrs {
                match attr {
                    TemplateAttrShape::Static {
                        name,
                        value,
                        namespace,
                    } => {
                        let name = attr_name(*name);
                        static_attrs.push((
                            name,
                            TemplateAttribute::Static {
                                name,
                                value: attr_static_value(*value),
                                namespace: namespace.map(namespace_name),
                            },
                        ));
                    }
                    TemplateAttrShape::Dynamic => {
                        let id = next_attr;
                        next_attr += 1;
                        dynamic_attrs.push(TemplateAttribute::Dynamic { id });
                    }
                }
            }
            static_attrs.sort_by(|(left, _), (right, _)| left.cmp(right));
            let attrs = static_attrs
                .into_iter()
                .map(|(_, attr)| attr)
                .chain(dynamic_attrs)
                .collect::<Vec<_>>();
            TemplateAttrSliceEntry {
                key: key.clone(),
                attrs: Box::leak(attrs.into_boxed_slice()),
            }
        })
        .attrs
}

fn dynamic_attr_count(attrs: &[TemplateAttrShape]) -> usize {
    attrs
        .iter()
        .filter(|attr| matches!(attr, TemplateAttrShape::Dynamic))
        .count()
}

fn collect_node_paths(roots: &[TemplateNodeSpec]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for (index, root) in roots.iter().enumerate() {
        let path = vec![index as u8];
        collect_node_paths_from_node(root, path, &mut out);
    }
    out
}

fn collect_node_paths_from_node(node: &TemplateNodeSpec, path: Vec<u8>, out: &mut Vec<Vec<u8>>) {
    match node {
        TemplateNodeSpec::Dynamic(_) => out.push(path),
        TemplateNodeSpec::Element { children, .. } => {
            for (index, child) in children.iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(index as u8);
                collect_node_paths_from_node(child, child_path, out);
            }
        }
        TemplateNodeSpec::Text(_) => {}
    }
}

fn collect_attr_paths(roots: &[TemplateNodeSpec]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for (index, root) in roots.iter().enumerate() {
        let path = vec![index as u8];
        collect_attr_paths_from_node(root, path, &mut out);
    }
    out
}

fn collect_attr_paths_from_node(node: &TemplateNodeSpec, path: Vec<u8>, out: &mut Vec<Vec<u8>>) {
    let TemplateNodeSpec::Element {
        attrs, children, ..
    } = node
    else {
        return;
    };

    for attr in attrs {
        if matches!(attr, TemplateAttrSpec::Dynamic(_)) {
            out.push(path.clone());
        }
    }

    for (index, child) in children.iter().enumerate() {
        let mut child_path = path.clone();
        child_path.push(index as u8);
        collect_attr_paths_from_node(child, child_path, out);
    }
}

fn intern_path_list(paths: Vec<Vec<u8>>) -> &'static [&'static [u8]] {
    if paths.is_empty() {
        return &[];
    }

    static CACHE: InternSet<PathListEntry> = InternSet::new();
    CACHE
        .get_or_insert_with(paths.as_slice(), || {
            let leaked = paths.iter().cloned().map(intern_path).collect::<Vec<_>>();
            PathListEntry {
                paths: paths.clone(),
                leaked: Box::leak(leaked.into_boxed_slice()),
            }
        })
        .leaked
}

fn intern_path(path: Vec<u8>) -> &'static [u8] {
    if path.is_empty() {
        return &[];
    }

    static CACHE: InternSet<PathEntry> = InternSet::new();
    CACHE
        .get_or_insert_with(path.as_slice(), || PathEntry {
            leaked: Box::leak(path.clone().into_boxed_slice()),
            path: path.clone(),
        })
        .leaked
}

fn leak_str(value: String) -> &'static str {
    static CACHE: InternSet<StaticString> = InternSet::new();
    CACHE
        .get_or_insert_with(value.as_str(), || StaticString {
            leaked: Box::leak(value.clone().into_boxed_str()),
            text: value.clone(),
        })
        .leaked
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

fn dynamic_attr_name(slot: usize, value: u8) -> &'static str {
    if value & 0x80 == 0 {
        attr_name(value)
    } else {
        listener_name(slot, value & 0x7f)
    }
}

fn listener_name(slot: usize, value: u8) -> &'static str {
    leak_str(format!("onevent{slot}_{value}"))
}

fn attr_static_value(value: u8) -> &'static str {
    // Reserve high static values for aliasing dynamic text attributes.
    if value >= 128 {
        return leak_str(format!("attr-value-{}", value - 128));
    }

    leak_str(format!("static{value}"))
}

fn text_value(value: u8) -> &'static str {
    leak_str(format!("static-text-{value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    fn element(
        tag: u8,
        attrs: Vec<TemplateAttrSpec>,
        children: Vec<TemplateNodeSpec>,
    ) -> TemplateNodeSpec {
        TemplateNodeSpec::Element {
            tag,
            namespace: None,
            attrs,
            children,
        }
    }

    #[test]
    fn identical_expanded_templates_reuse_static_parts() {
        let spec = TemplateSpec {
            cache_key: None,
            roots: vec![element(
                1,
                vec![TemplateAttrSpec::Dynamic(Vec::new())],
                vec![TemplateNodeSpec::Dynamic(DynamicSpec::Empty)],
            )],
        };

        let first = compile_template(&spec);
        let second = compile_template(&spec);

        assert!(ptr::eq(first.roots(), second.roots()));
        assert!(ptr::eq(first.node_paths(), second.node_paths()));
        assert!(ptr::eq(first.attr_paths(), second.attr_paths()));
    }

    #[test]
    fn related_templates_reuse_shared_child_slices() {
        let shared_child = element(
            9,
            vec![TemplateAttrSpec::Dynamic(Vec::new())],
            vec![TemplateNodeSpec::Dynamic(DynamicSpec::Empty)],
        );
        let first = compile_template(&TemplateSpec {
            cache_key: None,
            roots: vec![element(1, Vec::new(), vec![shared_child.clone()])],
        });
        let second = compile_template(&TemplateSpec {
            cache_key: None,
            roots: vec![element(2, Vec::new(), vec![shared_child])],
        });

        let [
            TemplateNode::Element {
                children: first_children,
                ..
            },
        ] = first.roots()
        else {
            panic!("expected first root element");
        };
        let [
            TemplateNode::Element {
                children: second_children,
                ..
            },
        ] = second.roots()
        else {
            panic!("expected second root element");
        };

        assert!(ptr::eq(*first_children, *second_children));
    }

    #[test]
    fn dynamic_subtrees_include_dynamic_base_in_key() {
        let spec = element(
            1,
            Vec::new(),
            vec![TemplateNodeSpec::Dynamic(DynamicSpec::Empty)],
        );

        let base_zero = intern_template_node(&spec.shape(), 0, 0);
        let base_one = intern_template_node(&spec.shape(), 1, 0);

        let TemplateNode::Element {
            children: [TemplateNode::Dynamic { id: zero_id }],
            ..
        } = base_zero
        else {
            panic!("expected base zero dynamic child");
        };
        let TemplateNode::Element {
            children: [TemplateNode::Dynamic { id: one_id }],
            ..
        } = base_one
        else {
            panic!("expected base one dynamic child");
        };

        assert_eq!(*zero_id, 0);
        assert_eq!(*one_id, 1);
    }

    #[test]
    fn dynamic_attr_slices_include_attr_base_in_key() {
        let attrs = [TemplateAttrSpec::Dynamic(Vec::new())];

        let base_zero = intern_template_attr_slice(&attrs, 0);
        let base_one = intern_template_attr_slice(&attrs, 1);

        assert!(matches!(base_zero, [TemplateAttribute::Dynamic { id: 0 }]));
        assert!(matches!(base_one, [TemplateAttribute::Dynamic { id: 1 }]));
        assert!(!ptr::eq(base_zero, base_one));
    }
}
