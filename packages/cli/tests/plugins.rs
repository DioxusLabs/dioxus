
use dioxus_cli::plugin::load_plugin;

#[tokio::test]
async fn load_plugin_works() -> wasmtime::Result<()> {
    let plugin = load_plugin("../cli-plugin/examples/output.wasm").await?;

    Ok(())
}
