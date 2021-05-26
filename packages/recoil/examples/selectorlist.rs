use std::collections::HashMap;

use dioxus_core::prelude::*;
use recoil::*;

const A_ITEMS: AtomFamily<i32, i32> = |_| HashMap::new();
const B_ITEMS: AtomFamily<i32, i32> = |_| HashMap::new();

const C_SELECTOR: SelectorFamily<i32, i32> = |api, key| {
    let a = api.get(&A_ITEMS.select(&key));
    let b = api.get(&B_ITEMS.select(&key));
    a + b
};

const D_SELECTOR: SelectorFamilyBorrowed<i32, i32> = |api, key| -> &i32 {
    let a = api.get(&A_ITEMS.select(&key));
    a
};

static App: FC<()> = |ctx, _| {
    use_init_recoil_root(ctx);

    let title = use_recoil_value(ctx, &C_SELECTOR);
    let title = "";
    rsx! { in ctx,
        div {
            "{title}"
            // button { onclick: {next_light}, "Next light" }
        }
    }
};

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}
