//! tests to prove that the iterative implementation works

use anyhow::{Context, Result};
use dioxus::{
    arena::SharedResources,
    diff::{CreateMeta, DiffMachine},
    prelude::*,
    scheduler::Mutations,
    DomEdit,
};
mod test_logging;
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;

#[test]
fn test_original_diff() {
    static App: FC<()> = |cx| {
        cx.render(rsx! {
            div {
                div {
                    "Hello, world!"
                }
            }
        })
    };

    let mut dom = VirtualDom::new(App);
    let mutations = dom.rebuild().unwrap();
    dbg!(mutations);
}

#[async_std::test]
async fn test_iterative_create() {
    static App: FC<()> = |cx| {
        cx.render(rsx! {
            div {
                div {
                    "Hello, world!"
                    div {
                        div {
                            Fragment {
                                "hello"
                                "world"
                            }
                        }
                    }
                }
            }
        })
    };

    test_logging::set_up_logging();

    let mut dom = VirtualDom::new(App);
    let mutations = dom.rebuild_async().await.unwrap();
    dbg!(mutations);
}

#[async_std::test]
async fn test_iterative_create_list() {
    static App: FC<()> = |cx| {
        cx.render(rsx! {
            {(0..10).map(|f| rsx!{ div {
                "hello"
            }})}
        })
    };

    test_logging::set_up_logging();

    let mut dom = VirtualDom::new(App);
    let mutations = dom.rebuild_async().await.unwrap();
    dbg!(mutations);
}

// #[async_std::test]
// async fn websys_loop() {
//     ///loop {
//     ///    let deadline = request_idle_callback().await;
//     ///    let edits = dom.work(deadline);
//     ///    request_animation_frame().await;
//     ///    apply(edits);
//     ///}
// }
