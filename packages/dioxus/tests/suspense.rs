use std::future::IntoFuture;

use dioxus::prelude::*;

#[inline_props]
fn suspense_boundary<'a>(cx: Scope<'a>, children: Element<'a>) -> Element {
    cx.use_hook(|| cx.provide_context(SuspenseBoundary::new(cx.scope_id())));
    cx.render(rsx! { children })
}

fn basic_child(cx: Scope) -> Element {
    cx.render(rsx! {
        div { "basic child 1" }
    })
}

async fn async_child(cx: Scope<'_>) -> Element {
    let username = use_future!(cx, || async {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        "async child 1"
    });

    let age = use_future!(cx, || async {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        println!("long future completed");
        1234
    });

    let (_user, _age) = use_future!(cx, || async {
        tokio::join!(
            tokio::time::sleep(std::time::Duration::from_secs(1)),
            tokio::time::sleep(std::time::Duration::from_secs(2))
        );
        ("async child 1", 1234)
    })
    .await;

    let (username, age) = tokio::join!(username.into_future(), age.into_future());

    cx.render(rsx!(
        div { "Hello! {username}, you are {age}, {_user} {_age}" }
    ))
}

#[tokio::test]
async fn basic_prints() {
    let mut dom = VirtualDom::new(|cx| {
        cx.render(rsx! {
            div {
                h1 { "var" }
                suspense_boundary {
                    basic_child { }
                    async_child { }
                }
            }
        })
    });

    dbg!(dom.rebuild());

    dom.wait_for_work().await;

    dbg!(dom.rebuild());
}
