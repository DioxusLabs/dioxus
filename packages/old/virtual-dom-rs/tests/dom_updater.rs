//! Ensure that our DomUpdater maintains Rc's to closures so that they work even
//! after dropping virtual dom nodes.
//!
//! To run all tests in this file:
//!
//! wasm-pack test crates/virtual-dom-rs --chrome --headless -- --test dom_updater

// #![feature(proc_macro_hygiene)]

use console_error_panic_hook;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use virtual_dom_rs::prelude::*;
use virtual_dom_rs::DomUpdater;
use wasm_bindgen::JsCast;
use wasm_bindgen_test;
use wasm_bindgen_test::*;
use web_sys::*;

wasm_bindgen_test_configure!(run_in_browser);

// Verify that our DomUpdater's patch method works.
// We test a simple case here, since diff_patch.rs is responsible for testing more complex
// diffing and patching.
#[wasm_bindgen_test]
fn patches_dom() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();

    let vdom = html! { <div></div> };

    let mut dom_updater = DomUpdater::new(vdom);

    let new_vdom = html! { <div id="patched"></div> };
    dom_updater.update(new_vdom);

    document
        .body()
        .unwrap()
        .append_child(&dom_updater.root_node());
    assert_eq!(document.query_selector("#patched").unwrap().is_some(), true);
}

// When you replace a DOM node with another DOM node we need to make sure that the closures
// from the new DOM node are stored by the DomUpdater otherwise they'll get dropped and
// won't work.
#[wasm_bindgen_test]
fn updates_active_closure_on_replace() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    let old = html! { <div> </div> };
    let mut dom_updater = DomUpdater::new_append_to_mount(old, &body);

    let text = Rc::new(RefCell::new("Start Text".to_string()));
    let text_clone = Rc::clone(&text);

    let id = "update-active-closures-on-replace";

    {
        let replace_node = html! {
         <input
            id=id
            oninput=move |event: Event| {
               let input_elem = event.target().unwrap();
               let input_elem = input_elem.dyn_into::<HtmlInputElement>().unwrap();
               *text_clone.borrow_mut() = input_elem.value();
            }
            value="End Text"
         >
        };

        // New node replaces old node.
        // We are testing that we've stored this new node's closures even though `new` will be dropped
        // at the end of this block.
        dom_updater.update(replace_node);
    }

    let input_event = InputEvent::new("input").unwrap();

    assert_eq!(&*text.borrow(), "Start Text");

    // After dispatching the oninput event our `text` should have a value of the input elements value.
    let input = document.get_element_by_id(&id).unwrap();
    web_sys::EventTarget::from(input)
        .dispatch_event(&input_event)
        .unwrap();

    assert_eq!(&*text.borrow(), "End Text");
}

// When you replace a DOM node with another DOM node we need to make sure that the closures
// from the new DOM node are stored by the DomUpdater otherwise they'll get dropped and
// won't work.
#[wasm_bindgen_test]
fn updates_active_closures_on_append() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    let old = html! { <div> </div> };
    let mut dom_updater = DomUpdater::new_append_to_mount(old, &body);

    let text = Rc::new(RefCell::new("Start Text".to_string()));
    let text_clone = Rc::clone(&text);

    let id = "update-active-closures-on-append";

    {
        let append_node = html! {
        <div>
           <input
              id=id
              oninput=move |event: Event| {
                 let input_elem = event.target().unwrap();
                 let input_elem = input_elem.dyn_into::<HtmlInputElement>().unwrap();
                 *text_clone.borrow_mut() = input_elem.value();
              }
              value="End Text"
           >
         </div>
        };

        // New node gets appended into the DOM.
        // We are testing that we've stored this new node's closures even though `new` will be dropped
        // at the end of this block.
        dom_updater.update(append_node);
    }

    let input_event = InputEvent::new("input").unwrap();

    assert_eq!(&*text.borrow(), "Start Text");

    // After dispatching the oninput event our `text` should have a value of the input elements value.
    let input = document.get_element_by_id(id).unwrap();
    web_sys::EventTarget::from(input)
        .dispatch_event(&input_event)
        .unwrap();

    assert_eq!(&*text.borrow(), "End Text");
}
