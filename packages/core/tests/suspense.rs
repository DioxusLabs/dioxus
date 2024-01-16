use dioxus::prelude::*;

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

    if val() < 3 {
        spawn(async move {
            val += 1;
        });
        suspend()?;
    }

    rsx!("child")
}
