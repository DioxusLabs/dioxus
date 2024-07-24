use dioxus_lib::prelude::*;
use dioxus_router::prelude::*;
use dioxus_ssr::incremental::*;
use dioxus_ssr::renderer;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::Config;

fn server_context_for_route(route: &str) -> dioxus_fullstack::prelude::DioxusServerContext {
    use dioxus_fullstack::prelude::*;
    let request = http::Request::builder().uri(route).body(()).unwrap();
    let (parts, _) = request.into_parts();

    DioxusServerContext::new(parts)
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

    // Create the static output dir
    std::fs::create_dir_all(&config.output_dir)?;

    let mut renderer = config.create_renderer();
    let mut cache = config.create_cache();

    let mut routes_to_render: HashSet<String> = config.additional_routes.iter().cloned().collect();
    if let Some(site_map) = block_in_place(|| extract_site_map(app)) {
        let flat_site_map = site_map.iter().flat_map(SiteMapSegment::flatten);
        for route in flat_site_map {
            let Some(static_route) = route
                .iter()
                .filter(|s| s.to_child().is_none())
                .map(SegmentType::to_static)
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let url = format!("/{}", static_route.join("/"));

            routes_to_render.insert(url);
        }
    } else {
        tracing::trace!("No site map found, rendering the additional routes");
    }

    for url in routes_to_render {
        prerender_route(app, url, &mut renderer, &mut cache, &config).await?;
    }

    // Copy over the web output dir into the static output dir
    let assets_path = dioxus_cli_config::CURRENT_CONFIG
        .as_ref()
        .map(|c| c.application.out_dir.clone())
        .unwrap_or("./dist".into());

    copy_static_files(&assets_path, &config.output_dir)?;

    Ok(())
}

fn copy_static_files(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    let index_path = src.join("index.html");
    let mut queue = vec![src.to_path_buf()];
    while let Some(path) = queue.pop() {
        if path == index_path {
            continue;
        }
        if path.is_dir() {
            for entry in fs::read_dir(&path).into_iter().flatten().flatten() {
                let path = entry.path();
                queue.push(path);
            }
        } else {
            let output_location = dst.join(path.strip_prefix(src).unwrap());
            let parent = output_location.parent().unwrap();
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
            fs::copy(&path, output_location)?;
        }
    }
    Ok(())
}

async fn prerender_route(
    app: fn() -> Element,
    route: String,
    renderer: &mut renderer::Renderer,
    cache: &mut dioxus_ssr::incremental::IncrementalRenderer,
    config: &Config,
) -> Result<RenderFreshness, dioxus_ssr::incremental::IncrementalRendererError> {
    use dioxus_fullstack::prelude::*;

    let context = server_context_for_route(&route);
    let wrapper = config.fullstack_template();
    let mut virtual_dom = VirtualDom::new(app);
    let document = std::rc::Rc::new(dioxus_fullstack::document::ServerDocument::default());
    virtual_dom.provide_root_context(document.clone() as std::rc::Rc<dyn Document>);
    with_server_context(context.clone(), || {
        tokio::task::block_in_place(|| virtual_dom.rebuild_in_place());
    });
    ProvideServerContext::new(virtual_dom.wait_for_suspense(), context).await;

    let mut wrapped = String::new();

    // Render everything before the body
    wrapper.render_head(&mut wrapped, &virtual_dom)?;

    renderer.render_to(&mut wrapped, &virtual_dom)?;

    wrapper.render_after_main(&mut wrapped, &virtual_dom)?;
    wrapper.render_after_body(&mut wrapped)?;

    cache.cache(route, wrapped)
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
