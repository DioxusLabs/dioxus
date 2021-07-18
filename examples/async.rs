//! Example: README.md showcase
//!
//! The example from the README.md

use dioxus::prelude::*;
fn main() {
    dioxus::desktop::launch(App, |c| c).expect("faield to launch");
}

pub static App: FC<()> = |cx| {
    let count = use_state(cx, || 0);
    let mut direction = use_state(cx, || 1);

    let (async_count, dir) = (count.for_async(), *direction);
    let (task, _result) = cx.use_task(move || async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(250).await;
            *async_count.get_mut() += dir;
        }
    });

    cx.render(rsx! {
        div {
            h1 {"count is {count}"}
            button {
                "Stop counting"
                onclick: move |_| task.stop()
            }
            button {
                "Start counting"
                onclick: move |_| task.start()
            }
            button {
                "Switch counting direcion"
                onclick: move |_| {
                    direction *= -1;
                    task.restart();
                }
            }
        }
    })
};
