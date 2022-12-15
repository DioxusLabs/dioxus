use dioxus_interpreter_js::INTERPRETER_JS;
static MAIN_JS: &str = include_str!("./main.js");

pub fn interpreter_glue(url: &str) -> String {
    format!(
        r#"
<script>
    var WS_ADDR = "{url}";
    {INTERPRETER_JS}
    {MAIN_JS}
    main();
</script>
    "#
    )
}

pub mod adapters {
    #[cfg(feature = "warp")]
    pub mod warp_adapter;

    #[cfg(feature = "axum")]
    pub mod axum_adapter;

    #[cfg(feature = "salvo")]
    pub mod salvo_adapter;
}

pub mod pool;
pub use pool::*;

#[derive(Debug, thiserror::Error)]
pub enum LiveViewError {
    #[error("Connection Failed")]
    Warp(#[from] warp::Error),
}
