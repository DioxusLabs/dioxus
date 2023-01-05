#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::*;
use serde::{Deserialize, Serialize};

fn main() {
    dioxus_web::launch(app);
}

#[derive(Serialize, Deserialize, PartialEq, Routable)]
enum Routes {
    #[at("/")]
    Home,

    #[at("/blog")]
    Blog,

    #[at("/blog")]
    BlogPost { id: String },
}

fn app(cx: Scope) -> Element {
    render! {
        Router {
            div { class: "nav",
                Link { to: Routes::Home, "Home" }
                Link { to: Routes::Blog, "Blog" }
                Link { to: Routes::BlogPost { id: "tim".into() }, "Tim's blog" }
            }
            Outlet::<Routes> {}
        }
    }
}

fn Home(cx: Scope) -> Element {
    render! { h1 { "Home" }  }
}

fn Blog(cx: Scope) -> Element {
    render! { div { "Blog List" } }
}

fn BlogPost(cx: Scope) -> Element {
    let Some(id) = use_route(cx).segment("id") else {
        return render!( div { "No blog post id"  } )
    };

    render! {
        div {
            h3 { "blog post: {id:?}"  }
            Link { to: "/blog/", "back to blog list" }
        }
    }
}
