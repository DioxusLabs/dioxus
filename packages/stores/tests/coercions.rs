#![allow(unused)]

use dioxus::prelude::*;
use dioxus_signals::MappedMutSignal;
use dioxus_stores::*;

#[derive(Store)]
struct TodoItem {
    checked: bool,
    contents: String,
}

fn string_as_str(s: &String) -> &str {
    s.as_str()
}

fn string_as_str_mut(s: &mut String) -> &mut str {
    s.as_mut_str()
}

fn app() -> Element {
    let item = use_store(|| TodoItem {
        checked: false,
        contents: "Learn about stores".to_string(),
    });

    rsx! {
        TakesReadSignal {
            item,
        }
        TakesReadStore {
            item,
        }
        TakesStr {
            item: MappedMutSignal::new(item.contents(), string_as_str, string_as_str_mut),
        }
        TakesStrStore {
            item: Store::from_lens(MappedMutSignal::new(item.contents(), string_as_str, string_as_str_mut)),
        }
    }
}

#[component]
fn TakesStr(item: ReadSignal<str>) -> Element {
    rsx! {
        TakesStr {
            item,
        }
    }
}

#[component]
fn TakesStrStore(item: ReadStore<str>) -> Element {
    rsx! {
        TakesStrStore {
            item,
        }
    }
}

#[component]
fn TakesReadSignal(item: ReadSignal<TodoItem>) -> Element {
    rsx! {
        TakesReadSignal {
            item,
        }
    }
}

#[component]
fn TakesReadStore(item: ReadStore<TodoItem>) -> Element {
    rsx! {
        TakesReadStore {
            item,
        }
    }
}
