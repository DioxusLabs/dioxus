//! Extensions to the incremental renderer to support pre-caching static routes.
use std::str::FromStr;

use crate::prelude::*;
use dioxus_lib::prelude::*;
use dioxus_ssr::Renderer;

/// Pre-cache all static routes.
pub async fn pre_cache_static_routes<Rt, R: WrapBody>(
    renderer: &mut Renderer,
    cache: &mut IncrementalRenderer,
    wrapper: &R,
) -> Result<(), IncrementalRendererError>
where
    Rt: Routable,
    <Rt as FromStr>::Err: std::fmt::Display,
{
    for route in Rt::static_routes() {
        render_route(renderer, cache, route, wrapper).await?;
    }

    Ok(())
}

/// Render a route to a writer.
pub(crate) async fn render_route<R: WrapBody, Rt>(
    renderer: &mut Renderer,
    cache: &mut IncrementalRenderer,
    route: Rt,
    wrapper: &R,
) -> Result<RenderFreshness, IncrementalRendererError>
where
    Rt: Routable,
    <Rt as FromStr>::Err: std::fmt::Display,
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
                config: move |_| RouterConfig::default().history(MemoryHistory::with_initial_path(path.clone()))
            }
        }
    }

    let route_string = route.to_string();
    let mut virtual_dom = VirtualDom::new_with_props(RenderPath, RootProps(route));
    virtual_dom.rebuild_in_place();

    virtual_dom.wait_for_suspense().await;

    let render = renderer.render(&virtual_dom);

    let wrapped = wrapper.wrap_body(&render);

    cache.cache(route_string, wrapped)
}
