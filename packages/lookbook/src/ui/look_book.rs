use crate::{prefixed_route::use_prefix, register, PrefixedRoute, Preview, HOME};
use dioxus::prelude::*;
use dioxus_material::Theme;
use dioxus_router::prelude::Router;

#[component]
pub fn LookBook<I: IntoIterator<Item = Preview> + PartialEq + Clone + 'static>(
    previews: I,
    home: Component,
    #[props(default = None)] prefix: Option<&'static str>,
) -> Element {
    use_hook(move || {
        for preview in previews.clone() {
            register(preview.name, preview.component)
        }

        HOME.try_with(|cell| *cell.borrow_mut() = Some(home))
            .unwrap();
    });

    use_prefix(prefix);

    rsx! {
        Theme {
            primary_color: "rgb(59, 130, 246)",
            secondary_container_color: "rgb(233, 96, 32)",
            Router::<PrefixedRoute> {}
        }
    }
}
