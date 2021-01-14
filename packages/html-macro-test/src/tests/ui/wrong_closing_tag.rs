#![feature(proc_macro_hygiene)]

extern crate virtual_dom_rs;
use virtual_dom_rs::prelude::*;

// Expected a closing div tag, found a closing strong tag
fn main () {
    html! {
        <div> </strong>
    };
}