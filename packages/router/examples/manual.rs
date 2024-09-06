use std::num::ParseIntError;

use dioxus::prelude::*;

fn main() {
    dioxus::launch(|| {
        rsx! {
            Router::<Route> {}
        }
    });
}

#[derive(Clone, PartialEq, Debug)]
enum Route {
    Home,
    About,
    Blog { id: usize },
}

impl Routable for Route {
    const SITE_MAP: &'static [SiteMapSegment] = &[];

    fn render(&self, _level: usize) -> Element {
        rsx! {
            div {
                nav {
                    Link { to: Route::Home, "Home" }
                    Link { to: Route::About, "About" }
                    Link { to: "/home", "About" }
                    Link { to: "https://example.com/about", "Other about" }
                }
                match self {
                    Route::Home => rsx! {
                        h1 { "Home" }
                    },
                    Route::About => rsx! {
                        h1 { "About" }
                    },
                    Route::Blog { id } => rsx! {
                        h1 { "Blog" }
                        p { "Id: {id}" }
                    },
                }
            }
        }
    }

    fn serialize(&self) -> String {
        match self {
            Route::Home => "/".to_string(),
            Route::About => "/about".to_string(),
            Route::Blog { id } => format!("/blog/{id}"),
        }
    }

    fn deserialize(route: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match route {
            "/" => Ok(Route::Home),
            "/about" => Ok(Route::About),
            blah if blah.starts_with("/blog/") => blah
                .strip_prefix("/blog/")
                .ok_or_else(|| format!("Failed to parse route: {}", route).into())
                .and_then(|s| s.parse().map_err(|e: ParseIntError| e.to_string().into()))
                .map(|id| Route::Blog { id }),
            _ => Err(format!("Failed to parse route: {}", route).into()),
        }
    }
}
