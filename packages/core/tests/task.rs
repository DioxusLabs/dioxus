//! Verify that tasks get polled by the virtualdom properly, and that we escape wait_for_work safely

use dioxus::prelude::*;
use std::{sync::atomic::AtomicUsize, time::Duration};

static POLL_COUNT: AtomicUsize = AtomicUsize::new(0);

#[cfg(not(miri))]
#[tokio::test]
async fn it_works() {
    let mut dom = VirtualDom::new(app);

    let _ = dom.rebuild();

    tokio::select! {
        _ = dom.wait_for_work() => {}
        _ = tokio::time::sleep(Duration::from_millis(500)) => {}
    };

    // By the time the tasks are finished, we should've accumulated ticks from two tasks
    // Be warned that by setting the delay to too short, tokio might not schedule in the tasks
    assert_eq!(
        POLL_COUNT.fetch_add(0, std::sync::atomic::Ordering::Relaxed),
        135
    );
}

fn app(cx: Scope) -> Element {
    cx.use_hook(|| {
        cx.spawn(async {
            for x in 0..10 {
                tokio::time::sleep(Duration::from_micros(50)).await;
                POLL_COUNT.fetch_add(x, std::sync::atomic::Ordering::Relaxed);
            }
        });

        cx.spawn(async {
            for x in 0..10 {
                tokio::time::sleep(Duration::from_micros(25)).await;
                POLL_COUNT.fetch_add(x * 2, std::sync::atomic::Ordering::Relaxed);
            }
        });
    });

    cx.render(rsx!(()))
}
