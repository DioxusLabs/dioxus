use dioxus::prelude::*;
use dioxus_core::{ElementId, Mutation, Mutations, Portal, RenderTargetId, Runtime, VirtualDom};
use dioxus_renderer_oracle::MultiTargetWriter;
use std::{
    any::Any,
    collections::BTreeMap,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

/// Collect one rebuild into per-target mutation lists. Targets created during
/// the diff lazily get a fresh `Mutations` collector; untouched targets are
/// dropped so tests can assert absence with `get(..) == None`.
fn rebuild_to_targeted_vec(dom: &mut VirtualDom) -> BTreeMap<RenderTargetId, Mutations> {
    let mut writer = MultiTargetWriter::<Mutations>::with_factory(Mutations::default);
    dom.rebuild(&mut writer);
    drain_targets(writer)
}

/// [`rebuild_to_targeted_vec`], but for one `render_immediate` pass.
fn render_immediate_to_targeted_vec(dom: &mut VirtualDom) -> BTreeMap<RenderTargetId, Mutations> {
    let mut writer = MultiTargetWriter::<Mutations>::with_factory(Mutations::default);
    dom.render_immediate(&mut writer);
    drain_targets(writer)
}

fn drain_targets(writer: MultiTargetWriter<Mutations>) -> BTreeMap<RenderTargetId, Mutations> {
    writer
        .into_targets()
        .into_iter()
        .filter(|(_, m)| !m.edits.is_empty())
        .collect()
}

static ROOT_CLICKS: AtomicUsize = AtomicUsize::new(0);
static PORTAL_CLICKS: AtomicUsize = AtomicUsize::new(0);
static RETARGET_CLICKS: AtomicUsize = AtomicUsize::new(0);
static EFFECTS: AtomicUsize = AtomicUsize::new(0);
static SHOW_PORTAL: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct SharedContext(&'static str);

#[derive(Clone, PartialEq, Props)]
struct AppProps {
    target: RenderTargetId,
}

#[derive(Clone, PartialEq, Props)]
struct RetargetProps {
    first: RenderTargetId,
    second: RenderTargetId,
}

#[derive(Clone, PartialEq, Props)]
struct ReopenProps {
    first: RenderTargetId,
    second: RenderTargetId,
}

fn noop_app(props: AppProps) -> Element {
    rsx! {
        Portal {
            target: props.target,
            EffectChild {}
        }
    }
}

fn context_app(props: AppProps) -> Element {
    use_hook(|| provide_context(SharedContext("shared")));

    rsx! {
        Portal {
            target: props.target,
            ContextChild {}
        }
    }
}

fn retarget_app(props: RetargetProps) -> Element {
    let mut target = use_signal(|| props.first);
    let second = props.second;

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
                target: props.target,
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
                target: if id == 0 { props.first } else { props.second },
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
            PortalWrapper { target: props.target }
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

fn click_event() -> Event<dyn Any> {
    Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
        true,
    )
}

fn has_click_listener(mutations: &Mutations, id: ElementId) -> bool {
    mutations.edits.iter().any(|mutation| {
        matches!(
            mutation,
            Mutation::NewEventListener { name, id: listener_id }
                if *name == "click" && *listener_id == id
        )
    })
}

fn first_click_listener(mutations: &Mutations) -> ElementId {
    mutations
        .edits
        .iter()
        .find_map(|mutation| match mutation {
            Mutation::NewEventListener { name, id } if *name == "click" => Some(*id),
            _ => None,
        })
        .unwrap()
}

fn app(props: AppProps) -> Element {
    rsx! {
        div {
            onclick: move |_| {
                ROOT_CLICKS.fetch_add(1, Ordering::SeqCst);
            },
            Portal {
                target: props.target,
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

    let mut dom = VirtualDom::new_with_props(app, AppProps { target: RenderTargetId(1) });
    let target = dom.runtime().create_render_target();
    assert_eq!(target, RenderTargetId(1));

    let edits = rebuild_to_targeted_vec(&mut dom);

    let root_edits = edits.get(&RenderTargetId::ROOT).unwrap();
    let portal_edits = edits.get(&target).unwrap();

    assert!(has_click_listener(root_edits, ElementId(1)));
    assert!(has_click_listener(portal_edits, ElementId(1)));

    dom.runtime()
        .handle_event("click", click_event(), ElementId(1));
    assert_eq!(ROOT_CLICKS.load(Ordering::SeqCst), 1);
    assert_eq!(PORTAL_CLICKS.load(Ordering::SeqCst), 0);

    dom.runtime()
        .handle_event_for_target(target, "click", click_event(), ElementId(1));
    assert_eq!(PORTAL_CLICKS.load(Ordering::SeqCst), 1);
    assert_eq!(ROOT_CLICKS.load(Ordering::SeqCst), 2);
}

#[test]
fn writerless_targets_do_not_mount_effects() {
    EFFECTS.store(0, Ordering::SeqCst);

    let mut dom = VirtualDom::new_with_props(noop_app, AppProps { target: RenderTargetId(1) });
    let target = dom.runtime().create_render_target();
    assert_eq!(target, RenderTargetId(1));

    // A plain writer serves only the root target, so the portal's target has
    // no writer: its content keeps logical state alive without mounting.
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.process_events();
    dom.render_immediate(&mut dioxus_core::NoOpMutations);

    assert_eq!(EFFECTS.load(Ordering::SeqCst), 0);
}

#[test]
fn portal_children_keep_scope_context() {
    let mut dom = VirtualDom::new_with_props(context_app, AppProps { target: RenderTargetId(1) });
    let target = dom.runtime().create_render_target();
    assert_eq!(target, RenderTargetId(1));

    let edits = rebuild_to_targeted_vec(&mut dom);
    assert!(edits.contains_key(&target));
}

#[test]
fn retargeting_portal_drops_and_recreates_target_subtree() {
    RETARGET_CLICKS.store(0, Ordering::SeqCst);
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    let mut dom = VirtualDom::new_with_props(
        retarget_app,
        RetargetProps { first: RenderTargetId(1), second: RenderTargetId(2) },
    );
    let first = dom.runtime().create_render_target();
    let second = dom.runtime().create_render_target();
    assert_eq!(first, RenderTargetId(1));
    assert_eq!(second, RenderTargetId(2));

    let edits = rebuild_to_targeted_vec(&mut dom);
    assert!(has_click_listener(edits.get(&first).unwrap(), ElementId(1)));

    dom.runtime()
        .handle_event("click", click_event(), ElementId(2));

    let edits = render_immediate_to_targeted_vec(&mut dom);

    assert!(
        edits
            .get(&first)
            .unwrap()
            .edits
            .iter()
            .any(|mutation| matches!(mutation, Mutation::Remove { id } if *id == ElementId(1)))
    );
    assert!(has_click_listener(
        edits.get(&second).unwrap(),
        ElementId(1)
    ));

    dom.runtime()
        .handle_event_for_target(second, "click", click_event(), ElementId(1));
    assert_eq!(RETARGET_CLICKS.load(Ordering::SeqCst), 1);
}

#[test]
fn replacing_portal_with_local_node_removes_old_target_subtree() {
    SHOW_PORTAL.store(1, Ordering::SeqCst);

    let mut dom =
        VirtualDom::new_with_props(replace_portal_app, AppProps { target: RenderTargetId(1) });
    let target = dom.runtime().create_render_target();
    assert_eq!(target, RenderTargetId(1));

    let edits = rebuild_to_targeted_vec(&mut dom);
    assert!(edits.get(&target).unwrap().edits.iter().any(
        |mutation| matches!(mutation, Mutation::LoadTemplate { id, .. } if *id == ElementId(1))
    ));

    SHOW_PORTAL.store(0, Ordering::SeqCst);
    dom.mark_dirty(ScopeId::APP);

    let edits = render_immediate_to_targeted_vec(&mut dom);

    assert!(
        edits
            .get(&target)
            .unwrap()
            .edits
            .iter()
            .any(|mutation| matches!(mutation, Mutation::Remove { id } if *id == ElementId(1)))
    );
}

#[test]
fn detached_targets_do_not_write_or_mount_effects() {
    EFFECTS.store(0, Ordering::SeqCst);
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    let mut dom =
        VirtualDom::new_with_props(dropped_target_app, AppProps { target: RenderTargetId(1) });
    let target = dom.runtime().create_render_target();
    assert_eq!(target, RenderTargetId(1));

    // The target's writer is attached for the rebuild, then gone for the
    // next pass — the host stopped serving it, like a closed desktop window.
    let mut writer = MultiTargetWriter::<Mutations>::new();
    writer.insert(RenderTargetId::ROOT, Mutations::default());
    writer.insert(target, Mutations::default());
    dom.rebuild(&mut writer);
    let edits = drain_targets(writer);
    let show_button = first_click_listener(edits.get(&RenderTargetId::ROOT).unwrap());

    dom.runtime()
        .handle_event("click", click_event(), show_button);

    let mut writer = MultiTargetWriter::<Mutations>::new();
    writer.insert(RenderTargetId::ROOT, Mutations::default());
    dom.render_immediate(&mut writer);
    let edits = drain_targets(writer);

    assert!(!edits.contains_key(&target));
    dom.process_events();
    assert_eq!(EFFECTS.load(Ordering::SeqCst), 0);
}

#[test]
fn can_open_new_portal_after_closing_previous_keyed_portal() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    let mut dom = VirtualDom::new_with_props(
        reopen_after_close_app,
        ReopenProps { first: RenderTargetId(1), second: RenderTargetId(2) },
    );
    let first = dom.runtime().create_render_target();
    let second = dom.runtime().create_render_target();
    assert_eq!(first, RenderTargetId(1));
    assert_eq!(second, RenderTargetId(2));

    let edits = rebuild_to_targeted_vec(&mut dom);
    let open_button = first_click_listener(edits.get(&RenderTargetId::ROOT).unwrap());

    dom.runtime()
        .handle_event("click", click_event(), open_button);

    let edits = render_immediate_to_targeted_vec(&mut dom);
    assert!(has_click_listener(edits.get(&first).unwrap(), ElementId(1)));

    dom.runtime()
        .handle_event_for_target(first, "click", click_event(), ElementId(1));

    let edits = render_immediate_to_targeted_vec(&mut dom);
    assert!(
        edits
            .get(&first)
            .unwrap()
            .edits
            .iter()
            .any(|mutation| matches!(mutation, Mutation::Remove { id } if *id == ElementId(1)))
    );

    dom.runtime()
        .handle_event("click", click_event(), open_button);

    let edits = render_immediate_to_targeted_vec(&mut dom);

    assert!(has_click_listener(
        edits.get(&second).unwrap(),
        ElementId(1)
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
    assert!(has_click_listener(
        edits.get(&RenderTargetId(1)).unwrap(),
        ElementId(1)
    ));

    dom.runtime()
        .handle_event_for_target(RenderTargetId(1), "click", click_event(), ElementId(1));

    let edits = render_immediate_to_targeted_vec(&mut dom);
    assert!(
        edits
            .get(&RenderTargetId(1))
            .unwrap()
            .edits
            .iter()
            .any(|mutation| matches!(mutation, Mutation::Remove { id } if *id == ElementId(1)))
    );

    dom.runtime()
        .handle_event("click", click_event(), open_button);

    let edits = render_immediate_to_targeted_vec(&mut dom);

    assert!(has_click_listener(
        edits.get(&RenderTargetId(2)).unwrap(),
        ElementId(1)
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
                target: props.target,
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
            let mut dom = VirtualDom::new_with_props(
                suspended_portal_app,
                AppProps { target: RenderTargetId(1) },
            );
            let target = dom.runtime().create_render_target();
            assert_eq!(target, RenderTargetId(1));

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
                Mutation::CreateTextNode { value, .. } if value.as_str() == "1"
            )));
            assert!(portal_edits.edits.iter().any(|mutation| matches!(
                mutation,
                Mutation::AppendChildren { id: ElementId(0), .. }
            )));
            // The live portal subtree was reused, not re-created from scratch.
            assert_eq!(PORTAL_STATE_INITS.load(Ordering::SeqCst), 1);
        });
}
