//! Verify that tasks get polled by the virtualdom properly, and that we escape wait_for_work safely

use dioxus::prelude::*;
use std::time::Duration;

static mut POLL_COUNT: usize = 0;

#[tokio::test]
async fn it_works() {
    let mut dom = VirtualDom::new(app);

    let _ = dom.rebuild();

    tokio::select! {
        _ = dom.wait_for_work() => {}
        _ = tokio::time::sleep(Duration::from_millis(10)) => {}
    };

    // By the time the tasks are finished, we should've accumulated ticks from two tasks
    // Be warned that by setting the delay to too short, tokio might not schedule in the tasks
    assert_eq!(unsafe { POLL_COUNT }, 135);
}

fn app(cx: Scope) -> Element {
    cx.use_hook(|| {
        cx.spawn(async {
            for x in 0..10 {
                tokio::time::sleep(Duration::from_micros(50)).await;
                unsafe { POLL_COUNT += x }
            }
        });

        cx.spawn(async {
            for x in 0..10 {
                tokio::time::sleep(Duration::from_micros(25)).await;
                unsafe { POLL_COUNT += x * 2 }
            }
        });
    });

    cx.render(rsx!(()))
}
