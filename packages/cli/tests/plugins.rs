use dioxus_cli::plugin::{convert::ConvertWithState, load_plugin, CliPlugin};

#[tokio::test]
async fn load_plugin_works() -> wasmtime::Result<()> {
    let plugin = load_plugin("../cli-plugin/examples/output.wasm").await?;

    let CliPlugin {
        bindings,
        mut store,
        ..
    } = plugin;

    let real_toml: toml::Value = toml::from_str(
        r#"
    ip = '127.0.0.1'
 
    [keys]
    github = 'xxxxxxxxxxxxxxxxx'
    travis = 'yyyyyyyyyyyyyyyyy'
 "#,
    )
    .unwrap();

    let real_handle = real_toml.clone().convert_with_state(store.data_mut()).await;

    let lib = bindings.plugins_main_definitions();
    let _val = lib.call_get_default_config(&mut store).await?;
    let toml_val = store
        .data()
        .tomls
        .get(real_handle.rep() as usize)
        .cloned()
        .unwrap()
        .convert_with_state(store.data_mut())
        .await;
    let _ = lib.call_apply_config(&mut store, real_handle).await?;
    assert_eq!(toml_val, real_toml);

    Ok(())
}
