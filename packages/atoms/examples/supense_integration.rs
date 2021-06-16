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

static App: FC<()> = |ctx| {
    //

    let title = use_async_atom();
    let title_card = suspend(&ctx, title, move |val| {
        //
        rsx!(in ctx, div {
            h3 { "{val}" }
        })
    });

    // let fut = (use_async_atom(), use_async_atom());
    // let title_card2 = ctx.suspend(fut, move |(text, text2)| {
    //     ctx.render(rsx!( h3 { "{text}" } ))
    // });

    ctx.render(rsx! {
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
