use std::process::exit;

use dioxus_rsx::{BodyNode, CallBody, TemplateBody};

use super::*;

/// Translate some source file into Dioxus code
#[derive(Clone, Debug, Parser)]
#[clap(name = "http-server")]
pub struct Httpserver {}

impl Httpserver {
    pub async fn serve(self) -> Result<()> {
        todo!()
    }
}
