#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use std::io::prelude::*;
use std::{path::PathBuf, str::FromStr};

fn main() {
    render_static_pages();
}

fn render_static_pages() {
    for route in Route::SITE_MAP
        .iter()
        .flat_map(|seg| seg.flatten().into_iter())
    {
        // check if this is a static segment
        let mut file_path = PathBuf::from("./");
        let mut full_path = String::new();
        let mut is_static = true;
        for segment in &route {
            match segment {
                SegmentType::Static(s) => {
                    file_path.push(s);
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
            let route = Route::from_str(&full_path).unwrap();
            let mut vdom = VirtualDom::new_with_props(RenderPath, RenderPathProps { path: route });
            let _ = vdom.rebuild();

            file_path.push("index.html");
            std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
            let mut file = std::fs::File::create(file_path).unwrap();

            let body = dioxus_ssr::render(&vdom);
            let html = format!(
                r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
</head>
<body>
    {}
</body>
</html>
"#,
                full_path, body
            );
            file.write_all(html.as_bytes()).unwrap();
        }
    }
}

#[inline_props]
fn RenderPath(cx: Scope, path: Route) -> Element {
    let path = path.clone();
    render! {
        Router {
            config: || RouterConfig::default().history(MemoryHistory::with_initial_path(path))
        }
    }
}

#[inline_props]
fn Blog(cx: Scope) -> Element {
    render! {
        div {
            "Blog"
        }
    }
}

#[inline_props]
fn Post(cx: Scope) -> Element {
    render! {
        div {
            "Post"
        }
    }
}

#[inline_props]
fn Home(cx: Scope) -> Element {
    render! {
        div {
            "Home"
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[nest("/blog")]
        #[route("/")]
        Blog {},
        #[route("/post")]
        Post {},
    #[end_nest]
    #[route("/")]
    Home {},
}
