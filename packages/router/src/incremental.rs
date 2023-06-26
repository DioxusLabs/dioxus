//! Exentsions to the incremental renderer to support pre-caching static routes.
use std::str::FromStr;

use dioxus::prelude::*;
use dioxus_ssr::incremental::{
    IncrementalRenderer, IncrementalRendererError, RenderFreshness, RenderHTML,
};

use crate::prelude::*;

/// Pre-cache all static routes.
pub async fn pre_cache_static_routes<Rt, R: RenderHTML + Send>(
    renderer: &mut IncrementalRenderer<R>,
) -> Result<(), IncrementalRendererError>
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
                SegmentType::Child => {}
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
                    render_route(renderer, route, &mut tokio::io::sink(), |_| {}).await?;
                }
                Err(e) => {
                    log::info!("@ route: {}", full_path);
                    log::error!("Error pre-caching static route: {}", e);
                }
            }
        }
    }

    Ok(())
}

/// Render a route to a writer.
pub async fn render_route<R: RenderHTML + Send, Rt, W, F: FnOnce(&mut VirtualDom)>(
    renderer: &mut IncrementalRenderer<R>,
    route: Rt,
    writer: &mut W,
    modify_vdom: F,
) -> Result<RenderFreshness, IncrementalRendererError>
where
    Rt: Routable,
    <Rt as FromStr>::Err: std::fmt::Display,
    W: tokio::io::AsyncWrite + Unpin + Send,
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

    renderer
        .render(
            route.to_string(),
            RenderPath,
            RenderPathProps { path: route },
            writer,
            modify_vdom,
        )
        .await
}
