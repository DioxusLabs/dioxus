use std::rc::Rc;

use dioxus::html::geometry::PixelsVector2D;
use dioxus::prelude::*;

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(Scroll::default()));

    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Blog(id: i32) -> Element {
    rsx! {
        GoBackButton { "Go back" }
        div { "Blog post {id}" }
    }
}

type Scroll = Option<PixelsVector2D>;

#[component]
fn Home() -> Element {
    let mut element: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut scroll = use_context::<Signal<Scroll>>();

    _ = use_resource(move || async move {
        if let (Some(element), Some(scroll)) = (element.read().as_ref(), *scroll.peek()) {
            element
                .scroll(scroll, ScrollBehavior::Instant)
                .await
                .unwrap();
        }
    });

    rsx! {
        div {
            height: "300px",
            overflow_y: "auto",
            border: "1px solid black",

            onmounted: move |event| element.set(Some(event.data())),

            onscroll: move |_| async move {
                if let Some(element) = element.cloned() {
                    scroll.set(Some(element.get_scroll_offset().await.unwrap()))
                }
            },

            for i in 0..100 {
                div { height: "20px",

                    Link { to: Route::Blog { id: i }, "Blog {i}" }
                }
            }
        }
    }
}
