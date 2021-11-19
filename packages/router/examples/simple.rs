use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_router::*;

fn main() {
    console_error_panic_hook::set_once();
    dioxus_web::launch(App, |c| c);
}

#[derive(Clone, Debug, PartialEq)]
enum Route {
    Home,
    About,
    NotFound,
}

static App: FC<()> = |cx, props| {
    let route = use_router(cx, Route::parse);

    match route {
        Route::Home => rsx!(cx, div { "Home" }),
        Route::About => rsx!(cx, div { "About" }),
        Route::NotFound => rsx!(cx, div { "NotFound" }),
    }
};

impl ToString for Route {
    fn to_string(&self) -> String {
        match self {
            Route::Home => "/".to_string(),
            Route::About => "/about".to_string(),
            Route::NotFound => "/404".to_string(),
        }
    }
}
impl Route {
    fn parse(s: &str) -> Self {
        match s {
            "/" => Route::Home,
            "/about" => Route::About,
            _ => Route::NotFound,
        }
    }
}
