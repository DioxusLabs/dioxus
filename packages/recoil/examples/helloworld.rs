use dioxus_core::prelude::*;
use recoil::*;

const COUNT: Atom<i32> = |_| 0;

static App: FC<()> = |ctx, _| {
    let (count, set_count) = use_recoil_state(ctx, &COUNT);

    rsx! { in ctx,
        div {
            "Count: {count}"
            button { onclick: move |_| set_count(count + 1), "Incr" }
            button { onclick: move |_| set_count(count - 1), "Decr" }
        }
    }
};

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}
