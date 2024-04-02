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

#[cfg(all(feature = "ssr", feature = "fullstack"))]
fn server_context_for_route(route: &str) -> dioxus_fullstack::prelude::DioxusServerContext {
    use dioxus_fullstack::prelude::*;
    use std::sync::Arc;
    let request = http::Request::builder().uri(route).body(()).unwrap();
    let (parts, _) = request.into_parts();
    let server_context = DioxusServerContext::new(Arc::new(tokio::sync::RwLock::new(parts)));
    server_context
}

/// Try to extract the site map by finding the root router that a component renders.
fn extract_site_map(app: fn() -> Element) -> Option<&'static [SiteMapSegment]> {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    vdom.in_runtime(|| ScopeId::ROOT.in_runtime(|| root_router().map(|r| r.site_map())))
}

#[cfg(all(feature = "ssr", feature = "fullstack"))]
/// Generate a static site from any fullstack app that uses the router.
pub async fn generate_static_site<R: WrapBody + Send + Sync>(
    app: fn() -> Element,
    renderer: &mut IncrementalRenderer,
    wrapper: &R,
) -> Result<(), IncrementalRendererError> {
    use dioxus_fullstack::prelude::ProvideServerContext;
    use tokio::task::block_in_place;
    
    let site_map = extract_site_map(app).expect("Failed to find a router in the application");
    let flat_site_map = site_map.iter().flat_map(SiteMapSegment::flatten);

    for route in flat_site_map {
        let Some(static_route) = route
            .iter()
            .map(SegmentType::to_static)
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let url = format!("/{}", static_route.join("/"));

        let context = server_context_for_route(&url);
        let future = async {
            renderer
                .render(
                    url,
                    || VirtualDom::new(app),
                    &mut tokio::io::sink(),
                    |vdom| {
                        Box::pin(async move {
                            block_in_place(|| vdom.rebuild_in_place());
                            vdom.wait_for_suspense().await;
                        })
                    },
                    wrapper,
                )
                .await
        };
        ProvideServerContext::new(future, context).await?;
    }

    Ok(())
}

#[test]
fn extract_site_map_works() {
    use dioxus::prelude::*;

    #[derive(Clone, Routable, Debug, PartialEq)]
    enum Route {
        #[route("/")]
        Home {},
        #[route("/about")]
        About {},
    }

    fn Home() -> Element {
        rsx! { "Home" }
    }

    fn About() -> Element {
        rsx! { "About" }
    }

    fn app() -> Element {
        rsx! {
            div {
                Other {}
            }
        }
    }

    fn Other() -> Element {
        rsx! {
            Router::<Route> {}
        }
    }

    let site_map = extract_site_map(app);
    assert_eq!(site_map, Some(Route::SITE_MAP));
}
