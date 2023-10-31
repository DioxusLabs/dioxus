#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

mod cache;
pub mod config;
mod fs_cache;
#[cfg(feature = "incremental")]
pub mod incremental;
#[cfg(feature = "incremental")]
mod incremental_cfg;

pub mod renderer;
pub mod template;

use dioxus_core::{Element, LazyNodes, Scope, VirtualDom};
use std::cell::Cell;

pub use crate::renderer::Renderer;

/// A convenience function to render an `rsx!` call to a string
///
/// For advanced rendering, create a new `SsrRender`.
pub fn render_lazy(f: LazyNodes<'_, '_>) -> String {
    // We need to somehow get the lazy call into the virtualdom even with the lifetime
    // Since the lazy lifetime is valid for this function, we can just transmute it to static temporarily
    // This is okay since we're returning an owned value
    struct RootProps<'a, 'b> {
        caller: Cell<Option<LazyNodes<'a, 'b>>>,
    }

    fn lazy_app<'a>(cx: Scope<'a, RootProps<'static, 'static>>) -> Element<'a> {
        let lazy = cx.props.caller.take().unwrap();
        let lazy: LazyNodes = unsafe { std::mem::transmute(lazy) };
        Some(lazy.call(cx))
    }

    let props: RootProps = unsafe {
        std::mem::transmute(RootProps {
            caller: Cell::new(Some(f)),
        })
    };

    let mut dom = VirtualDom::new_with_props(lazy_app, props);
    _ = dom.rebuild();

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
