#![allow(unused)]

use dioxus::prelude::*;
use dioxus_stores::*;

#[derive(Store)]
struct TodoItem {
    checked: bool,
    contents: String,
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
            item: item.contents().deref(),
        }
        TakesStrStore {
            item: item.contents().deref(),
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
