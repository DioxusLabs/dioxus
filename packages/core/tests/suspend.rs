use std::{cell::Cell, ptr::null_mut, time::Duration};

use dioxus_core::*;

#[tokio::test]
async fn it_works() {
    let mut dom = VirtualDom::new(app);

    let mut mutations = vec![];
    dom.rebuild(&mut mutations);

    println!("mutations: {:?}", mutations);

    dom.wait_for_work().await;
}

fn app(cx: Scope) -> Element {
    let dy = cx.component(async_child, (), "async_child");
    VNode::single_component(&cx, dy, "app")
}

async fn async_child(cx: Scope<'_>) -> Element {
    println!("rendering async child");

    let fut = cx.use_hook(|| {
        Box::pin(async {
            println!("Starting sleep");
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("Sleep ended");
        })
    });

    fut.await;

    println!("Future awaited and complete");

    let dy = cx.component(async_child, (), "async_child");
    VNode::single_component(&cx, dy, "app")

    // VNode::single_text(&cx, &[TemplateNode::Text("it works!")], "beauty")
}
