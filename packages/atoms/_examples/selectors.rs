use dioxus_core::prelude::*;
use recoil::*;

const A: Atom<i32> = |_| 0;
const B: Atom<i32> = |_| 0;
const C: Selector<i32> = |api| api.get(&A) + api.get(&B);

static App: FC<()> = |cx| {
    use_init_recoil_root(cx, |_| {});
    rsx! { in cx,
        div {
            Banner {}
            BtnA {}
            BtnB {}
        }
    }
};

static Banner: FC<()> = |cx| {
    let count = use_read(&cx, &C);
    cx.render(rsx! { h1 { "Count: {count}" } })
};

static BtnA: FC<()> = |cx| {
    let (a, set) = use_read_write(&cx, &A);
    rsx! { in cx,
        div { "a"
            button { "+", onclick: move |_| set(a + 1) }
            button { "-", onclick: move |_| set(a - 1) }
        }
    }
};

static BtnB: FC<()> = |cx| {
    let (b, set) = use_read_write(&cx, &B);
    rsx! { in cx,
        div { "b"
            button { "+", onclick: move |_| set(b + 1) }
            button { "-", onclick: move |_| set(b - 1) }
        }
    }
};

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}
