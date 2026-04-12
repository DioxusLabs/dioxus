#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::test_attr_in_doctest)] // The doctests need to show examples of tests
//! A testing crate for Dioxus.
//!
//! This crate facilitates rendering, interacting with, and querying the DOM in tests of Dioxus
//! apps. Tests have fairlz precise control over both the rendering lifecycle and asynchronous
//! operations. Thus they can assert both on the final outcome of interactions, such as the
//! rendered data obtained from a call to a backend, as well as intermediate states, such as the
//! presence of a spinner while loading data.
//!
//! This uses the dioxus-native crate to manage the DOM, which in turn uses
//! [Blitz](https://crates.io/crates/blitz) for layout. It does not depend on a browser or any
//! other external process.
//!
//! Tests operate "headless", so they cannot render their state to the screen.
//!
//! ## Usage
//!
//! Tests can construct a [DocumentTester] instance to render and interact with the DOM. To
//! construct a [DocumentTester], the test can invoke the [render] function on a Dioxus component.
//! They must invoke the `build` to trigger the initial layout. The tester provides methods for
//! querying elements by CSS selector or by test ID.
//!
//! ```
//! use dioxus::prelude::*;
//! use dioxus_test::render;
//!
//! #[component]
//! fn MyComponent() -> Element {
//!     rsx! {
//!         div {
//!              class: "test-component",
//!              "Hello, world!"
//!         }
//!     }
//! }
//!
//! #[test]
//! fn my_component_renders_correctly() {
//!     let tester = render(MyComponent).build();
//!     assert_eq!(
//!         tester.find_by_css_selector(".test-component").unwrap().inner_html(),
//!         "Hello, world!"
//!     );
//! }
//! ```
//!
//! [DocumentTester] also provides methods for interacting with elements and driving the runtime.
//! After interacting with an element, the test must call [DocumentTester::pump] to cause the event
//! handler to be invoked.
//!
//! ```
//! use dioxus::prelude::*;
//! use dioxus_test::render;
//!
//! #[component]
//! fn MyComponent() -> Element {
//!     let mut text = use_signal(|| "Click me!");
//!     rsx! {
//!         button {
//!              class: "test-button",
//!              onclick: move |_| {
//!                  *text.write() = "Don't click any more!";
//!              },
//!              {text}
//!         }
//!     }
//! }
//!
//! #[tokio::test]
//! async fn my_component_changes_button_text_on_click() {
//!     let mut tester = render(MyComponent).build();
//!     tester.find_first_by_css_selector(".test-button").unwrap().click();
//!     tester.pump().await;
//!     assert_eq!(
//!         tester.find_first_by_css_selector(".test-button").unwrap().inner_html(),
//!         "Don't click any more!"
//!     );
//! }
//! ```
//!
//! ## Asynchronous operations
//!
//! The method [DocumentTester::pump] returns control to the async runtime and thus drives any
//! asynchronous operations such as requests to the backend. If any rendering depends on the result
//! of a request, then the test must invoke [DocumentTester::pump] to resolve that.
//!
//! The test can also assert on the state of the DOM while backend requests are in flight.
//!
//! ## Limitations
//!
//! Interactions with the DOM operate directly on elements, not on the screen. So if, say, the test
//! dispatches a click on an element which is covered by a frost, the element will respond as though
//! it were reachable even though it would not be in reality.
//!
//! The layout system is limited by what the Blitz layout system can support. Since Blitz is not
//! complete as of the time of writing, computed layouts will often not be as in reality.

mod condition;
mod document;
mod element;
mod matcher;
mod result;

pub use condition::{AllElementsCondition, ElementCondition, ImmediateCondition};
pub use document::{DocumentTester, by_testid, render};
pub use matcher::{Matcher, contains_string, empty, inner_html, not};
pub use result::{Result, TesterError};
