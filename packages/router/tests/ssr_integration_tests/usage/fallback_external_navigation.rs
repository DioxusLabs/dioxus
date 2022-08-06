use dioxus::prelude::*;
use dioxus_router::{history::MemoryHistory, prelude::*};

use crate::{render, test_routes};

#[test]
fn show_fallback_content() {
    let message = format!(
        "{title}{link}{link2}{link3}{p1}{p2}{p3}{button}",
        title = "<h1>Oops, you weren't meant to go here!</h1>",
        link = "<p><a href=\"https://dioxuslabs.com/\" dioxus-prevent-default=\"\" class=\"\" ",
        link2 = "id=\"\" rel=\"noopener noreferrer\" target=\"\">Click here to get back on track!",
        link3 = "</a></p>",
        p1 = "<p>The application you are using tried to send you to an external website, but it ",
        p2 = "<!--spacer-->couldn't. Click the link above to open the external website, or the ",
        p3 = "button below to <!--spacer-->go back to the previous page.</p>",
        button = "<button dioxus-prevent-default=\"onclick\">Click here to go back</button>"
    );
    assert_eq!(message, render(App));

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                history: &||MemoryHistory::with_first(String::from("/external-navigation-failure")),

                Outlet { }
            }
        })
    }
}
