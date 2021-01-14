//! Ensure that our DomUpdater maintains Rc's to closures so that they work even
//! after dropping virtual dom nodes.
//!
//! To run all tests in this file:
//!
//! wasm-pack test crates/virtual-dom-rs --chrome --headless -- --test closures

// #![feature(proc_macro_hygiene)]

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::rc::Rc;
use virtual_dom_rs::prelude::*;
use virtual_dom_rs::DomUpdater;
use wasm_bindgen::JsCast;
use wasm_bindgen_test;
use wasm_bindgen_test::*;
use web_sys::*;

wasm_bindgen_test_configure!(run_in_browser);

// TODO: This test current fails in headless browsers but works in non headless browsers
// (tested in both geckodriver and chromedriver)
// Need to figure out why
#[wasm_bindgen_test]
fn closure_not_dropped() {
    let text = Rc::new(RefCell::new("Start Text".to_string()));

    let document = web_sys::window().unwrap().document().unwrap();

    let mut dom_updater = None;

    {
        let mut input = make_input_component(Rc::clone(&text));
        input
            .as_velement_mut()
            .expect("Not an element")
            .attrs
            .insert("id".into(), "old-input-elem".into());

        let mount = document.create_element("div").unwrap();
        mount.set_id("mount");
        document.body().unwrap().append_child(&mount).unwrap();

        dom_updater = Some(DomUpdater::new_replace_mount(input, mount));

        let mut dom_updater = dom_updater.as_mut().unwrap();

        // Input VirtualNode from above gets dropped at the end of this block,
        // yet that element held Rc's to the Closure's that power the oninput event.
        //
        // We're patching the DOM with a new vdom, but since our new vdom doesn't contain any
        // new elements, `.create_element` won't get called and so no new Closures will be
        // created.
        //
        // So, we're testing that our old Closure's still work. The reason that they work is
        // that dom_updater maintains Rc's to those Closures.
        let mut new_node = make_input_component(Rc::clone(&text));
        new_node
            .as_velement_mut()
            .expect("Not an element")
            .attrs
            .insert("id".into(), "new-input-elem".into());

        dom_updater.update(new_node);
    }

    let dom_updater = dom_updater.as_ref().unwrap();

    let input: HtmlInputElement = document
        .get_element_by_id("new-input-elem")
        .expect("Input element")
        .dyn_into()
        .unwrap();
    let input_event = InputEvent::new("input").unwrap();

    assert_eq!(&*text.borrow(), "Start Text");

    // After dispatching the oninput event our `text` should have a value of the input elements value.
    web_sys::EventTarget::from(input)
        .dispatch_event(&input_event)
        .unwrap();

    assert_eq!(&*text.borrow(), "End Text");

    assert_eq!(
        dom_updater.active_closures.get(&1).as_ref().unwrap().len(),
        1
    );
}

// We're just making sure that things compile - other tests give us confidence that the closure
// will work just fine.
//
// https://github.com/chinedufn/percy/issues/81
//
//#[wasm_bindgen_test]
//fn closure_with_no_params_compiles() {
//    let _making_sure_this_works = html! {
//        <div onclick=|| {}></div>
//    };
//}

fn make_input_component(text_clone: Rc<RefCell<String>>) -> VirtualNode {
    html! {
        <input
           // On input we'll set our Rc<RefCell<String>> value to the input elements value
           oninput=move |event: Event| {
              let input_elem = event.target().unwrap();
              let input_elem = input_elem.dyn_into::<HtmlInputElement>().unwrap();
              *text_clone.borrow_mut() = input_elem.value();
           }
           value="End Text"
        >
    }
}
