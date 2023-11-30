
use dioxus_cli::plugin::{load_plugin, CliPlugin, interface::ConvertWithState};

#[tokio::test]
async fn load_plugin_works() -> wasmtime::Result<()> {
    let plugin = load_plugin("../cli-plugin/examples/output.wasm").await?;

    let CliPlugin {bindings, mut store, .. } = plugin;

    let lib = bindings.plugins_main_definitions();
    let val = lib.call_get_default_config(&mut store).await?;
    let toml_val = store.data().tomls.get(val.rep() as usize).cloned().unwrap().convert_with_state(store.data_mut()).await;
    dbg!(toml_val);

    Ok(())
}
