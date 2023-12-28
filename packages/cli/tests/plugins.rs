use dioxus_cli::{
    plugin::{convert::ConvertWithState, load_plugin},
    DioxusLock,
};

#[tokio::test]
async fn load_plugin_works() -> dioxus_cli::Result<()> {
    let dioxus_lock = DioxusLock::load()?;
    let mut plugin = load_plugin("../cli-plugin/examples/output.wasm", &dioxus_lock).await?;

    // let CliPlugin {
    //     bindings,
    //     mut store,
    //     ..
    // } = plugin;

    let real_toml: toml::Value = toml::from_str(
        r#"
    ip = '127.0.0.1'
 
    [keys]
    github = 'xxxxxxxxxxxxxxxxx'
    travis = 'yyyyyyyyyyyyyyyyy'
 "#,
    )
    .unwrap();

    let real_handle = real_toml.clone().convert_with_state(plugin.as_mut()).await;

    let toml_val = plugin.get(plugin.clone_handle(&real_handle)).await;
    let _ = plugin.apply_config(real_handle).await?;
    assert_eq!(toml_val, real_toml);

    Ok(())
}
