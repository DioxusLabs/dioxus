#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

#[rustfmt::skip]
fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        // ANCHOR: OptionalProps_usage
Title {
    title: "Some Title",
},
Title {
    title: "Some Title",
    subtitle: "Some Subtitle",
},
// Providing an Option explicitly won't compile though:
// Title {
//     title: "Some Title",
//     subtitle: None,
// },
        // ANCHOR_END: OptionalProps_usage

        // ANCHOR: ExplicitOption_usage
ExplicitOption {
    title: "Some Title",
    subtitle: None,
},
ExplicitOption {
    title: "Some Title",
    subtitle: Some("Some Title"),
},
// This won't compile:
// ExplicitOption {
//     title: "Some Title",
// },
        // ANCHOR_END: ExplicitOption_usage

        // ANCHOR: DefaultComponent_usage
DefaultComponent {
    number: 5,
},
DefaultComponent {},
        // ANCHOR_END: DefaultComponent_usage

        // ANCHOR: IntoComponent_usage
IntoComponent {
    string: "some &str",
},
        // ANCHOR_END: IntoComponent_usage
    })
}

// ANCHOR: OptionalProps
#[derive(Props)]
struct OptionalProps<'a> {
    title: &'a str,
    subtitle: Option<&'a str>,
}

fn Title<'a>(cx: Scope<'a, OptionalProps>) -> Element<'a> {
    cx.render(rsx!(h1{
        "{cx.props.title}: ",
        cx.props.subtitle.unwrap_or("No subtitle provided"),
    }))
}
// ANCHOR_END: OptionalProps

// ANCHOR: ExplicitOption
#[derive(Props)]
struct ExplicitOptionProps<'a> {
    title: &'a str,
    #[props(!optional)]
    subtitle: Option<&'a str>,
}

fn ExplicitOption<'a>(cx: Scope<'a, ExplicitOptionProps>) -> Element<'a> {
    cx.render(rsx!(h1 {
        "{cx.props.title}: ",
        cx.props.subtitle.unwrap_or("No subtitle provided"),
    }))
}
// ANCHOR_END: ExplicitOption

// ANCHOR: DefaultComponent
#[derive(PartialEq, Props)]
struct DefaultProps {
    // default to 42 when not provided
    #[props(default = 42)]
    number: i64,
}

fn DefaultComponent(cx: Scope<DefaultProps>) -> Element {
    cx.render(rsx!(h1 { "{cx.props.number}" }))
}
// ANCHOR_END: DefaultComponent

// ANCHOR: IntoComponent
#[derive(PartialEq, Props)]
struct IntoProps {
    #[props(into)]
    string: String,
}

fn IntoComponent(cx: Scope<IntoProps>) -> Element {
    cx.render(rsx!(h1 { "{cx.props.string}" }))
}
// ANCHOR_END: IntoComponent
