use dioxus_core as dioxus;
use dioxus_core_macro::*;

fn main() {}

#[derive(Props)]
struct ChildProps<'a> {
    name: &'a str,
}
