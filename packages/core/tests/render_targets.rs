use dioxus::prelude::*;
use dioxus_core::{
    ElementId, Mutation, Mutations, Portal, RenderTargetId, Runtime, VirtualDom,
};
use std::{
    any::Any,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

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
    let target = dom.create_render_target();
    assert_eq!(target, RenderTargetId(1));

    let edits = dom.rebuild_to_targeted_vec();

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
fn noop_targets_do_not_mount_effects() {
    EFFECTS.store(0, Ordering::SeqCst);

    let mut dom = VirtualDom::new_with_props(noop_app, AppProps { target: RenderTargetId(1) });
    let target = dom.create_noop_render_target();
    assert_eq!(target, RenderTargetId(1));

    dom.rebuild();
    dom.process_events();
    dom.render_immediate();

    assert_eq!(EFFECTS.load(Ordering::SeqCst), 0);
}

#[test]
fn portal_children_keep_scope_context() {
    let mut dom = VirtualDom::new_with_props(context_app, AppProps { target: RenderTargetId(1) });
    let target = dom.create_render_target();
    assert_eq!(target, RenderTargetId(1));

    let edits = dom.rebuild_to_targeted_vec();
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
    let first = dom.create_render_target();
    let second = dom.create_render_target();
    assert_eq!(first, RenderTargetId(1));
    assert_eq!(second, RenderTargetId(2));

    let edits = dom.rebuild_to_targeted_vec();
    assert!(has_click_listener(edits.get(&first).unwrap(), ElementId(1)));

    dom.runtime()
        .handle_event("click", click_event(), ElementId(2));

    let edits = dom.render_immediate_to_targeted_vec();

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
    let target = dom.create_render_target();
    assert_eq!(target, RenderTargetId(1));

    let edits = dom.rebuild_to_targeted_vec();
    assert!(edits.get(&target).unwrap().edits.iter().any(
        |mutation| matches!(mutation, Mutation::LoadTemplate { id, .. } if *id == ElementId(1))
    ));

    SHOW_PORTAL.store(0, Ordering::SeqCst);
    dom.mark_dirty(ScopeId::APP);

    let edits = dom.render_immediate_to_targeted_vec();

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
fn dropped_targets_do_not_write_or_mount_effects() {
    EFFECTS.store(0, Ordering::SeqCst);
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    let mut dom =
        VirtualDom::new_with_props(dropped_target_app, AppProps { target: RenderTargetId(1) });
    let target = dom.create_render_target();
    assert_eq!(target, RenderTargetId(1));

    let edits = dom.rebuild_to_targeted_vec();
    let show_button = first_click_listener(edits.get(&RenderTargetId::ROOT).unwrap());

    dom.runtime().drop_render_target(target);
    dom.runtime()
        .handle_event("click", click_event(), show_button);

    let edits = dom.render_immediate_to_targeted_vec();

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
    let first = dom.create_render_target();
    let second = dom.create_render_target();
    assert_eq!(first, RenderTargetId(1));
    assert_eq!(second, RenderTargetId(2));

    let edits = dom.rebuild_to_targeted_vec();
    let open_button = first_click_listener(edits.get(&RenderTargetId::ROOT).unwrap());

    dom.runtime()
        .handle_event("click", click_event(), open_button);

    let edits = dom.render_immediate_to_targeted_vec();
    assert!(has_click_listener(edits.get(&first).unwrap(), ElementId(1)));

    dom.runtime()
        .handle_event_for_target(first, "click", click_event(), ElementId(1));

    let edits = dom.render_immediate_to_targeted_vec();
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

    let edits = dom.render_immediate_to_targeted_vec();

    assert!(has_click_listener(
        edits.get(&second).unwrap(),
        ElementId(1)
    ));
}

#[test]
fn can_open_new_dynamic_target_after_closing_previous_keyed_portal() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    let mut dom = VirtualDom::new(dynamic_reopen_after_close_app);

    let edits = dom.rebuild_to_targeted_vec();
    let open_button = first_click_listener(edits.get(&RenderTargetId::ROOT).unwrap());

    dom.runtime()
        .handle_event("click", click_event(), open_button);

    let edits = dom.render_immediate_to_targeted_vec();
    assert!(has_click_listener(
        edits.get(&RenderTargetId(1)).unwrap(),
        ElementId(1)
    ));

    dom.runtime()
        .handle_event_for_target(RenderTargetId(1), "click", click_event(), ElementId(1));

    let edits = dom.render_immediate_to_targeted_vec();
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

    let edits = dom.render_immediate_to_targeted_vec();

    assert!(has_click_listener(
        edits.get(&RenderTargetId(2)).unwrap(),
        ElementId(1)
    ));
}
