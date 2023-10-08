use dioxus::prelude::*;

#[test]
fn it_works() {
    // wait just a moment, not enough time for the boundary to resolve

    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            _ = dom.rebuild();
            dom.wait_for_suspense().await;
            let out = dioxus_ssr::pre_render(&dom);

            assert_eq!(out, "<div>Waiting for... child</div>");

            dbg!(out);
        });
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        div {
            "Waiting for... "
            suspended_child {}
        }
    ))
}

fn suspended_child(cx: Scope) -> Element {
    let val = use_state(cx, || 0);

    if **val < 3 {
        let mut val = val.clone();
        cx.spawn(async move {
            val += 1;
        });
        cx.suspend()?;
    }

    render!("child")
}
