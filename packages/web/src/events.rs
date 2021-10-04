//! Ported events into Dioxus Synthetic Event system

use dioxus_core::events::on::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, UiEvent};
