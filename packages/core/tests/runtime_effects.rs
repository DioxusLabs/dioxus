use dioxus::prelude::*;
use dioxus_core::{
    AttributeValue, ElementId, Runtime, WriteMutations, generation, needs_update, queue_effect,
};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[test]
fn effect_queued_during_render_immediate_has_runtime() {
    fn app() -> Element {
        if generation() == 0 {
            needs_update();
        } else {
            queue_effect(|| {
                assert!(Runtime::try_current().is_some());
            });
        }

        rsx!({})
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
}

static SHOW_EFFECT_CHILD: AtomicBool = AtomicBool::new(false);
static RENDERER_FLUSHED: AtomicBool = AtomicBool::new(false);
static EFFECT_RUNS: AtomicUsize = AtomicUsize::new(0);
static EFFECT_RAN_BEFORE_FLUSH: AtomicUsize = AtomicUsize::new(0);

#[test]
fn effects_run_after_renderer_flush_boundary() {
    SHOW_EFFECT_CHILD.store(false, Ordering::Relaxed);
    RENDERER_FLUSHED.store(false, Ordering::Relaxed);
    EFFECT_RUNS.store(0, Ordering::Relaxed);
    EFFECT_RAN_BEFORE_FLUSH.store(0, Ordering::Relaxed);

    let mut dom = VirtualDom::new(effect_flush_boundary_app);
    let mut renderer = BufferedRenderer::default();

    dom.rebuild(&mut renderer);
    renderer.flush();

    SHOW_EFFECT_CHILD.store(true, Ordering::Relaxed);
    RENDERER_FLUSHED.store(false, Ordering::Relaxed);
    dom.mark_dirty(ScopeId::APP);

    dom.render_immediate(&mut renderer);

    assert_eq!(
        EFFECT_RUNS.load(Ordering::Relaxed),
        0,
        "effect ran before the renderer flush boundary"
    );
    assert_eq!(
        EFFECT_RAN_BEFORE_FLUSH.load(Ordering::Relaxed),
        0,
        "effects must not run before the renderer has flushed queued mutations"
    );

    renderer.flush();
    dom.process_events();

    assert_eq!(
        EFFECT_RUNS.load(Ordering::Relaxed),
        1,
        "effect should run after the renderer flush boundary"
    );
    assert_eq!(
        EFFECT_RAN_BEFORE_FLUSH.load(Ordering::Relaxed),
        0,
        "effect observed unflushed renderer state"
    );
}

fn effect_flush_boundary_app() -> Element {
    if SHOW_EFFECT_CHILD.load(Ordering::Relaxed) {
        rsx! {
            EffectChild {}
        }
    } else {
        rsx! {
            div { "before" }
        }
    }
}

#[component]
fn EffectChild() -> Element {
    use_effect(|| {
        EFFECT_RUNS.fetch_add(1, Ordering::Relaxed);
        if !RENDERER_FLUSHED.load(Ordering::Relaxed) {
            EFFECT_RAN_BEFORE_FLUSH.fetch_add(1, Ordering::Relaxed);
        }
    });

    rsx! {
        div { id: "effect-child", "after" }
    }
}

#[derive(Default)]
struct BufferedRenderer;

impl BufferedRenderer {
    fn flush(&mut self) {
        RENDERER_FLUSHED.store(true, Ordering::Relaxed);
    }
}

impl WriteMutations for BufferedRenderer {
    fn push_id(&mut self, _id: ElementId) {}

    fn pop_id(&mut self, _id: ElementId) {}

    fn child(&mut self, _index: usize) {}

    fn pop(&mut self) {}

    fn create_element(&mut self, _tag: &str, _ns: Option<&str>) {}

    fn create_text(&mut self, _value: &str) {}

    fn clone(&mut self) {}

    fn append_children(&mut self, _m: usize) {}

    fn replace_with(&mut self, _m: usize) {}

    fn insert_after(&mut self, _m: usize) {}

    fn insert_before(&mut self, _m: usize) {}

    fn set_attribute(&mut self, _name: &str, _ns: Option<&str>, _value: &AttributeValue) {}

    fn set_text(&mut self, _value: &str) {}

    fn add_event_listener(&mut self, _name: &str) {}

    fn remove_event_listener(&mut self, _name: &str) {}

    fn remove(&mut self) {}
}
