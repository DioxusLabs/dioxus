use std::collections::HashMap;

use dioxus_core::prelude::*;
use recoil::*;

const A_ITEMS: AtomHashMap<i32, i32> = |_| HashMap::new();
const B_ITEMS: AtomHashMap<i32, i32> = |_| HashMap::new();

const C_SELECTOR: SelectorFamily<i32, i32> = |api, key| {
    let a = api.get(&A_ITEMS.select(&key));
    let b = api.get(&B_ITEMS.select(&key));
    a + b
};

const D_SELECTOR: SelectorFamilyBorrowed<i32, i32> = |api, key| -> &i32 {
    let a = api.get(&A_ITEMS.select(&key));
    a
};

static App: FC<()> = |cx| {
    use_init_recoil_root(cx, |_| {});

    let title = use_read(cx, &C_SELECTOR);

    rsx! { in cx,
        div {
            "{title}"
            // button { onclick: {next_light}, "Next light" }
        }
    }
};

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}
