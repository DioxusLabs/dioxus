use dioxus_ssr::{
    prelude::*,
    prelude::{builder::IntoDomTree, dioxus::events::on::MouseEvent},
    TextRenderer,
};

fn main() {
    TextRenderer::new(App);
}

fn App(ctx: Context, props: &()) -> DomTree {}
