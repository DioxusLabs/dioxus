#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::*;

fn main() {
    dioxus_web::launch(app);
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
    cx.render(rsx! { h1 { "Home" } })
}

fn BlogList(cx: Scope) -> Element {
    cx.render(rsx! { div { "Blog List" } })
}

fn BlogPost(cx: Scope) -> Element {
    let id = use_route(&cx).segment("id")?;

    log::trace!("rendering blog post {}", id);

    cx.render(rsx! { div { "{id:?}" } })
}
