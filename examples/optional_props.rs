//! Optional props
//!
//! This example demonstrates how to use optional props in your components. The `Button` component has several props,
//! and we use a variety of attributes to set them.

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        // We can set some of the props, and the rest will be filled with their default values
        // By default `c` can take a `None` value, but `d` is required to wrap a `Some` value
        Button {
            a: "asd".to_string(),
            // b can be omitted, and it will be filled with its default value
            c: "asd".to_string(),
            d: Some("asd".to_string()),
            e: Some("asd".to_string()),
        }

        Button {
            a: "asd".to_string(),
            b: "asd".to_string(),

            // We can omit the `Some` on `c` since Dioxus automatically transforms Option<T> into optional
            c: "asd".to_string(),
            d: Some("asd".to_string()),
            e: "asd".to_string(),
        }

        // `b` and `e` are ommitted
        Button {
            a: "asd".to_string(),
            c: "asd".to_string(),
            d: Some("asd".to_string()),
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct ButtonProps {
    a: String,

    #[props(default)]
    b: String,

    c: Option<String>,

    #[props(!optional)]
    d: Option<String>,

    #[props(optional)]
    e: SthElse<String>,
}

type SthElse<T> = Option<T>;

#[allow(non_snake_case)]
fn Button(props: ButtonProps) -> Element {
    rsx! {
        button {
            "{props.a} | "
            "{props.b:?} | "
            "{props.c:?} | "
            "{props.d:?} | "
            "{props.e:?}"
        }
    }
}
