//! Exentsions to the incremental renderer to support pre-caching static routes.
use std::str::FromStr;

use dioxus::prelude::*;
use dioxus_ssr::incremental::{IncrementalRenderer, IncrementalRendererError, RenderHTML};

use crate::prelude::*;

trait IncrementalRendererRouterExt {
    /// Pre-cache all static routes.
    fn pre_cache_static_routes<Rt>(&mut self) -> Result<(), IncrementalRendererError>
    where
        Rt: Routable,
        <Rt as FromStr>::Err: std::fmt::Display;

    /// Render a route to a writer.
    fn render_route<Rt, W, F: FnOnce(&mut VirtualDom)>(
        &mut self,
        route: Rt,
        writer: &mut W,
        modify_vdom: F,
    ) -> Result<(), IncrementalRendererError>
    where
        Rt: Routable,
        <Rt as FromStr>::Err: std::fmt::Display,
        W: std::io::Write;
}

impl<R: RenderHTML> IncrementalRendererRouterExt for IncrementalRenderer<R> {
    fn pre_cache_static_routes<Rt>(&mut self) -> Result<(), IncrementalRendererError>
    where
        Rt: Routable,
        <Rt as FromStr>::Err: std::fmt::Display,
    {
        for route in Rt::SITE_MAP
            .iter()
            .flat_map(|seg| seg.flatten().into_iter())
        {
            // check if this is a static segment
            let mut is_static = true;
            let mut full_path = String::new();
            for segment in &route {
                match segment {
                    SegmentType::Static(s) => {
                        full_path += "/";
                        full_path += s;
                    }
                    _ => {
                        // skip routes with any dynamic segments
                        is_static = false;
                        break;
                    }
                }
            }

            if is_static {
                match Rt::from_str(&full_path) {
                    Ok(route) => {
                        self.render_route(route, &mut std::io::sink(), |_| {})?;
                    }
                    Err(e) => {
                        log::error!("Error pre-caching static route: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    fn render_route<Rt, W, F: FnOnce(&mut VirtualDom)>(
        &mut self,
        route: Rt,
        writer: &mut W,
        modify_vdom: F,
    ) -> Result<(), IncrementalRendererError>
    where
        Rt: Routable,
        <Rt as FromStr>::Err: std::fmt::Display,
        W: std::io::Write,
    {
        #[inline_props]
        fn RenderPath<R>(cx: Scope, path: R) -> Element
        where
            R: Routable,
            <R as FromStr>::Err: std::fmt::Display,
        {
            let path = path.clone();
            render! {
                GenericRouter::<R> {
                    config: || RouterConfig::default().history(MemoryHistory::with_initial_path(path))
                }
            }
        }

        self.render(
            route.to_string(),
            RenderPath,
            RenderPathProps { path: route },
            writer,
            modify_vdom,
        )
    }
}
