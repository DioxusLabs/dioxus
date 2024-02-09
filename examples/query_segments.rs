//! Example: Url query segments usage
//! ------------------------------------
//!
//! This example shows how to access and use multiple query segments present in an url on the web.
//!
//! Run `dx serve` and navigate to `http://localhost:8080/blog?name=John&surname=Doe`
use dioxus::prelude::*;
use std::fmt::Display;

#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    // segments that start with ?:.. are query segments that capture the entire query
    #[route("/blog?:..query_params")]
    BlogPost {
        // You must include query segments in child variants
        query_params: ManualBlogQuerySegments,
    },

    // segments that follow the ?:field&:other_field syntax are query segments that follow the standard url query syntax
    #[route("/autoblog?:name&:surname")]
    AutomaticBlogPost {
        name: String,
        surname: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct ManualBlogQuerySegments {
    name: String,
    surname: String,
}

/// The display impl needs to display the query in a way that can be parsed:
impl Display for ManualBlogQuerySegments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name={}&surname={}", self.name, self.surname)
    }
}

/// The query segment is anything that implements <https://docs.rs/dioxus-router/latest/dioxus::router/routable/trait.FromQuery.html>. You can implement that trait for a struct if you want to parse multiple query parameters.
impl FromQuery for ManualBlogQuerySegments {
    fn from_query(query: &str) -> Self {
        let mut name = None;
        let mut surname = None;
        let pairs = form_urlencoded::parse(query.as_bytes());
        pairs.for_each(|(key, value)| {
            if key == "name" {
                name = Some(value.clone().into());
            }
            if key == "surname" {
                surname = Some(value.clone().into());
            }
        });
        Self {
            name: name.unwrap(),
            surname: surname.unwrap(),
        }
    }
}

#[component]
fn BlogPost(query_params: ManualBlogQuerySegments) -> Element {
    rsx! {
        div { "This is your blogpost with a query segment:" }
        div { "{query_params:?}" }
    }
}

#[component]
fn AutomaticBlogPost(name: String, surname: String) -> Element {
    rsx! {
        div { "This is your blogpost with a query segment:" }
        div { "name={name}&surname={surname}" }
    }
}

#[component]
fn App() -> Element {
    rsx! { Router::<Route> {} }
}

fn main() {
    launch(App);
}
