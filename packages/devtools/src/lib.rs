use dioxus_core::internal::HotReloadedTemplate;
use dioxus_core::{ScopeId, VirtualDom};
use dioxus_signals::{GlobalKey, Signal, WritableExt};

pub use dioxus_devtools_types::*;
pub use subsecond;
use subsecond::PatchError;

/// Applies template and literal changes to the VirtualDom
///
/// Assets need to be handled by the renderer.
pub fn apply_changes(dom: &VirtualDom, msg: &HotReloadMsg) {
    try_apply_changes(dom, msg).unwrap()
}

/// Applies template and literal changes to the VirtualDom, but doesn't panic if patching fails.
///
/// Assets need to be handled by the renderer.
pub fn try_apply_changes(dom: &VirtualDom, msg: &HotReloadMsg) -> Result<(), PatchError> {
    dom.runtime().in_scope(ScopeId::ROOT, || {
        // 1. Update signals...
        let ctx = dioxus_signals::get_global_context();
        for template in &msg.templates {
            let value = template.template.clone();
            let key = GlobalKey::File {
                file: template.key.file.as_str(),
                line: template.key.line as _,
                column: template.key.column as _,
                index: template.key.index as _,
            };
            if let Some(mut signal) = ctx.get_signal_with_key(key.clone()) {
                signal.set(Some(value));
            }
        }

        // 2. Attempt to hotpatch
        if let Some(jump_table) = msg.jump_table.as_ref().cloned() {
            if msg.for_build_id == Some(dioxus_cli_config::build_id()) {
                let our_pid = if cfg!(target_family = "wasm") {
                    None
                } else {
                    Some(std::process::id())
                };

                if msg.for_pid == our_pid {
                    unsafe { subsecond::apply_patch(jump_table) }?;
                    dom.runtime().force_all_dirty();
                    ctx.clear::<Signal<Option<HotReloadedTemplate>>>();
                }
            }
        }

        Ok(())
    })
}

/// Connect to the devserver and handle its messages with a callback.
///
/// This doesn't use any form of security or protocol, so it's not safe to expose to the internet.
#[cfg(not(target_family = "wasm"))]
pub fn connect(callback: impl FnMut(DevserverMsg) + Send + 'static) {
    let Some(endpoint) = dioxus_cli_config::devserver_ws_endpoint() else {
        return;
    };

    connect_at(endpoint, callback);
}

/// Connect to the devserver and handle hot-patch messages only, implementing the subsecond hotpatch
/// protocol.
///
/// This is intended to be used by non-dioxus projects that want to use hotpatching.
///
/// To handle the full devserver protocol, use `connect` instead.
#[cfg(not(target_family = "wasm"))]
pub fn connect_subsecond() {
    connect(|msg| {
        if let DevserverMsg::HotReload(hot_reload_msg) = msg {
            if let Some(jumptable) = hot_reload_msg.jump_table {
                if hot_reload_msg.for_pid == Some(std::process::id()) {
                    unsafe { subsecond::apply_patch(jumptable).unwrap() };
                }
            }
        }
    });
}

#[cfg(not(target_family = "wasm"))]
pub fn connect_at(endpoint: String, mut callback: impl FnMut(DevserverMsg) + Send + 'static) {
    std::thread::spawn(move || {
        let uri = format!(
            "{endpoint}?aslr_reference={}&build_id={}&pid={}",
            subsecond::aslr_reference(),
            dioxus_cli_config::build_id(),
            std::process::id()
        );

        let (mut websocket, _req) = match tungstenite::connect(uri) {
            Ok((websocket, req)) => (websocket, req),
            Err(_) => return,
        };

        while let Ok(msg) = websocket.read() {
            if let tungstenite::Message::Text(text) = msg {
                if let Ok(msg) = serde_json::from_str(&text) {
                    callback(msg);
                }
            }
        }
    });
}

/// Run this asynchronous future to completion.
///
/// Whenever your code changes, the future is dropped and a new one is created using the new function.
///
/// This is useful for using subsecond outside of dioxus, like with axum. To pass args to the underlying
/// function, you can use the `serve_subsecond_with_args` function.
///
/// ```rust, ignore
/// #[tokio::main]
/// async fn main() {
///     dioxus_devtools::serve_subsecond(router_main).await;
/// }
///
/// async fn router_main() {
///     use axum::{Router, routing::get};
///
///     let app = Router::new().route("/", get(test_route));
///
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
///     println!("Server running on http://localhost:3000");
///
///     axum::serve(listener, app.clone()).await.unwrap()
/// }
///
/// async fn test_route() -> axum::response::Html<&'static str> {
///     "axum works!!!!!".into()
/// }
/// ```
#[cfg(feature = "serve")]
#[cfg(not(target_family = "wasm"))]
pub async fn serve_subsecond<O, F>(mut callback: impl FnMut() -> F)
where
    F: std::future::Future<Output = O> + 'static,
{
    serve_subsecond_with_args((), move |_args| callback()).await
}

/// Run this asynchronous future to completion.
///
/// Whenever your code changes, the future is dropped and a new one is created using the new function.
///
/// ```rust, ignore
/// #[tokio::main]
/// async fn main() {
///     let args = ("abc".to_string(),);
///     dioxus_devtools::serve_subsecond_with_args(args, router_main).await;
/// }
///
/// async fn router_main(args: (String,)) {
///     use axum::{Router, routing::get};
///
///     let app = Router::new().route("/", get(test_route));
///
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
///     println!("Server running on http://localhost:3000 -> {}", args.0);
///
///     axum::serve(listener, app.clone()).await.unwrap()
/// }
///
/// async fn test_route() -> axum::response::Html<&'static str> {
///     "axum works!!!!!".into()
/// }
/// ```
#[cfg(feature = "serve")]
pub async fn serve_subsecond_with_args<A: Clone, O, F>(args: A, mut callback: impl FnMut(A) -> F)
where
    F: std::future::Future<Output = O> + 'static,
{
    let (tx, mut rx) = futures_channel::mpsc::unbounded();

    connect(move |msg| {
        if let DevserverMsg::HotReload(hot_reload_msg) = msg {
            if let Some(jumptable) = hot_reload_msg.jump_table {
                if hot_reload_msg.for_pid == Some(std::process::id()) {
                    unsafe { subsecond::apply_patch(jumptable).unwrap() };
                    tx.unbounded_send(()).unwrap();
                }
            }
        }
    });

    let wrapped = move |args| -> std::pin::Pin<Box<dyn std::future::Future<Output = O>>> {
        Box::pin(callback(args))
    };

    let mut hotfn = subsecond::HotFn::current(wrapped);
    let mut cur_future = hotfn.call((args.clone(),));

    loop {
        use futures_util::StreamExt;
        let res = futures_util::future::select(cur_future, rx.next()).await;

        match res {
            futures_util::future::Either::Left(_completed) => _ = rx.next().await,
            futures_util::future::Either::Right((None, callback)) => {
                // Receiving `None` here means that the sender is not connected, which
                // typically means the dioxus devtools protocol has never connected.
                // We want to run the future to completion and return instead of
                // re-running the future constantly in the loop.
                callback.await;
                return;
            }
            futures_util::future::Either::Right((Some(_), _)) => {}
        }

        cur_future = hotfn.call((args.clone(),));
    }
}
