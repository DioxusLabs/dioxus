use dioxus_core as dioxus;
use dioxus_core::prelude::*;

#[derive(Props)]
struct MyProps<'a> {
    blah: u128,
    b: &'a (),
}

fn main() {
    // let p = unsafe { MyProps {}.memoize(&MyProps {}) };
    // dbg!(p);
}
