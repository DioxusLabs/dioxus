//! Tests that ensure that we create the right DOM element from a VirtualNode
//!
//! To run all tests in this file:
//!
//! wasm-pack test crates/virtual-dom-rs --chrome --headless -- --test create_element

// #![feature(proc_macro_hygiene)]

extern crate wasm_bindgen_test;
extern crate web_sys;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Element, Event, EventTarget, MouseEvent};

use virtual_dom_rs::prelude::*;

wasm_bindgen_test_configure!(run_in_browser);

/// wasm-pack test crates/virtual-dom-rs --chrome --headless -- --test create_element nested_divs
#[wasm_bindgen_test]
fn nested_divs() {
    let vdiv = html! { <div> <div> <div></div> </div> </div> };
    let div: Element = vdiv.create_dom_node().node.unchecked_into();

    assert_eq!(&div.inner_html(), "<div><div></div></div>");
}

/// wasm-pack test crates/virtual-dom-rs --chrome --headless -- --test create_element svg_element
/// TODO: Temporarily disabled until we figure out why it's failing in CI but not failing locally
// #[wasm_bindgen_test]
// fn svg_element() {
//     let vdiv = html! { <div><svg xmlns="http://www.w3.org/2000/svg">
//       <circle cx="50" cy="50" r="50"/>
//     </svg></div> };
//     let div: Element = vdiv.create_dom_node().node.unchecked_into();

//     assert_eq!(
//         &div.inner_html(),
//         r#"<svg xmlns="http://www.w3.org/2000/svg"><circle cx="50" cy="50" r="50"></circle></svg>"#
//     );
// }

/// wasm-pack test crates/virtual-dom-rs --chrome --headless -- --test create_element div_with_attributes
#[wasm_bindgen_test]
fn div_with_attributes() {
    let vdiv = html! { <div id="id-here" class="two classes"></div> };
    let div: Element = vdiv.create_dom_node().node.unchecked_into();

    assert_eq!(&div.id(), "id-here");

    assert!(div.class_list().contains("two"));
    assert!(div.class_list().contains("classes"));

    assert_eq!(div.class_list().length(), 2);
}

/// wasm-pack test crates/virtual-dom-rs --chrome --headless -- --test create_element click_event
#[wasm_bindgen_test]
fn click_event() {
    let clicked = Rc::new(Cell::new(false));
    let clicked_clone = Rc::clone(&clicked);

    let div = html! {
     <div
         onclick=move |_ev: MouseEvent| {
             clicked_clone.set(true);
         }
     >
     </div>
    };

    let click_event = Event::new("click").unwrap();

    let div = div.create_dom_node().node;

    (EventTarget::from(div))
        .dispatch_event(&click_event)
        .unwrap();

    assert_eq!(*clicked, Cell::new(true));
}

/// wasm-pack test crates/virtual-dom-rs --chrome --headless -- --test create_element inner_html
/// @book start inner-html
#[wasm_bindgen_test]
fn inner_html() {
    let div = html! {
    <div
      unsafe_inner_html="<span>hi</span>"
    >
    </div>
    };
    let div: Element = div.create_dom_node().node.unchecked_into();

    assert_eq!(div.inner_html(), "<span>hi</span>");
}
// @book end inner-html

/// wasm-pack test crates/virtual-dom-rs --chrome --headless -- --test create_element on_create_elem
/// @book start on-create-elem
#[wasm_bindgen_test]
fn on_create_elem() {
    let div = html! {
    <div
      on_create_elem=|elem: web_sys::Element| {
        elem.set_inner_html("Hello world");
      }
    >
        <span>This span should get replaced</span>
    </div>
    };
    let div: Element = div.create_dom_node().node.unchecked_into();

    assert_eq!(div.inner_html(), "Hello world");
}
// @book end on-create-elem
