//! Verify that tasks get polled by the virtualdom properly, and that we escape wait_for_work safely

use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    task::Poll,
    time::Duration,
};

use dioxus::prelude::*;
use dioxus_core::{NoOpMutations, generation, needs_update, spawn_forever};

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

#[tokio::test]
async fn spawn_forever_persists() {
    use std::sync::atomic::Ordering;
    static POLL_COUNT: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        if generation() > 0 {
            rsx!(div {})
        } else {
            needs_update();
            rsx!(Child {})
        }
    }

    #[component]
    fn Child() -> Element {
        spawn_forever(async move {
            for _ in 0..10 {
                POLL_COUNT.fetch_add(1, Ordering::Relaxed);
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });

        rsx!(div {})
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);

    tokio::select! {
        _ = dom.wait_for_work() => {}
        // We intentionally wait a bit longer than 50ms*10 to make sure the test has time to finish
        // Without the extra time, the test can fail on windows
        _ = tokio::time::sleep(Duration::from_millis(1000)) => {}
    };

    // By the time the tasks are finished, we should've accumulated ticks from two tasks
    // Be warned that by setting the delay to too short, tokio might not schedule in the tasks
    assert_eq!(POLL_COUNT.load(Ordering::Relaxed), 10);
}

/// Prove that yield_now doesn't cause a deadlock
#[tokio::test]
async fn yield_now_works() {
    thread_local! {
        static SEQUENCE: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
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

/// Run `drive` on its own thread and fail the test if it has not returned within `deadline`.
///
/// A scheduler that re-polls a task forever never returns control, so the check has to live
/// outside the thread that drives the VirtualDom.
fn returns_within<T: Send + 'static>(
    deadline: Duration,
    drive: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    let driver = std::thread::spawn(move || {
        let _ = tx.send(drive());
    });
    match rx.recv_timeout(deadline) {
        Ok(value) => value,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            panic!("the VirtualDom did not hand control back within {deadline:?}")
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            std::panic::resume_unwind(driver.join().unwrap_err())
        }
    }
}

/// A task that can always make progress: every poll wakes its own waker before returning
/// `Pending`, so it is queued again while it is still being polled.
async fn wake_self_forever(polls: &'static AtomicUsize) {
    std::future::poll_fn(|cx| {
        polls.fetch_add(1, Ordering::Relaxed);
        cx.waker().wake_by_ref();
        Poll::<()>::Pending
    })
    .await
}

/// Wake our own waker and return `Pending` once, then complete.
async fn yield_once() {
    let mut yielded = false;
    std::future::poll_fn(|cx| {
        if yielded {
            return Poll::Ready(());
        }
        yielded = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    })
    .await
}

/// `wait_for_work` must give the executor control back between polls of a task that keeps
/// waking itself, instead of polling it again forever.
#[test]
fn wait_for_work_yields_between_polls_of_a_self_waking_task() {
    static POLLS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        use_hook(|| spawn(wake_self_forever(&POLLS)));
        rsx!({})
    }

    let [earlier, later] = returns_within(Duration::from_secs(10), || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let mut dom = VirtualDom::new(app);
            dom.rebuild(&mut NoOpMutations);
            let work = dom.wait_for_work();
            tokio::pin!(work);
            let mut polls = [0; 2];
            for sample in &mut polls {
                tokio::select! {
                    _ = &mut work => panic!("no scope is ever dirtied, so wait_for_work should not return"),
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {}
                }
                *sample = POLLS.load(Ordering::Relaxed);
            }
            polls
        })
    });

    // The task kept being polled while wait_for_work was pending, rather than being parked.
    assert!(later > earlier, "polls: {earlier} then {later}");
}

/// The synchronous drains poll a task a bounded number of times per call, so they return even when
/// a task always wakes itself, and the task still runs again on the next call.
#[test]
fn synchronous_drains_return_with_a_self_waking_task() {
    static POLLS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        use_hook(|| spawn(wake_self_forever(&POLLS)));
        rsx!({})
    }

    let polls_after_each_call = returns_within(Duration::from_secs(10), || {
        let mut dom = VirtualDom::new(app);
        dom.rebuild(&mut NoOpMutations);
        let mut polls = Vec::new();
        for _ in 0..2 {
            dom.process_events();
            polls.push(POLLS.load(Ordering::Relaxed));
            dom.render_immediate(&mut NoOpMutations);
            polls.push(POLLS.load(Ordering::Relaxed));
        }
        polls
    });

    // Every call polled the task again: it was queued for the next call, not dropped.
    assert!(
        polls_after_each_call
            .windows(2)
            .all(|pair| pair[1] > pair[0]),
        "polls after each call: {polls_after_each_call:?}"
    );
}

/// Effects that run while the scheduler drains after polling tasks can wake tasks too; the drain
/// must return even when an effect starts a task that always wakes itself.
#[test]
fn effect_draining_returns_with_a_self_waking_task() {
    static POLLS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        use_effect(|| {
            spawn(wake_self_forever(&POLLS));
        });
        rsx!({})
    }

    returns_within(Duration::from_secs(10), || {
        let mut dom = VirtualDom::new(app);
        dom.rebuild(&mut NoOpMutations);
        dom.process_events();
    });

    assert!(POLLS.load(Ordering::Relaxed) > 0);
}

/// A task that always wakes itself in a parent scope is polled before tasks in child scopes.
/// It must not keep the child's task from ever being polled.
#[test]
fn self_waking_task_does_not_starve_a_child_task() {
    static PARENT_POLLS: AtomicUsize = AtomicUsize::new(0);
    static CHILD_DONE: AtomicBool = AtomicBool::new(false);

    fn app() -> Element {
        use_hook(|| spawn(wake_self_forever(&PARENT_POLLS)));
        rsx! { Child {} }
    }

    #[component]
    fn Child() -> Element {
        use_hook(|| {
            spawn(async {
                for _ in 0..3 {
                    yield_once().await;
                }
                CHILD_DONE.store(true, Ordering::Relaxed);
            })
        });
        rsx!({})
    }

    returns_within(Duration::from_secs(10), || {
        let mut dom = VirtualDom::new(app);
        dom.rebuild(&mut NoOpMutations);
        for _ in 0..8 {
            dom.process_events();
        }
    });

    assert!(CHILD_DONE.load(Ordering::Relaxed));
}

/// Tokio makes a task that has used up its cooperative budget return `Pending` from its next
/// tokio resource. Off the runtime's worker threads it also wakes that task immediately, and it
/// only refills the budget once the executor gets control back. A task that simply loops over a
/// tokio resource (an interval, a channel) must therefore keep running rather than freeze the
/// VirtualDom.
#[test]
fn exhausted_tokio_coop_budget_does_not_freeze_the_virtual_dom() {
    static UNITS: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        use_hook(|| {
            spawn(async {
                loop {
                    tokio::task::coop::consume_budget().await;
                    UNITS.fetch_add(1, Ordering::Relaxed);
                }
            })
        });
        rsx!({})
    }

    returns_within(Duration::from_secs(10), || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_time()
            .build()
            .unwrap();
        // `block_on` from a thread that is not one of the runtime's workers.
        rt.block_on(async {
            let mut dom = VirtualDom::new(app);
            dom.rebuild(&mut NoOpMutations);
            tokio::select! {
                _ = dom.wait_for_work() => panic!("no scope is ever dirtied, so wait_for_work should not return"),
                _ = tokio::time::sleep(Duration::from_millis(100)) => {}
            }
        });
    });

    // The default budget is 128 units per poll. Getting past it means the budget was refilled,
    // which only happens when the executor got control back.
    assert!(UNITS.load(Ordering::Relaxed) > 128);
}
