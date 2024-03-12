//! Extensions to the incremental renderer to support pre-caching static routes.
use core::pin::Pin;
use std::future::Future;
use std::str::FromStr;

use dioxus_lib::prelude::*;
use dioxus_ssr::incremental::{
    IncrementalRenderer, IncrementalRendererError, RenderFreshness, WrapBody,
};

use crate::prelude::*;

/// Pre-cache all static routes.
pub async fn pre_cache_static_routes<Rt, R: WrapBody + Send + Sync>(
    renderer: &mut IncrementalRenderer,
    wrapper: &R,
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
                    render_route(
                        renderer,
                        route,
                        &mut tokio::io::sink(),
                        |vdom| {
                            Box::pin(async move {
                                vdom.rebuild_in_place();
                                vdom.wait_for_suspense().await;
                            })
                        },
                        wrapper,
                    )
                    .await?;
                }
                Err(e) => {
                    tracing::info!("@ route: {}", full_path);
                    tracing::error!("Error pre-caching static route: {}", e);
                }
            }
        }
    }

    Ok(())
}

/// Render a route to a writer.
pub async fn render_route<
    R: WrapBody + Send + Sync,
    Rt,
    W,
    F: FnOnce(&mut VirtualDom) -> Pin<Box<dyn Future<Output = ()> + '_>>,
>(
    renderer: &mut IncrementalRenderer,
    route: Rt,
    writer: &mut W,
    modify_vdom: F,
    wrapper: &R,
) -> Result<RenderFreshness, IncrementalRendererError>
where
    Rt: Routable,
    <Rt as FromStr>::Err: std::fmt::Display,
    W: tokio::io::AsyncWrite + Unpin + Send,
{
    #[derive(Clone)]
    struct RootProps<Rt>(Rt);

    impl<Rt> PartialEq for RootProps<Rt> {
        fn eq(&self, _: &Self) -> bool {
            true
        }
    }

    fn RenderPath<R>(props: RootProps<R>) -> Element
    where
        R: Routable,
        <R as FromStr>::Err: std::fmt::Display,
    {
        let path = props.0;
        rsx! {
            Router::<R> {
                config: || RouterConfig::default().history(MemoryHistory::with_initial_path(path))
            }
        }
    }

    renderer
        .render(
            route.to_string(),
            || VirtualDom::new_with_props(RenderPath, RootProps(route)),
            writer,
            modify_vdom,
            wrapper,
        )
        .await
}
