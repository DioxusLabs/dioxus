#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

mod cache;
pub mod config;
#[cfg(feature = "incremental")]
mod fs_cache;
#[cfg(feature = "incremental")]
pub mod incremental;
#[cfg(feature = "incremental")]
mod incremental_cfg;

pub mod renderer;
pub mod template;

use dioxus_core::NoOpMutations;
use dioxus_core::{Element, VirtualDom};

pub use crate::renderer::Renderer;

/// A convenience function to render an `rsx!` call to a string
///
/// For advanced rendering, create a new `SsrRender`.
pub fn render_element(element: Element) -> String {
    fn lazy_app(props: Element) -> Element {
        props
    }

    let mut dom = VirtualDom::new_with_props(lazy_app, element);
    dom.rebuild(&mut NoOpMutations);

    Renderer::new().render(&dom)
}

/// A convenience function to render an existing VirtualDom to a string
///
/// We generally recommend creating a new `Renderer` to take advantage of template caching.
pub fn render(dom: &VirtualDom) -> String {
    Renderer::new().render(dom)
}

/// A convenience function to pre-render an existing VirtualDom to a string
///
/// We generally recommend creating a new `Renderer` to take advantage of template caching.
pub fn pre_render(dom: &VirtualDom) -> String {
    let mut renderer = Renderer::new();
    renderer.pre_render = true;
    renderer.render(dom)
}
