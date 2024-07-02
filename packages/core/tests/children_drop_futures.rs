//! Verify that when children are dropped, they drop their futures before they are polled

use std::{sync::atomic::AtomicUsize, time::Duration};

use dioxus::prelude::*;

#[tokio::test]
async fn child_futures_drop_first() {
    static POLL_COUNT: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        if generation() == 0 {
            rsx! {Child {}}
        } else {
            rsx! {}
        }
    }

    #[component]
    fn Child() -> Element {
        // Spawn a task that will increment POLL_COUNT every 10 milliseconds
        // This should be dropped after the second time the parent is run
        use_hook(|| {
            spawn(async {
                POLL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            });
        });

        rsx! {}
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    // Here the parent and task could resolve at the same time, but because the task is in the child, dioxus should run the parent first because the child might be dropped
    dom.mark_dirty(ScopeId::APP);

    tokio::select! {
        _ = dom.wait_for_work() => {}
        _ = tokio::time::sleep(Duration::from_millis(500)) => panic!("timed out")
    };

    dom.render_immediate(&mut dioxus_core::NoOpMutations);

    // By the time the tasks are finished, we should've accumulated ticks from two tasks
    // Be warned that by setting the delay to too short, tokio might not schedule in the tasks
    assert_eq!(
        POLL_COUNT.fetch_add(0, std::sync::atomic::Ordering::Relaxed),
        0
    );
}
