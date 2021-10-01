use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;

use dioxus_html as dioxus_elements;

fn main() {
    std::thread::spawn(|| {
        let mut vdom = VirtualDom::new(App);
        let f = async_std::task::block_on(vdom.wait_for_work());
    });
    let a = 10;
    // async_std::task::spawn_blocking(|| async move {
    // });
}

static App: FC<()> = |cx, props| {
    //
    cx.render(rsx!(
        div {
            "hello world!"
        }
    ))
};
