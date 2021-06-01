use dioxus_ssr::{
    prelude::*,
    prelude::{builder::IntoVNode, dioxus::events::on::MouseEvent},
    TextRenderer,
};

fn main() {
    TextRenderer::new(App);
}

fn App(ctx: Context<()>) -> VNode {
    todo!()
}
