use dioxus_core::prelude::*;
use recoil::*;

const COUNT: Atom<i32> = |_| 0;

static App: FC<()> = |ctx| {
    use_init_recoil_root(ctx, |_| {});

    let (count, set_count) = use_read_write(&ctx, &COUNT);

    rsx! { in ctx,
        div {
            "Count: {count}"
            br {}
            button { onclick: move |_| set_count(count + 1), "<Incr" }
            ">___<"
            button { onclick: move |_| set_count(count - 1), "Decr>" }
        }
    }
};

fn main() {
    // Setup logging
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    log::debug!("asd");
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App));
}
