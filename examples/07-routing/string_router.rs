use dioxus::{
    prelude::*,
    router::components::{EmptyRoutable, Route},
};

fn main() {
    dioxus::launch(|| {
        rsx! {
            Router::<EmptyRoutable> {
                Route { to: "/" , h1 { "Home" } }
                Route { to: "/about" , h1 { "About" } }
            }
        }
    });
}
