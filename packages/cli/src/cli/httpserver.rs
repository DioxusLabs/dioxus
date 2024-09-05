use super::*;

/// Translate some source file into Dioxus code
#[derive(Clone, Debug, Parser)]
#[clap(name = "http-server")]
pub(crate) struct Httpserver {}

impl Httpserver {
    pub(crate) async fn serve(self) -> Result<()> {
        todo!()
    }
}
