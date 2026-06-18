#![allow(non_snake_case)]

use crate::{
    cache::InternMap,
    context::{HarnessContext, SuspenseReadyFuture},
    lifecycle::LifecycleRole,
    model::*,
};
use dioxus::prelude::*;
#[cfg(test)]
use dioxus_core::internal::DecodedTemplateOp;
use dioxus_core::internal::RuntimeTemplateBuilder;
use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, DynamicValue, Portal, Runtime, Task, Template,
    VComponent, VNode, VText,
};
use std::future::pending;

pub(crate) fn App(context: HarnessContext) -> Element {
    let model = context.read_model();
    Ok(build_vnode(&context, &model.root))
}

#[derive(Clone, PartialEq, Props)]
struct GeneratedProps {
    context: HarnessContext,
    id: u64,
    suspense_ancestors: Vec<u64>,
    node: VNodeSpec,
}

#[derive(Clone, PartialEq, Props)]
struct GeneratedSuspenseProps {
    context: HarnessContext,
    id: u64,
    ready_generation: u64,
    required_ready_wake_count: usize,
    mode: SuspenseMode,
    wake_mutation: WakeMutationSpec,
    wake_applied: bool,
    suspense_ancestors: Vec<u64>,
    child: VNodeSpec,
}

#[derive(Clone, PartialEq, Props)]
struct GeneratedPortalProps {
    context: HarnessContext,
    suspense_ancestors: Vec<u64>,
    child: VNodeSpec,
}

#[derive(Clone, Copy)]
struct RenderScopeContext;

#[derive(Clone, Copy)]
struct RenderRootContext;

fn GeneratedPortal(props: GeneratedPortalProps) -> Element {
    // Each generated portal scope allocates its own real render target. We
    // intentionally never register a `WriteMutations` writer for that target
    // in the harness — the diff dispatcher silently drops mutations destined
    // for a target with no writer. This exercises the "writes enabled"
    // branches of `Portal::{create,diff,remove}` and the generic diff helpers
    // (`at_anchor`, `create_at_anchor_with_parents`, `create_with_parents`
    // with `state.to = Some(_)`) without interleaving the portal body's edits
    // into the outer ROOT oracle and diverging from the fresh-render
    // comparison.
    let target = use_hook(|| Runtime::current().create_render_target());
    let context = props.context.clone();
    let suspense_ancestors = props.suspense_ancestors.clone();
    let child_spec = props.child.clone();
    let child = build_vnode_with_suspense(&context, &child_spec, &suspense_ancestors);
    rsx! {
        Portal {
            target: target,
            {child}
        }
    }
}

fn GeneratedSuspenseFallback() -> Element {
    exercise_scope_render_apis(false);
    rsx! { "suspense-fallback" }
}

fn GeneratedComponent(props: GeneratedProps) -> Element {
    let context = props.context;
    track_lifecycle(
        &context,
        LifecycleRole::ComponentA,
        props.id,
        &props.suspense_ancestors,
    );
    exercise_scope_render_apis(!props.suspense_ancestors.is_empty());
    Ok(build_vnode_with_suspense(
        &context,
        &props.node,
        &props.suspense_ancestors,
    ))
}

fn OtherGeneratedComponent(props: GeneratedProps) -> Element {
    let context = props.context;
    track_lifecycle(
        &context,
        LifecycleRole::ComponentB,
        props.id,
        &props.suspense_ancestors,
    );
    exercise_scope_render_apis(!props.suspense_ancestors.is_empty());
    Ok(build_vnode_with_suspense(
        &context,
        &props.node,
        &props.suspense_ancestors,
    ))
}

fn GeneratedSuspenseBoundary(props: GeneratedSuspenseProps) -> Element {
    let context = props.context;
    track_lifecycle(
        &context,
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
                fallback: |_| rsx! { GeneratedSuspenseFallback {} },
                GeneratedSuspenseChild {
                    context,
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
    let child = build_suspense_child_vnode(
        &context,
        &child_spec,
        &child_suspense_ancestors,
        wake_mutation,
        wake_applied,
    );
    let wake_not_applied = false;
    rsx! {
        SuspenseBoundary {
            fallback: |_| rsx! { GeneratedSuspenseFallback {} },
            GeneratedSuspenseChild {
                context: context.clone(),
                id,
                ready_generation,
                required_ready_wake_count,
                mode,
                wake_mutation: WakeMutationSpec::None,
                wake_applied: wake_not_applied,
                suspense_ancestors,
                child: VNodeSpec::minimal(),
            }
            {child}
        }
    }
}

fn GeneratedSuspenseChild(props: GeneratedSuspenseProps) -> Element {
    let context = props.context;
    track_lifecycle(
        &context,
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
                    let task_context = context.clone();
                    let new_task = spawn(async move {
                        SuspenseReadyFuture {
                            context: task_context.clone(),
                            key,
                            required_wakes,
                        }
                        .await;
                        let wake_mutation =
                            task_context.read_model().wake_mutation_for_ready_key(key);
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
        &context,
        &props.child,
        &child_suspense_ancestors,
        wake_mutation,
        props.wake_applied || local_wake_mutation != WakeMutationSpec::None,
    ))
}

fn track_lifecycle(
    context: &HarnessContext,
    role: LifecycleRole,
    id: u64,
    suspense_ancestors: &[u64],
) {
    let suspense_ancestors = suspense_ancestors.to_vec();
    let context = context.clone();
    let guard = use_hook({
        let suspense_ancestors = suspense_ancestors.clone();
        let context = context.clone();
        move || context.lifecycle.track(role, id, &suspense_ancestors)
    });
    guard.update(role, id, &suspense_ancestors);
}

fn exercise_scope_render_apis(schedule_task_update: bool) {
    let scope = dioxus_core::current_scope_id();
    let _ = format!("{scope:?}");
    let _ = dioxus_core::generation();

    dioxus_core::provide_context(RenderScopeContext);
    dioxus_core::provide_context(RenderScopeContext);
    let _ = dioxus_core::has_context::<RenderScopeContext>();
    let _ = dioxus_core::try_consume_context::<RenderScopeContext>();
    let _ = dioxus_core::consume_context::<RenderScopeContext>();
    let _ = dioxus_core::consume_context_from_scope::<RenderScopeContext>(scope);

    dioxus_core::provide_root_context(RenderRootContext);
    dioxus_core::provide_root_context(RenderRootContext);
    let _ = dioxus_core::try_consume_context::<RenderRootContext>();
    let _ =
        dioxus_core::consume_context_from_scope::<RenderRootContext>(dioxus_core::ScopeId::ROOT);

    let _ = dioxus_core::schedule_update();
    let _ = dioxus_core::schedule_update_any();
    let _: Task = use_hook(move || {
        dioxus_core::queue_effect(|| {});
        if schedule_task_update {
            dioxus_core::spawn_isomorphic(async {
                dioxus_core::needs_update();
            })
        } else {
            dioxus_core::spawn_isomorphic(async {})
        }
    });
}

fn build_suspense_child_vnode(
    context: &HarnessContext,
    child: &VNodeSpec,
    suspense_ancestors: &[u64],
    wake_mutation: WakeMutationSpec,
    wake_applied: bool,
) -> VNode {
    let child = build_vnode_with_suspense(context, child, suspense_ancestors);
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
        Box::new([DynamicValue::Node(DynamicNode::Fragment(vec![child]))]),
    )
}

fn vnode_contains_suspense(spec: &VNodeSpec) -> bool {
    spec.template
        .roots
        .iter()
        .any(template_node_contains_suspense)
}

fn template_node_contains_suspense(spec: &TemplateNodeSpec) -> bool {
    match spec {
        TemplateNodeSpec::Element { children, .. } => {
            children.iter().any(template_node_contains_suspense)
        }
        TemplateNodeSpec::Dynamic(DynamicSpec::Fragment(nodes)) => {
            nodes.iter().any(vnode_contains_suspense)
        }
        TemplateNodeSpec::Dynamic(
            DynamicSpec::ComponentA(component) | DynamicSpec::ComponentB(component),
        ) => vnode_contains_suspense(&component.child),
        TemplateNodeSpec::Dynamic(DynamicSpec::Suspense(_)) => true,
        TemplateNodeSpec::Dynamic(DynamicSpec::Portal(child)) => vnode_contains_suspense(child),
        TemplateNodeSpec::Text(_) | TemplateNodeSpec::Dynamic(_) => false,
    }
}

fn build_vnode(context: &HarnessContext, spec: &VNodeSpec) -> VNode {
    build_vnode_with_suspense(context, spec, &[])
}

fn build_vnode_with_suspense(
    context: &HarnessContext,
    spec: &VNodeSpec,
    suspense_ancestors: &[u64],
) -> VNode {
    let spec = spec.clone().normalize();
    let template = compile_template(&spec.template);
    let dynamic_nodes = spec
        .template
        .dynamics()
        .into_iter()
        .map(|dynamic| build_dynamic(context, dynamic, suspense_ancestors))
        .collect::<Vec<_>>();
    let dynamic_attrs = spec
        .template
        .dynamic_attr_lists()
        .into_iter()
        .enumerate()
        .map(|(slot, attrs)| {
            attrs
                .iter()
                .map(|attr| build_attr(context, slot, attr))
                .collect::<Box<[Attribute]>>()
        })
        .collect::<Vec<_>>();
    let dynamic_slots = dynamic_slots_for_template(&spec.template, &template);

    let dynamic_values: Vec<DynamicValue> = dynamic_slots
        .into_iter()
        .map(|slot| match slot {
            FuzzDynamicSlot::Node(id) => DynamicValue::Node(dynamic_nodes[id].clone()),
            FuzzDynamicSlot::Attrs(id) => DynamicValue::Attrs(dynamic_attrs[id].clone()),
        })
        .collect();

    VNode::new(
        spec.key.map(|key| format!("k{key}")),
        template,
        dynamic_values.into_boxed_slice(),
    )
}

#[derive(Clone, Copy, Debug)]
enum FuzzDynamicSlot {
    Node(usize),
    Attrs(usize),
}

/// Compute the node/attribute classification of each dynamic value in document order, mirroring the
/// raw-op emission in [`FuzzRawTemplateBuilder`] (dynamic attributes before children). This is the
/// fuzz-side source of truth for slot kinds; the lowered template no longer records it.
fn dynamic_slots_for_template(spec: &TemplateSpec, _template: &Template) -> Vec<FuzzDynamicSlot> {
    let shapes = spec
        .roots
        .iter()
        .map(TemplateNodeSpec::shape)
        .collect::<Vec<_>>();
    let mut slots = Vec::new();
    let mut next_node = 0;
    let mut next_attr = 0;
    for shape in &shapes {
        collect_fuzz_slots(shape, &mut slots, &mut next_node, &mut next_attr);
    }
    slots
}

fn collect_fuzz_slots(
    node: &TemplateNodeShape,
    slots: &mut Vec<FuzzDynamicSlot>,
    next_node: &mut usize,
    next_attr: &mut usize,
) {
    match node {
        TemplateNodeShape::Element {
            attrs, children, ..
        } => {
            for attr in attrs {
                if matches!(attr, TemplateAttrShape::Dynamic) {
                    slots.push(FuzzDynamicSlot::Attrs(*next_attr));
                    *next_attr += 1;
                }
            }
            for child in children {
                collect_fuzz_slots(child, slots, next_node, next_attr);
            }
        }
        TemplateNodeShape::Dynamic => {
            slots.push(FuzzDynamicSlot::Node(*next_node));
            *next_node += 1;
        }
        TemplateNodeShape::Text(_) => {}
    }
}

fn build_dynamic(
    context: &HarnessContext,
    spec: &DynamicSpec,
    suspense_ancestors: &[u64],
) -> DynamicNode {
    match spec {
        DynamicSpec::Empty => DynamicNode::Fragment(Vec::new()),
        DynamicSpec::Text(value) => DynamicNode::Text(VText::new(format!("text-{value}"))),
        DynamicSpec::Placeholder => DynamicNode::Fragment(Vec::new()),
        DynamicSpec::Fragment(nodes) => DynamicNode::Fragment(
            nodes
                .iter()
                .map(|node| build_vnode_with_suspense(context, node, suspense_ancestors))
                .collect(),
        ),
        DynamicSpec::ComponentA(component) => DynamicNode::Component(VComponent::new(
            GeneratedComponent,
            GeneratedProps {
                context: context.clone(),
                id: component.id,
                suspense_ancestors: suspense_ancestors.to_vec(),
                node: component.child.as_ref().clone(),
            },
            "GeneratedComponent",
        )),
        DynamicSpec::ComponentB(component) => DynamicNode::Component(VComponent::new(
            OtherGeneratedComponent,
            GeneratedProps {
                context: context.clone(),
                id: component.id,
                suspense_ancestors: suspense_ancestors.to_vec(),
                node: component.child.as_ref().clone(),
            },
            "OtherGeneratedComponent",
        )),
        DynamicSpec::Suspense(spec) => DynamicNode::Component(VComponent::new(
            GeneratedSuspenseBoundary,
            GeneratedSuspenseProps {
                context: context.clone(),
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
        DynamicSpec::Portal(child) => {
            // All generated portals share the ROOT render target so the harness'
            // single oracle observes mutations from both the outer tree and the
            // portal bodies. The portal scope still flows through the portal
            // driver's create/diff/remove regardless of whether the target
            // ultimately differs.
            DynamicNode::Component(VComponent::new(
                GeneratedPortal,
                GeneratedPortalProps {
                    context: context.clone(),
                    suspense_ancestors: suspense_ancestors.to_vec(),
                    child: child.as_ref().clone(),
                },
                "GeneratedPortal",
            ))
        }
    }
}

fn build_attr(context: &HarnessContext, slot: usize, spec: &AttrSpec) -> Attribute {
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
        AttrValueSpec::Int(value) => Attribute::new(
            dynamic_attr_name(slot, spec.name),
            value as i64,
            namespace,
            spec.volatile,
        ),
        AttrValueSpec::Bool(value) => Attribute::new(
            dynamic_attr_name(slot, spec.name),
            value,
            namespace,
            spec.volatile,
        ),
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
        AttrValueSpec::Listener => {
            let events = context.events.clone();
            Attribute::new(
                listener_name(slot, spec.name),
                AttributeValue::listener(move |_: Event<String>| events.handle_listener_event()),
                None,
                spec.volatile,
            )
        }
    }
}

fn compile_template(spec: &TemplateSpec) -> Template {
    static CACHE: InternMap<TemplateCacheKey, Template> = InternMap::new();

    CACHE.get_or_insert_with(&spec.cache_key(), || compile_template_uncached(spec))
}

fn compile_template_uncached(spec: &TemplateSpec) -> Template {
    let shapes = spec
        .roots
        .iter()
        .map(TemplateNodeSpec::shape)
        .collect::<Vec<_>>();
    compile_flat_template(&shapes)
}

fn compile_flat_template(roots: &[TemplateNodeShape]) -> Template {
    let mut builder = FuzzTemplateBuilder::default();
    builder.push_roots(roots);
    builder.template.finish()
}

#[derive(Default)]
struct FuzzTemplateBuilder {
    template: RuntimeTemplateBuilder,
}

impl FuzzTemplateBuilder {
    fn push_roots(&mut self, roots: &[TemplateNodeShape]) {
        for (index, root) in roots.iter().enumerate() {
            self.push_node(root, Self::siblings_have_static_node(roots, index + 1));
        }
    }

    fn push_node(&mut self, node: &TemplateNodeShape, following_static_at_parent: bool) {
        match node {
            TemplateNodeShape::Element {
                tag,
                namespace,
                attrs,
                children,
            } => self.push_element(*tag, *namespace, attrs, children),
            TemplateNodeShape::Text(value) => self.template.static_text(text_value(*value)),
            TemplateNodeShape::Dynamic => self.template.dynamic_node(following_static_at_parent),
        }
    }

    fn push_element(
        &mut self,
        tag: u8,
        namespace: Option<u8>,
        attrs: &[TemplateAttrShape],
        children: &[TemplateNodeShape],
    ) {
        self.template
            .open_element(tag_name(tag), namespace.map(namespace_name));

        let mut static_attrs = Vec::new();
        let mut dynamic_attr_count = 0usize;
        for attr in attrs {
            match attr {
                TemplateAttrShape::Static {
                    name,
                    value,
                    namespace,
                } => static_attrs.push((
                    attr_name(*name),
                    attr_static_value(*value),
                    namespace.map(namespace_name),
                )),
                TemplateAttrShape::Dynamic => dynamic_attr_count += 1,
            }
        }
        static_attrs.sort_by_key(|(name, _, _)| *name);

        for (name, value, namespace) in static_attrs {
            self.template.static_attr(name, value, namespace);
        }

        for _ in 0..dynamic_attr_count {
            self.template.dynamic_attr();
        }

        for (index, child) in children.iter().enumerate() {
            self.push_node(child, Self::siblings_have_static_node(children, index + 1));
        }

        self.template.close_element();
    }

    fn siblings_have_static_node(nodes: &[TemplateNodeShape], start: usize) -> bool {
        nodes[start..].iter().any(Self::node_has_static_root)
    }

    fn node_has_static_root(node: &TemplateNodeShape) -> bool {
        matches!(
            node,
            TemplateNodeShape::Element { .. } | TemplateNodeShape::Text(_)
        )
    }
}

fn leak_str(value: String) -> &'static str {
    static CACHE: InternMap<String, &'static str> = InternMap::new();
    CACHE.get_or_insert_with(value.as_str(), || Box::leak(value.clone().into_boxed_str()))
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
    fn identical_expanded_templates_reuse_packed_parts() {
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

        // A shared `ops` pointer proves the interning cache returned the identical leaked template
        // (a fresh compile would leak new slices).
        assert!(ptr::eq(first.ops(), second.ops()));
        assert!(ptr::eq(first.strings(), second.strings()));
    }

    #[test]
    fn dynamic_children_leave_only_static_structure_in_the_op_tape() {
        let spec = element(
            1,
            Vec::new(),
            vec![TemplateNodeSpec::Dynamic(DynamicSpec::Empty)],
        );

        let template = compile_template(&TemplateSpec {
            cache_key: None,
            roots: vec![spec],
        });

        // The dynamic child no longer appears in the op tape; only the static element remains.
        let decoded_ops = template
            .ops()
            .iter()
            .map(|op| op.decode())
            .collect::<Vec<_>>();
        assert_eq!(
            decoded_ops,
            vec![
                DecodedTemplateOp::Enter {
                    skip: 2,
                    namespace: false
                },
                DecodedTemplateOp::Static(0),
            ]
        );
        assert_eq!(template.strings()[0], "tag1");
        assert_eq!(template.root_count(), 1);
        assert_eq!(template.dynamic_value_count(), 1);
    }

    #[test]
    fn static_attrs_are_sorted_before_dynamic_attrs() {
        let template = compile_template(&TemplateSpec {
            cache_key: None,
            roots: vec![element(
                1,
                vec![
                    TemplateAttrSpec::Dynamic(Vec::new()),
                    TemplateAttrSpec::Static {
                        name: 2,
                        value: 3,
                        namespace: None,
                    },
                    TemplateAttrSpec::Static {
                        name: 1,
                        value: 4,
                        namespace: None,
                    },
                    TemplateAttrSpec::Dynamic(Vec::new()),
                ],
                Vec::new(),
            )],
        });

        // Static attributes are emitted into the op tape sorted by name; dynamic attributes leave
        // the tape entirely and live in the anchor table (2 dynamic attribute values here).
        assert_eq!(
            template.static_attr_at_op(2),
            Some(("attr1", "static4", None))
        );
        assert_eq!(
            template.static_attr_at_op(5),
            Some(("attr2", "static3", None))
        );
        assert_eq!(template.dynamic_value_count(), 2);
    }
}
