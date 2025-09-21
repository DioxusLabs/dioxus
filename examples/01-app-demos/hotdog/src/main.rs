mod backend;
mod frontend;

use dioxus::prelude::*;
use frontend::*;

#[derive(Routable, PartialEq, Clone)]
enum Route {
    #[layout(NavBar)]
    #[route("/")]
    DogView,

    #[route("/favorites")]
    Favorites,
}

fn main() {
    #[cfg(not(feature = "server"))]
    dioxus::fullstack::set_server_url("https://hot-dog.fly.dev");

    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        Stylesheet { href: asset!("/assets/main.css") }
        Router::<Route> {}
    }
}
