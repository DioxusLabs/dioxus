//! Tests that ensure that diffing and patching work properly in a real browser.
//!
//! To run all tests in this file:
//!
//! wasm-pack test crates/virtual-dom-rs --chrome --headless -- --test diff_patch

// #![feature(proc_macro_hygiene)]

extern crate wasm_bindgen_test;
extern crate web_sys;
use wasm_bindgen_test::*;

use virtual_dom_rs::prelude::*;

wasm_bindgen_test_configure!(run_in_browser);

mod diff_patch_test_case;
use self::diff_patch_test_case::DiffPatchTest;

#[wasm_bindgen_test]
fn replace_child() {
    DiffPatchTest {
        desc: "Replace a root node attribute attribute and a child text node",
        old: html! {
         <div>
           Original element
         </div>
        },
        new: html! { <div> Patched element</div> },
        override_expected: None,
    }
    .test();
}

#[wasm_bindgen_test]
fn truncate_children() {
    DiffPatchTest {
        desc: "Truncates extra children",
        old: html! {
         <div>
           <div> <div> <b></b> <em></em> </div> </div>
         </div>
        },
        new: html! {
         <div>
           <div> <div> <b></b> </div> </div>
         </div>
        },
        override_expected: None,
    }
    .test();

    DiffPatchTest {
        desc: "https://github.com/chinedufn/percy/issues/48",
        old: html! {
         <div>
          ab <p></p> c
         </div>
        },
        new: html! {
         <div>
           ab <p></p>
         </div>
        },
        override_expected: None,
    }
    .test();
}

#[wasm_bindgen_test]
fn remove_attributes() {
    DiffPatchTest {
        desc: "Removes attributes",
        old: html! { <div style=""> </div>
        },
        new: html! { <div></div> },
        override_expected: None,
    }
    .test();
}

#[wasm_bindgen_test]
fn append_children() {
    DiffPatchTest {
        desc: "Append a child node",
        old: html! { <div> </div>
        },
        new: html! { <div> <span></span> </div> },
        override_expected: None,
    }
    .test();
}

#[wasm_bindgen_test]
fn text_node_siblings() {
    // NOTE: Since there are two text nodes next to eachother we expect a `<!--ptns-->` separator in
    // between them.
    // @see virtual_node/mod.rs -> create_dom_node() for more information
    // TODO: A little more spacing than there should be in between the text nodes ... but doesn't
    // impact the user experience so we can look into that later..
    let override_expected = Some(
        r#"<div id="after"><span> The button has been clicked:  <!--ptns--> world </span></div>"#,
    );

    let old1 = VirtualNode::text("The button has been clicked: ");
    let old2 = VirtualNode::text("hello");

    let new1 = VirtualNode::text("The button has been clicked: ");
    let new2 = VirtualNode::text("world");

    DiffPatchTest {
        desc: "Diff patch on text node siblings",
        old: html! {
        <div id="before">
            <span> { {old1} {old2} } </span>
        </div>
        },
        new: html! {
        <div id="after">
            <span> { {new1} {new2} } </span>
        </div>
        },
        override_expected,
    }
    .test();
}

#[wasm_bindgen_test]
fn append_text_node() {
    DiffPatchTest {
        desc: "Append text node",
        old: html! { <div> </div> },
        new: html! { <div> Hello </div> },
        override_expected: None,
    }
    .test();
}

#[wasm_bindgen_test]
fn append_sibling_text_nodes() {
    let text1 = VirtualNode::text("Hello");
    let text2 = VirtualNode::text("World");

    DiffPatchTest {
        desc: "Append sibling text nodes",
        old: html! { <div> </div> },
        new: html! { <div> {text1} {text2} </div> },
        override_expected: None,
    }
    .test();
}

#[wasm_bindgen_test]
fn replace_with_children() {
    DiffPatchTest {
        desc: "Replace node that has children",
        old: html! { <table><tr><th>0</th></tr><tr><td>1</td></tr></table> },
        new: html! { <table><tr><td>2</td></tr><tr><th>3</th></tr></table> },
        override_expected: None,
    }
    .test();
}

// https://github.com/chinedufn/percy/issues/62
#[wasm_bindgen_test]
fn replace_element_with_text_node() {
    DiffPatchTest {
        desc: "#62: Replace element with text node",
        old: html! { <span> <br> </span> },
        new: html! { <span> a </span> },
        override_expected: None,
    }
    .test();
}

// https://github.com/chinedufn/percy/issues/68
#[wasm_bindgen_test]
fn text_root_node() {
    DiffPatchTest {
        desc: "Patching of text root node works",
        old: html! { Old text },
        new: html! { New text },
        override_expected: None,
    }
    .test();
}

#[wasm_bindgen_test]
fn replace_text_with_element() {
    DiffPatchTest {
        desc: "Replacing a text node with an element works",
        old: html! { <div>a</div> },
        new: html! { <div><br></div> },
        override_expected: None,
    }
    .test();
}
