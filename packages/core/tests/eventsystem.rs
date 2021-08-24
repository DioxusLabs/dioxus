use bumpalo::Bump;

use anyhow::{Context, Result};
use dioxus::{arena::SharedResources, diff::DiffMachine, prelude::*, DomEdit};
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;

#[async_std::test]
async fn event_queue_works() {
    static App: FC<()> = |cx| {
        cx.render(rsx! {
            div { "hello world" }
        })
    };

    let mut dom = VirtualDom::new(App);
    let edits = dom.rebuild();

    async_std::task::spawn_local(async move {
        match dom.run_unbounded().await {
            Err(_) => todo!(),
            Ok(mutations) => {
                //
            }
        }
    });
}
