use dioxus_studio::{set_up_logging, LaunchCommand, LaunchOptions};

#[async_std::main]
async fn main() -> dioxus_studio::Result<()> {
    set_up_logging();

    let opts: LaunchOptions = argh::from_env();

    dioxus_studio::Studio::new(opts).start().await?;

    Ok(())
}
