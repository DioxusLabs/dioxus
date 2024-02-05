//! Verify that tasks get polled by the virtualdom properly, and that we escape wait_for_work safely

use std::{sync::atomic::AtomicUsize, time::Duration};

use dioxus::prelude::*;

async fn run_vdom(app: fn() -> Element) {
    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    tokio::select! {
        _ = dom.wait_for_work() => {}
        _ = tokio::time::sleep(Duration::from_millis(500)) => {}
    };
}

#[tokio::test]
async fn running_async() {
    static POLL_COUNT: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        use_hook(|| {
            spawn(async {
                for x in 0..10 {
                    tokio::time::sleep(Duration::from_micros(50)).await;
                    POLL_COUNT.fetch_add(x, std::sync::atomic::Ordering::Relaxed);
                }
            });

            spawn(async {
                for x in 0..10 {
                    tokio::time::sleep(Duration::from_micros(25)).await;
                    POLL_COUNT.fetch_add(x * 2, std::sync::atomic::Ordering::Relaxed);
                }
            });
        });

        rsx!({})
    }

    run_vdom(app).await;

    // By the time the tasks are finished, we should've accumulated ticks from two tasks
    // Be warned that by setting the delay to too short, tokio might not schedule in the tasks
    assert_eq!(
        POLL_COUNT.fetch_add(0, std::sync::atomic::Ordering::Relaxed),
        135
    );
}

/// Prove that yield_now doesn't cause a deadlock
#[tokio::test]
async fn yield_now_works() {
    thread_local! {
        static SEQUENCE: std::cell::RefCell<Vec<usize>> = std::cell::RefCell::new(Vec::new());
    }

    fn app() -> Element {
        // these two tasks should yield to eachother
        use_hook(|| {
            spawn(async move {
                for _ in 0..10 {
                    tokio::task::yield_now().await;
                    SEQUENCE.with(|s| s.borrow_mut().push(1));
                }
            })
        });

        use_hook(|| {
            spawn(async move {
                for _ in 0..10 {
                    tokio::task::yield_now().await;
                    SEQUENCE.with(|s| s.borrow_mut().push(2));
                }
            })
        });

        rsx!({})
    }

    run_vdom(app).await;

    SEQUENCE.with(|s| assert_eq!(s.borrow().len(), 20));
}

/// Ensure that calling wait_for_flush waits for dioxus to finish its syncrhonous work
#[tokio::test]
async fn flushing() {
    thread_local! {
        static SEQUENCE: std::cell::RefCell<Vec<usize>> = std::cell::RefCell::new(Vec::new());
    }

    fn app() -> Element {
        use_hook(|| {
            spawn(async move {
                for _ in 0..10 {
                    flush_sync().await;
                    SEQUENCE.with(|s| s.borrow_mut().push(1));
                }
            })
        });

        use_hook(|| {
            spawn(async move {
                for _ in 0..10 {
                    flush_sync().await;
                    SEQUENCE.with(|s| s.borrow_mut().push(2));
                }
            })
        });

        rsx!({})
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    let fut = async {
        // Trigger the flush by waiting for work
        for _ in 0..40 {
            tokio::select! {
                _ = dom.wait_for_work() => {}
                _ = tokio::time::sleep(Duration::from_millis(1)) => {}
            };
        }
    };

    tokio::select! {
        _ = fut => {}
        _ = tokio::time::sleep(Duration::from_millis(500)) => {}
    };

    SEQUENCE.with(|s| assert_eq!(s.borrow().len(), 20));
}
