#![feature(proc_macro_hygiene)]

extern crate virtual_dom_rs;
use virtual_dom_rs::prelude::*;

// Used a tag name that does not exist in the HTML spec
fn main() {
    html! {
        <invalidtagname></invalidtagname>
    };
}
