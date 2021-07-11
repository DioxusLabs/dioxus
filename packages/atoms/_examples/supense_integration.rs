// const Posts: AtomFamily = |family| {
//     family.on_get(|key| {

//         //
//     })
// };

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}

use std::future::Future;

use dioxus_core::prelude::*;

static App: FC<()> = |cx| {
    //

    let title = use_async_atom();
    let title_card = suspend(&cx, title, move |val| {
        //
        rsx!(in cx, div {
            h3 { "{val}" }
        })
    });

    // let fut = (use_async_atom(), use_async_atom());
    // let title_card2 = cx.suspend(fut, move |(text, text2)| {
    //     cx.render(rsx!( h3 { "{text}" } ))
    // });

    cx.render(rsx! {
        div {
            {title_card}
            // {title_card2}
        }
    })
};

async fn use_async_atom() -> String {
    todo!()
}

fn suspend<'a, O>(
    c: &impl Scoped<'a>,
    f: impl Future<Output = O>,
    g: impl FnOnce(O) -> VNode<'a> + 'a,
) -> VNode<'a> {
    todo!()
}
