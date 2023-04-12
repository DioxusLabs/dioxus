#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;

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
    use_router(cx, &RouterConfiguration::default, &|| {
        Segment::content(comp(Home))
            .fixed(
                "blog",
                Route::empty().nested(
                    Segment::content(comp(BlogList)).catch_all((comp(BlogPost), PostId {})),
                ),
            )
            .fixed("oranges", comp(Oranges))
            .fixed("apples", "/oranges")
    });

    render! {
        h1 { "Your app here" }
        ul {
            li { Link { target: "/", "home" } }
            li { Link { target: "/blog", "blog" } }
            li { Link { target: "/blog/tim", "tims' blog" } }
            li { Link { target: "/blog/bill", "bills' blog" } }
            li { Link { target: "/blog/james", "james amazing' blog" } }
            li { Link { target: "/apples", "go to apples" } }
        }
        Outlet { }
    }
}

fn Home(cx: Scope) -> Element {
    log::debug!("rendering home {:?}", cx.scope_id());
    render! { h1 { "Home" } }
}

fn BlogList(cx: Scope) -> Element {
    log::debug!("rendering blog list {:?}", cx.scope_id());
    render! { div { "Blog List" } }
}

struct PostId;
fn BlogPost(cx: Scope) -> Element {
    let Some(id) = use_route(cx)?.parameter::<PostId>() else {
        return render!(div { "No blog post id" });
    };

    log::debug!("rendering blog post {}", id);

    render! {
        div {
            h3 { "blog post: {id:?}"  }
            Link { target: "/blog/", "back to blog list" }
        }
    }
}

fn Oranges(cx: Scope) -> Element {
    render!("Oranges are not apples!")
}
