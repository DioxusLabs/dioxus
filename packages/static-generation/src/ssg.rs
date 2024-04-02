use dioxus_lib::prelude::*;
use dioxus_router::prelude::*;

use crate::Config;

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

    vdom.in_runtime(|| {
        ScopeId::ROOT.in_runtime(|| dioxus_router::prelude::root_router().map(|r| r.site_map()))
    })
}

/// Generate a static site from any fullstack app that uses the router.
pub async fn generate_static_site(
    app: fn() -> Element,
    mut config: Config,
) -> Result<(), IncrementalRendererError> {
    use tokio::task::block_in_place;

    let mut renderer = config.create_renderer();

    let site_map = block_in_place(|| extract_site_map(app))
        .expect("Failed to find a router in the application");
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

        prerender_route(app, url, &mut renderer, &config).await?;
    }

    Ok(())
}

async fn prerender_route(
    app: fn() -> Element,
    route: String,
    renderer: &mut dioxus_ssr::incremental::IncrementalRenderer,
    config: &Config,
) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError> {
    use dioxus_fullstack::prelude::*;

    let context = server_context_for_route(&route);
    let wrapper = config.fullstack_template(&context);
    renderer
        .render(
            route,
            || VirtualDom::new(app),
            &mut tokio::io::sink(),
            |vdom| {
                Box::pin(async move {
                    with_server_context(context.clone(), || {
                        tokio::task::block_in_place(|| vdom.rebuild_in_place());
                    });
                    ProvideServerContext::new(vdom.wait_for_suspense(), context).await;
                })
            },
            &wrapper,
        )
        .await?;
    Ok(())
}

#[test]
fn extract_site_map_works() {
    use dioxus::prelude::*;
    use dioxus_router::prelude::*;

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
