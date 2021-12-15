use dioxus::prelude::*;
use dioxus::router::Router;

#[derive(PartialEq, Debug, Clone)]
pub enum Route {
    // #[at("/")]
    Home,

    // #[at("/:id")]
    AllUsers { page: u32 },

    // #[at("/:id")]
    User { id: u32 },

    // #[at("/:id")]
    BlogList { page: u32 },

    // #[at("/:id")]
    BlogPost { post_id: u32 },

    // #[at("/404")]
    // #[not_found]
    NotFound,
}

static App: Component<()> = |cx| {
    let route = use_router(cx, Route::parse)?;

    cx.render(rsx! {
        nav {
            Link { to: Route::Home, "Go home!" }
            Link { to: Route::AllUsers { page: 0 }, "List all users" }
            Link { to: Route::BlogList { page: 0 }, "Blog posts" }
        }
        {match route {
            Route::Home => rsx!("Home"),
            Route::AllUsers { page } => rsx!("All users - page {page}"),
            Route::User { id } => rsx!("User - id: {id}"),
            Route::BlogList { page } => rsx!("Blog posts - page {page}"),
            Route::BlogPost { post_id } => rsx!("Blog post - post {post_id}"),
            Route::NotFound => rsx!("Not found"),
        }}
        footer {}
    })
};

impl Route {
    // Generate the appropriate route from the "tail" end of the URL
    fn parse(url: &str) -> Self {
        use Route::*;
        match url {
            "/" => Home,
            "/users" => AllUsers { page: 1 },
            "/users/:page" => AllUsers { page: 1 },
            "/users/:page/:id" => User { id: 1 },
            "/blog" => BlogList { page: 1 },
            "/blog/:page" => BlogList { page: 1 },
            "/blog/:page/:id" => BlogPost { post_id: 1 },
            _ => NotFound,
        }
    }
}

impl Routable for Route {
    fn from_path(path: &str, params: &std::collections::HashMap<&str, &str>) -> Option<Self> {
        todo!()
    }

    fn to_path(&self) -> String {
        todo!()
    }

    fn routes() -> Vec<&'static str> {
        todo!()
    }

    fn not_found_route() -> Option<Self> {
        todo!()
    }

    fn recognize(pathname: &str) -> Option<Self> {
        todo!()
    }
}

fn main() {}
