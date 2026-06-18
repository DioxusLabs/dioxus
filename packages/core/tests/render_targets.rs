use dioxus::prelude::*;
use dioxus_core::{
    ElementId, MultiWriter, Mutation, Mutations, Portal, RenderTargetId, Runtime, VirtualDom,
};
use std::{
    any::Any,
    cell::Cell,
    collections::BTreeMap,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

/// Collect one rebuild into per-target mutation lists. Targets created during
/// the diff lazily get a fresh `Mutations` collector; untouched targets are
/// dropped so tests can assert absence with `get(..) == None`.
fn rebuild_to_targeted_vec(dom: &mut VirtualDom) -> BTreeMap<RenderTargetId, Mutations> {
    let mut writer = CollectingTargetWriter::new();
    dom.rebuild(&mut writer);
    drain_targets(writer.into_targets())
}

/// [`rebuild_to_targeted_vec`], but for one `render_immediate` pass.
fn render_immediate_to_targeted_vec(dom: &mut VirtualDom) -> BTreeMap<RenderTargetId, Mutations> {
    let mut writer = CollectingTargetWriter::new();
    dom.render_immediate(&mut writer);
    drain_targets(writer.into_targets())
}

struct CollectingTargetWriter {
    targets: BTreeMap<RenderTargetId, Mutations>,
}

impl CollectingTargetWriter {
    fn new() -> Self {
        Self { targets: BTreeMap::new() }
    }

    fn into_targets(self) -> BTreeMap<RenderTargetId, Mutations> {
        self.targets
    }
}

impl MultiWriter for CollectingTargetWriter {
    type Writer = Mutations;

    fn writer_for(&mut self, id: RenderTargetId) -> Option<&mut Mutations> {
        Some(self.targets.entry(id).or_default())
    }
}

fn drain_targets(
    targets: BTreeMap<RenderTargetId, Mutations>,
) -> BTreeMap<RenderTargetId, Mutations> {
    targets
        .into_iter()
        .filter(|(_, m)| !m.edits.is_empty())
        .collect()
}

static ROOT_CLICKS: AtomicUsize = AtomicUsize::new(0);
static PORTAL_CLICKS: AtomicUsize = AtomicUsize::new(0);
static RETARGET_CLICKS: AtomicUsize = AtomicUsize::new(0);
static EFFECTS: AtomicUsize = AtomicUsize::new(0);
static WRITERLESS_EFFECTS: AtomicUsize = AtomicUsize::new(0);
static SHOW_PORTAL: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct SharedContext(&'static str);

#[derive(Clone)]
struct TargetSlot(Rc<Cell<RenderTargetId>>);

impl PartialEq for TargetSlot {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl TargetSlot {
    fn new() -> Self {
        Self(Rc::new(Cell::new(RenderTargetId::ROOT)))
    }

    fn get(&self) -> RenderTargetId {
        self.0.get()
    }

    fn set(&self, id: RenderTargetId) {
        self.0.set(id);
    }
}

#[derive(Clone, PartialEq, Props)]
struct AppProps {
    target: TargetSlot,
}

#[derive(Clone, PartialEq, Props)]
struct RetargetProps {
    first: TargetSlot,
    second: TargetSlot,
}

#[derive(Clone, PartialEq, Props)]
struct ReopenProps {
    first: TargetSlot,
    second: TargetSlot,
}

fn noop_app(props: AppProps) -> Element {
    rsx! {
        Portal {
            target: props.target.get(),
            WriterlessEffectChild {}
        }
    }
}

fn context_app(props: AppProps) -> Element {
    use_hook(|| provide_context(SharedContext("shared")));

    rsx! {
        Portal {
            target: props.target.get(),
            ContextChild {}
        }
    }
}

fn retarget_app(props: RetargetProps) -> Element {
    let mut target = use_signal(|| props.first.get());
    let second = props.second.get();

    rsx! {
        div {
            button {
                onclick: move |_| target.set(second),
                "move"
            }
            Portal {
                target: target(),
                button {
                    onclick: move |_| {
                        RETARGET_CLICKS.fetch_add(1, Ordering::SeqCst);
                    },
                    "portal"
                }
            }
        }
    }
}

fn dropped_target_app(props: AppProps) -> Element {
    let mut show = use_signal(|| false);

    rsx! {
        button {
            onclick: move |_| show.set(true),
            "show"
        }
        if show() {
            Portal {
                target: props.target.get(),
                EffectChild {}
            }
        }
    }
}

fn reopen_after_close_app(props: ReopenProps) -> Element {
    let mut window_count = use_signal(|| 0usize);

    rsx! {
        button {
            onclick: move |_| window_count += 1,
            "open"
        }

        for id in 0..window_count() {
            CloseablePortal {
                key: "{id}",
                target: if id == 0 {
                    props.first.get()
                } else {
                    props.second.get()
                },
            }
        }
    }
}

fn dynamic_reopen_after_close_app() -> Element {
    let mut window_count = use_signal(|| 0usize);

    rsx! {
        button {
            onclick: move |_| window_count += 1,
            "open"
        }

        for id in 0..window_count() {
            DynamicCloseablePortal {
                key: "{id}",
            }
        }
    }
}

#[component]
fn CloseablePortal(target: RenderTargetId) -> Element {
    let mut closed = use_signal(|| false);

    if closed() {
        return VNode::empty();
    }

    rsx! {
        Portal {
            target,
            button {
                onclick: move |_| closed.set(true),
                "close"
            }
        }
    }
}

#[component]
fn DynamicCloseablePortal() -> Element {
    let target = use_hook(|| Runtime::current().create_render_target());
    let mut closed = use_signal(|| false);

    if closed() {
        return VNode::empty();
    }

    rsx! {
        Portal {
            target,
            button {
                onclick: move |_| closed.set(true),
                "close"
            }
        }
    }
}

fn replace_portal_app(props: AppProps) -> Element {
    if SHOW_PORTAL.load(Ordering::SeqCst) != 0 {
        rsx! {
            PortalWrapper { target: props.target.get() }
        }
    } else {
        rsx! {
            div { "root" }
        }
    }
}

#[component]
fn PortalWrapper(target: RenderTargetId) -> Element {
    rsx! {
        Portal {
            target,
            div { "portal" }
        }
    }
}

#[component]
fn ContextChild() -> Element {
    let value = consume_context::<SharedContext>();
    assert_eq!(value.0, "shared");

    rsx! { "context child" }
}

#[component]
fn EffectChild() -> Element {
    use_effect(|| {
        EFFECTS.fetch_add(1, Ordering::SeqCst);
    });

    rsx! { "effect child" }
}

#[component]
fn WriterlessEffectChild() -> Element {
    use_effect(|| {
        WRITERLESS_EFFECTS.fetch_add(1, Ordering::SeqCst);
    });

    rsx! { "effect child" }
}

fn click_event() -> Event<dyn Any> {
    Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
        true,
    )
}

fn has_click_listener(mutations: &Mutations, id: ElementId) -> bool {
    mutations.edits.windows(2).any(|window| {
        matches!(
            (&window[0], &window[1]),
            (Mutation::PushId { id: listener_id }, Mutation::NewEventListener { name })
                if *name == "click" && *listener_id == id
        )
    })
}

fn first_click_listener(mutations: &Mutations) -> ElementId {
    mutations
        .edits
        .windows(2)
        .find_map(|window| match (&window[0], &window[1]) {
            (Mutation::PushId { id }, Mutation::NewEventListener { name }) if *name == "click" => {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn removes_id(mutations: &Mutations, id: ElementId) -> bool {
    mutations.edits.windows(2).any(|window| {
        matches!(
            (&window[0], &window[1]),
            (Mutation::PushId { id: removed_id }, Mutation::Remove) if *removed_id == id
        )
    })
}

fn maps_id(mutations: &Mutations, id: ElementId) -> bool {
    mutations
        .edits
        .iter()
        .any(|mutation| matches!(mutation, Mutation::PopId { id: mapped_id } if *mapped_id == id))
}

fn appends_to_root(mutations: &Mutations) -> bool {
    let mut stack_depth = 0usize;
    let mut root_depth = None;

    for mutation in &mutations.edits {
        match mutation {
            Mutation::PushId { id } if *id == ElementId::ROOT => {
                root_depth = Some(stack_depth);
                stack_depth += 1;
            }
            Mutation::PushId { .. }
            | Mutation::CreateElement { .. }
            | Mutation::CreateText { .. } => {
                stack_depth += 1;
            }
            Mutation::AppendChildren { m } => {
                if root_depth.is_some_and(|depth| stack_depth.checked_sub(*m + 1) == Some(depth)) {
                    return true;
                }
                stack_depth -= *m;
            }
            Mutation::InsertAfter { m } | Mutation::InsertBefore { m } => {
                stack_depth -= *m;
            }
            Mutation::ReplaceWith { m } => {
                stack_depth -= *m + 1;
            }
            Mutation::PopId { .. } | Mutation::Pop | Mutation::Remove => {
                stack_depth -= 1;
                if root_depth.is_some_and(|depth| depth >= stack_depth) {
                    root_depth = None;
                }
            }
            _ => {}
        }
    }
    false
}

fn app(props: AppProps) -> Element {
    rsx! {
        div {
            onclick: move |_| {
                ROOT_CLICKS.fetch_add(1, Ordering::SeqCst);
            },
            Portal {
                target: props.target.get(),
                button {
                    onclick: move |_| {
                        PORTAL_CLICKS.fetch_add(1, Ordering::SeqCst);
                    },
                    "portal"
                }
            }
        }
    }
}

#[test]
fn portal_targets_have_isolated_element_arenas_and_logical_event_bubbling() {
    ROOT_CLICKS.store(0, Ordering::SeqCst);
    PORTAL_CLICKS.store(0, Ordering::SeqCst);
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    let target_slot = TargetSlot::new();
    let mut dom = VirtualDom::new_with_props(app, AppProps { target: target_slot.clone() });
    let target = dom.runtime().create_render_target();
    target_slot.set(target);

    let edits = rebuild_to_targeted_vec(&mut dom);

    let root_edits = edits.get(&RenderTargetId::ROOT).unwrap();
    let portal_edits = edits.get(&target).unwrap();

    assert!(has_click_listener(root_edits, ElementId::from_raw(1)));
    assert!(has_click_listener(portal_edits, ElementId::from_raw(1)));

    dom.runtime()
        .handle_event("click", click_event(), ElementId::from_raw(1));
    assert_eq!(ROOT_CLICKS.load(Ordering::SeqCst), 1);
    assert_eq!(PORTAL_CLICKS.load(Ordering::SeqCst), 0);

    dom.runtime()
        .handle_event_for_target(target, "click", click_event(), ElementId::from_raw(1));
    assert_eq!(PORTAL_CLICKS.load(Ordering::SeqCst), 1);
    assert_eq!(ROOT_CLICKS.load(Ordering::SeqCst), 2);
}

#[test]
fn writerless_targets_drop_writes_but_mount_effects() {
    WRITERLESS_EFFECTS.store(0, Ordering::SeqCst);

    let target_slot = TargetSlot::new();
    let mut dom = VirtualDom::new_with_props(noop_app, AppProps { target: target_slot.clone() });
    let target = dom.runtime().create_render_target();
    target_slot.set(target);

    // A plain writer serves only the root target, so the portal target's
    // mutations are dropped by the target router. The portal still mounts
    // logically and runs effects.
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.process_events();
    dom.render_immediate(&mut dioxus_core::NoOpMutations);

    assert_eq!(WRITERLESS_EFFECTS.load(Ordering::SeqCst), 1);
}

#[test]
fn portal_children_keep_scope_context() {
    let target_slot = TargetSlot::new();
    let mut dom = VirtualDom::new_with_props(context_app, AppProps { target: target_slot.clone() });
    let target = dom.runtime().create_render_target();
    target_slot.set(target);

    let edits = rebuild_to_targeted_vec(&mut dom);
    assert!(edits.contains_key(&target));
}

#[test]
fn retargeting_portal_drops_and_recreates_target_subtree() {
    RETARGET_CLICKS.store(0, Ordering::SeqCst);
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    let first_slot = TargetSlot::new();
    let second_slot = TargetSlot::new();
    let mut dom = VirtualDom::new_with_props(
        retarget_app,
        RetargetProps { first: first_slot.clone(), second: second_slot.clone() },
    );
    let first = dom.runtime().create_render_target();
    let second = dom.runtime().create_render_target();
    first_slot.set(first);
    second_slot.set(second);

    let edits = rebuild_to_targeted_vec(&mut dom);
    let move_button = first_click_listener(edits.get(&RenderTargetId::ROOT).unwrap());
    let first_portal_button = first_click_listener(edits.get(&first).unwrap());
    assert!(has_click_listener(
        edits.get(&first).unwrap(),
        first_portal_button
    ));

    dom.runtime()
        .handle_event("click", click_event(), move_button);

    let edits = render_immediate_to_targeted_vec(&mut dom);

    let first_edits = edits
        .get(&first)
        .unwrap_or_else(|| panic!("missing first target edits: {edits:#?}"));
    assert!(removes_id(first_edits, first_portal_button));
    let second_edits = edits.get(&second).unwrap();
    let second_portal_button = first_click_listener(second_edits);
    assert!(has_click_listener(second_edits, second_portal_button));

    dom.runtime()
        .handle_event_for_target(second, "click", click_event(), second_portal_button);
    assert_eq!(RETARGET_CLICKS.load(Ordering::SeqCst), 1);
}

#[test]
fn replacing_portal_with_local_node_removes_old_target_subtree() {
    SHOW_PORTAL.store(1, Ordering::SeqCst);

    let target_slot = TargetSlot::new();
    let mut dom =
        VirtualDom::new_with_props(replace_portal_app, AppProps { target: target_slot.clone() });
    let target = dom.runtime().create_render_target();
    target_slot.set(target);

    let edits = rebuild_to_targeted_vec(&mut dom);
    assert!(maps_id(edits.get(&target).unwrap(), ElementId::from_raw(1)));

    SHOW_PORTAL.store(0, Ordering::SeqCst);
    dom.mark_dirty(ScopeId::APP);

    let edits = render_immediate_to_targeted_vec(&mut dom);

    assert!(removes_id(
        edits.get(&target).unwrap(),
        ElementId::from_raw(1)
    ));
}

#[test]
fn detached_targets_drop_writes_but_mount_effects() {
    EFFECTS.store(0, Ordering::SeqCst);
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    let target_slot = TargetSlot::new();
    let mut dom =
        VirtualDom::new_with_props(dropped_target_app, AppProps { target: target_slot.clone() });
    let target = dom.runtime().create_render_target();
    target_slot.set(target);

    // The target's writer is attached for the rebuild, then gone for the next
    // pass. Targeted mutations are dropped, but logical mounting still runs.
    let mut writer = BTreeMap::new();
    writer.insert(RenderTargetId::ROOT, Mutations::default());
    writer.insert(target, Mutations::default());
    dom.rebuild(&mut writer);
    let edits = drain_targets(writer);
    let show_button = first_click_listener(edits.get(&RenderTargetId::ROOT).unwrap());

    dom.runtime()
        .handle_event("click", click_event(), show_button);

    let mut writer = BTreeMap::new();
    writer.insert(RenderTargetId::ROOT, Mutations::default());
    dom.render_immediate(&mut writer);
    let edits = drain_targets(writer);

    assert!(!edits.contains_key(&target));
    dom.process_events();
    assert!(EFFECTS.load(Ordering::SeqCst) > 0);
}

#[test]
fn can_open_new_portal_after_closing_previous_keyed_portal() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    let first_slot = TargetSlot::new();
    let second_slot = TargetSlot::new();
    let mut dom = VirtualDom::new_with_props(
        reopen_after_close_app,
        ReopenProps { first: first_slot.clone(), second: second_slot.clone() },
    );
    let first = dom.runtime().create_render_target();
    let second = dom.runtime().create_render_target();
    first_slot.set(first);
    second_slot.set(second);

    let edits = rebuild_to_targeted_vec(&mut dom);
    let open_button = first_click_listener(edits.get(&RenderTargetId::ROOT).unwrap());

    dom.runtime()
        .handle_event("click", click_event(), open_button);

    let edits = render_immediate_to_targeted_vec(&mut dom);
    assert!(has_click_listener(
        edits.get(&first).unwrap(),
        ElementId::from_raw(1)
    ));

    dom.runtime()
        .handle_event_for_target(first, "click", click_event(), ElementId::from_raw(1));

    let edits = render_immediate_to_targeted_vec(&mut dom);
    assert!(removes_id(
        edits.get(&first).unwrap(),
        ElementId::from_raw(1)
    ));

    dom.runtime()
        .handle_event("click", click_event(), open_button);

    let edits = render_immediate_to_targeted_vec(&mut dom);

    assert!(has_click_listener(
        edits.get(&second).unwrap(),
        ElementId::from_raw(1)
    ));
}

#[test]
fn can_open_new_dynamic_target_after_closing_previous_keyed_portal() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    let mut dom = VirtualDom::new(dynamic_reopen_after_close_app);

    let edits = rebuild_to_targeted_vec(&mut dom);
    let open_button = first_click_listener(edits.get(&RenderTargetId::ROOT).unwrap());

    dom.runtime()
        .handle_event("click", click_event(), open_button);

    let edits = render_immediate_to_targeted_vec(&mut dom);
    let first_target = *edits
        .keys()
        .find(|id| **id != RenderTargetId::ROOT)
        .expect("first dynamic portal target should render");
    assert!(has_click_listener(
        edits.get(&first_target).unwrap(),
        ElementId::from_raw(1)
    ));

    dom.runtime().handle_event_for_target(
        first_target,
        "click",
        click_event(),
        ElementId::from_raw(1),
    );

    let edits = render_immediate_to_targeted_vec(&mut dom);
    assert!(removes_id(
        edits.get(&first_target).unwrap(),
        ElementId::from_raw(1)
    ));

    dom.runtime()
        .handle_event("click", click_event(), open_button);

    let edits = render_immediate_to_targeted_vec(&mut dom);
    let second_target = *edits
        .keys()
        .find(|id| **id != RenderTargetId::ROOT && **id != first_target)
        .expect("second dynamic portal target should render");

    assert!(has_click_listener(
        edits.get(&second_target).unwrap(),
        ElementId::from_raw(1)
    ));
}

static PORTAL_STATE_INITS: AtomicUsize = AtomicUsize::new(0);
static PORTAL_LABEL: GlobalSignal<usize> = Signal::global(|| 0);

fn suspended_portal_app(props: AppProps) -> Element {
    rsx! {
        SuspenseBoundary {
            fallback: |_| rsx! { "fallback" },
            SuspendingChild {}
            Portal {
                target: props.target.get(),
                PortalStateChild {}
            }
        }
    }
}

#[component]
fn SuspendingChild() -> Element {
    let mut resolved = use_signal(|| false);
    let task = use_hook(|| {
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            resolved.set(true);
        })
    });
    if !resolved() {
        suspend(task)?;
    }
    rsx! {
        div { "resolved" }
    }
}

#[component]
fn PortalStateChild() -> Element {
    use_hook(|| {
        PORTAL_STATE_INITS.fetch_add(1, Ordering::SeqCst);
    });
    rsx! { "{PORTAL_LABEL}" }
}

/// A portal hidden under a suspended boundary keeps its component state and
/// emits its up-to-date content into the target when the boundary resolves.
///
/// Resolving re-creates the boundary's children from dom-state, which reaches
/// the live portal scope through `PortalDriver::create`: the driver must
/// re-create from the mounted output instead of the props' children handle,
/// whose mount cell never observes the mounts of the first render. Re-creating
/// from the unmounted handle allocates fresh child scopes, resetting portal
/// subtree state.
#[test]
fn portal_under_suspense_keeps_state_and_updates_target_on_resolve() {
    PORTAL_STATE_INITS.store(0, Ordering::SeqCst);

    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let target_slot = TargetSlot::new();
            let mut dom = VirtualDom::new_with_props(
                suspended_portal_app,
                AppProps { target: target_slot.clone() },
            );
            let target = dom.runtime().create_render_target();
            target_slot.set(target);

            let edits = rebuild_to_targeted_vec(&mut dom);
            // The boundary suspends: the fallback renders on the main target
            // and the background-created portal content writes nothing.
            assert!(edits.get(&RenderTargetId::ROOT).is_some());
            assert!(edits.get(&target).is_none());
            assert_eq!(PORTAL_STATE_INITS.load(Ordering::SeqCst), 1);

            // Update the hidden portal child while the boundary is suspended.
            dom.in_scope(ScopeId::APP, || *PORTAL_LABEL.write() = 1);

            // Drive work/render passes until the suspended task finishes and
            // the boundary resolves: the resolve pass swaps the fallback for
            // the children on the main target and emits the portal's current
            // content into its target.
            let mut resolve_edits = None;
            for _ in 0..10 {
                dom.wait_for_work().await;
                let edits = render_immediate_to_targeted_vec(&mut dom);
                if edits.contains_key(&target) {
                    resolve_edits = Some(edits);
                    break;
                }
            }
            let edits =
                resolve_edits.expect("the boundary should resolve and write the portal target");
            assert!(edits.get(&RenderTargetId::ROOT).is_some());
            let portal_edits = edits.get(&target).unwrap();
            assert!(portal_edits.edits.iter().any(|mutation| matches!(
                mutation,
                Mutation::CreateText { value } if value.as_str() == "1"
            )));
            assert!(appends_to_root(portal_edits), "{portal_edits:#?}");
            // The live portal subtree was reused, not re-created from scratch.
            assert_eq!(PORTAL_STATE_INITS.load(Ordering::SeqCst), 1);
        });
}
