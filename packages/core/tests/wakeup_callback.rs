//! Verify that the wakeup callback fires when async tasks complete.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dioxus::prelude::*;

#[tokio::test]
async fn wakeup_callback_fires_on_task_completion() {

    fn app() -> Element {
        use_hook(|| {
            spawn(async {
                tokio::time::sleep(Duration::from_millis(10)).await;
            });
        });
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);

    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = count.clone();
    dom.set_wakeup_callback(move || {
        count_clone.fetch_add(1, Ordering::Relaxed);
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    tokio::select! {
        _ = dom.wait_for_work() => {}
        _ = tokio::time::sleep(Duration::from_millis(200)) => {}
    };

    // The callback should have fired at least once when the spawned task woke up.
    assert!(count.load(Ordering::Relaxed) > 0, "wakeup callback should have been called");
}

#[tokio::test]
async fn wakeup_callback_not_called_without_async_work() {
    fn app() -> Element {
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);

    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = count.clone();
    dom.set_wakeup_callback(move || {
        count_clone.fetch_add(1, Ordering::Relaxed);
    });

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    // No async work spawned — callback should not fire.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(count.load(Ordering::Relaxed), 0, "wakeup callback should not fire without async work");
}
