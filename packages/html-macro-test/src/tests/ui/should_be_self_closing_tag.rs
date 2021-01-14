#![feature(proc_macro_hygiene)]

extern crate virtual_dom_rs;
use virtual_dom_rs::prelude::*;

// We are using open and close tags for a tag that should
// actually be a self closing tag
fn main () {
    html! {
        <br></br>
    };
}