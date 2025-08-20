use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn NavBar() -> Element {
    rsx! {
        div { id: "title",
            span {}

            Link { to: Route::DogView,
                h1 { "🌭 HotDog! " }
            }

            Link { to: Route::Favorites, id: "heart", "♥️" }
        }

        Outlet::<Route> {}
    }
}
