// #![feature(proc_macro_hygiene)]

extern crate wasm_bindgen_test;
extern crate web_sys;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen_test::*;

use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::*;

use virtual_dom_rs::prelude::*;

wasm_bindgen_test_configure!(run_in_browser);

// Make sure that we successfully attach an event listener and see it work.
#[wasm_bindgen_test]
fn on_input() {
    let text = Rc::new(RefCell::new("Start Text".to_string()));
    let text_clone = Rc::clone(&text);

    let input = html! {
        <input
            // On input we'll set our Rc<RefCell<String>> value to the input elements value
            oninput=move |event: Event| {
                let input_elem = event.target().unwrap();
                let input_elem = input_elem.dyn_into::<HtmlInputElement>().unwrap();
                *text_clone.borrow_mut() = input_elem.value();
            }
            value="End Text"
        >
    };

    let input_event = InputEvent::new("input").unwrap();
    let input = input.create_dom_node().node;

    assert_eq!(&*text.borrow(), "Start Text");

    // After dispatching the oninput event our `text` should have a value of the input elements value.
    web_sys::EventTarget::from(input)
        .dispatch_event(&input_event)
        .unwrap();

    assert_eq!(&*text.borrow(), "End Text");
}
