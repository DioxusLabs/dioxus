use dioxus::prelude::*;

#[cfg(feature = "server")]
#[worker::event(fetch)]
async fn main(req: worker::Request, env: worker::Env, ctx: worker::Context) -> worker::Result<worker::Response> {
    let handler = serve_dioxus_application("");
    let rep = handler(req, env);
    rep.await
}
