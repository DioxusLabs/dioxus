#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::*;

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .with_module_level("dioxus_router", log::LevelFilter::Trace)
        .with_module_level("dioxus", log::LevelFilter::Trace)
        .init()
        .unwrap();
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            h1 { "Your app here" }
            ul {
                Link { to: "/", li { "home" } }
                Link { to: "/blog", li { "blog" } }
                Link { to: "/blog/tim", li { "tims' blog" } }
                Link { to: "/blog/bill", li { "bills' blog" } }
                Link { to: "/blog/james",
                        li { "james amazing' blog" }
                }
                Link { to: "/apples", li { "go to apples" } }
            }
            Route { to: "/", Home {} }
            Route { to: "/blog/", BlogList {} }
            Route { to: "/blog/:id/", BlogPost {} }
            Route { to: "/oranges", "Oranges are not apples!" }
            Redirect { from: "/apples", to: "/oranges" }
        }
    })
}

fn Home(cx: Scope) -> Element {
    log::debug!("rendering home {:?}", cx.scope_id());
    cx.render(rsx! { h1 { "Home" } })
}

fn BlogList(cx: Scope) -> Element {
    log::debug!("rendering blog list {:?}", cx.scope_id());
    cx.render(rsx! { div { "Blog List" } })
}

fn BlogPost(cx: Scope) -> Element {
    let Some(id) = use_route(cx).segment("id") else {
        return cx.render(rsx! { div { "No blog post id" } })
    };

    log::debug!("rendering blog post {}", id);

    cx.render(rsx! {
        div {
            h3 { "blog post: {id:?}"  }
            Link { to: "/blog/", "back to blog list" }
        }
    })
}
