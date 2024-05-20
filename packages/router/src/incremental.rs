//! Extensions to the incremental renderer to support pre-caching static routes.
use core::pin::Pin;
use std::future::Future;
use std::str::FromStr;

use crate::prelude::*;
use dioxus_lib::prelude::*;

/// Pre-cache all static routes.
pub async fn pre_cache_static_routes<Rt, R: WrapBody + Send + Sync>(
    renderer: &mut IncrementalRenderer,
    wrapper: &R,
) -> Result<(), IncrementalRendererError>
where
    Rt: Routable,
    <Rt as FromStr>::Err: std::fmt::Display,
{
    for route in Rt::static_routes() {
        render_route(
            renderer,
            route,
            &mut std::io::sink(),
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
    W: std::io::Write + Unpin + Send,
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
