use dioxus::prelude::*;
use dioxus_core::{Runtime, generation, needs_update, queue_effect};

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
