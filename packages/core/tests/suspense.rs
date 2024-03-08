use dioxus::prelude::*;
use std::future::poll_fn;
use std::task::Poll;

#[test]
fn suspense_resolves() {
    // wait just a moment, not enough time for the boundary to resolve
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            dom.rebuild(&mut dioxus_core::NoOpMutations);
            dom.wait_for_suspense().await;
            let out = dioxus_ssr::render(&dom);

            assert_eq!(out, "<div>Waiting for... child</div>");

            dbg!(out);
        });
}

fn app() -> Element {
    rsx!(
        div {
            "Waiting for... "
            suspended_child {}
        }
    )
}

fn suspended_child() -> Element {
    let mut val = use_signal(|| 0);

    // Tasks that are not suspended should never be polled
    spawn(async move {
        panic!("Non-suspended task was polled");
    });

    // Memos should still work like normal
    let memo = use_memo(move || val * 2);
    assert_eq!(memo, val * 2);

    if val() < 3 {
        let task = spawn(async move {
            // Poll each task 3 times
            let mut count = 0;
            poll_fn(|cx| {
                println!("polling... {}", count);
                if count < 3 {
                    count += 1;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            })
            .await;

            println!("waiting... {}", val);
            val += 1;
        });
        suspend(task)?;
    }

    rsx!("child")
}
