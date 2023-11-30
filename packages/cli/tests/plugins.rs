use dioxus_cli::plugin::{interface::ConvertWithState, load_plugin, CliPlugin};

#[tokio::test]
async fn load_plugin_works() -> wasmtime::Result<()> {
    let plugin = load_plugin("../cli-plugin/examples/output.wasm").await?;

    let CliPlugin {
        bindings,
        mut store,
        ..
    } = plugin;

    let real_toml = toml::Value::Array((0..10).map(toml::Value::Integer).collect());

    let lib = bindings.plugins_main_definitions();
    let val = lib.call_get_default_config(&mut store).await?;
    let toml_val = store
        .data()
        .tomls
        .get(val.rep() as usize)
        .cloned()
        .unwrap()
        .convert_with_state(store.data_mut())
        .await;
    assert_eq!(toml_val, real_toml);

    Ok(())
}
