use crate::{prefixed_route::PrefixedRoute, ui::pane::HorizontalPane, Route, CONTEXT};
use dioxus::prelude::*;
use dioxus_material::{use_theme, IconFont};
use dioxus_router::prelude::*;

/// The main application wrap component.
#[component]
pub fn Wrap() -> Element {
    let mut query = use_signal(String::new);
    let elements = use_memo(move || {
        CONTEXT
            .try_with(|cx| {
                cx.borrow()
                    .iter()
                    .filter(|(name, _)| name.to_lowercase().contains(&query.read().to_lowercase()))
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap()
    });

    let navigator = use_navigator();
    let theme = use_theme();

    let left = rsx!(
        div {
            flex: 1,
            display: "flex",
            flex_direction: "column",
            gap: "10px",
            width: "200px",
            margin: 0,
            padding: "10px 20px",
            font_size: "14px",
            background: "#eeeeee",
            div {
                display: "flex",
                flex_direction: "row",
                align_items: "center",
                justify_content: "flex-start",
                margin: "20px 0",
                img { src: "https://avatars.githubusercontent.com/u/79236386?s=200&v=4", width: 50}
                h1 {
                    cursor: "pointer",
                    margin: "0",
                    onclick: move |_| {
                        navigator.push(Route::Home);
                    },
                    "Dioxus"
                }
            }
            input {
                placeholder: "Search",
                value: "{query}",
                width: "100%",
                margin: "5px",
                margin_bottom: "20px",
                padding: "10px",
                border: "2px solid #999",
                border_radius: &*theme.border_radius_small,
                outline: "none",
                background: "none",
                font_size: 14.,
                oninput: move |event: FormEvent| query.set(event.value().clone())
            }
            { elements().into_iter().map(move | (name, _) | { rsx!(NavItem { route :
            Route::ComponentScreen { name : name.to_string(), }, label : "{name}" }) }) }
        }
    );

    let right = rsx!(Outlet::<PrefixedRoute> {});

    rsx! {
        IconFont {}
        div {
            position: "absolute",
            top: 0,
            left: 0,
            width: "100vw",
            height: "100vh",
            display: "flex",
            flex_direction: "row",
            font_family: "sans-serif",
            margin: 0,
            padding: 0,
            HorizontalPane { left: left, right: right }
        }
    }
}

/// Navigation rail item component.
#[component]
fn NavItem(route: Route, label: String) -> Element {
    let navigator = use_navigator();
    let current_route: PrefixedRoute = use_route();
    let theme = use_theme();

    let prefixed_route = PrefixedRoute(route.clone());
    let is_selected = current_route == prefixed_route;

    rsx!(
        div {
            padding: "10px 15px",
            border_radius: &*theme.border_radius_small,
            cursor: "pointer",
            color: if is_selected { "#fff" } else { "#444" },
            background: if is_selected { &theme.primary_color } else { "" },
            onclick: move |_| {
                navigator.push(PrefixedRoute(route.clone()));
            },
            "{label}"
        }
    )
}
