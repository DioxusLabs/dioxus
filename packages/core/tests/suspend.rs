use dioxus_core::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

#[tokio::test]
async fn it_works() {
    let mut dom = VirtualDom::new(app);

    let mutations = dom.rebuild();

    println!("mutations: {:?}", mutations);

    dom.wait_for_work().await;
}

fn app(cx: Scope) -> Element {
    println!("running root app");

    VNode::template_from_dynamic_node(
        cx,
        cx.component(suspense_boundary, (), "suspense_boundary"),
        "app",
    )
}

fn suspense_boundary(cx: Scope) -> Element {
    println!("running boundary");

    let _ = cx.use_hook(|| {
        cx.provide_context(Rc::new(RefCell::new(SuspenseBoundary::new(cx.scope_id()))))
    });

    VNode::template_from_dynamic_node(cx, cx.component(async_child, (), "async_child"), "app")
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

    VNode::template_from_dynamic_node(cx, cx.component(async_text, (), "async_text"), "app")
}

async fn async_text(cx: Scope<'_>) -> Element {
    VNode::single_text(&cx, &[TemplateNode::Text("it works!")], "beauty")
}
