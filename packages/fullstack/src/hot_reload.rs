use dioxus_hot_reload::HotReloadReceiver;

// Hot reloading can be expensive to start so we spawn a new thread
static HOT_RELOAD_STATE: tokio::sync::OnceCell<HotReloadReceiver> =
    tokio::sync::OnceCell::const_new();
pub(crate) async fn spawn_hot_reload() -> &'static HotReloadReceiver {
    HOT_RELOAD_STATE
        .get_or_init(|| async {
            println!("spinning up hot reloading");
            let r = tokio::task::spawn_blocking(dioxus_hot_reload::connect_hot_reload)
                .await
                .unwrap();
            println!("hot reloading ready");
            r
        })
        .await
}
