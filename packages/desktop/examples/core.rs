use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {
    let (window_loop, tasks) = dioxus_desktop::start(App, |c| c);

    std::thread::spawn(move || {
        //
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async move {
            let mut vir = VirtualDom::new_with_props(root, props);
            let channel = vir.get_event_sender();
            loop {
                vir.wait_for_work().await;
                let edits = vir.run_with_deadline(|| false);
                let edit_string = serde_json::to_string(&edits[0].edits).unwrap();
                event_tx.send(edit_string).unwrap();
            }
        })
    });

    window_loop.run();
}

static App: FC<()> = |cx| {
    //
    cx.render(rsx!(div {}))
};
