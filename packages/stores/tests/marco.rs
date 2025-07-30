use dioxus_stores::*;
use std::collections::HashMap;

#[derive(Store, PartialEq, Clone, Debug)]
struct TodoState {
    todos: HashMap<u32, TodoItem>,
    filter: FilterState,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum FilterState {
    All,
    Active,
    Completed,
}

#[derive(Store, PartialEq, Clone, Debug)]
struct TodoItem {
    checked: bool,
    contents: String,
}

fn access_todos(store: Store<TodoItem>) {
    let checked = store.checked();
    let contents = store.contents();
    let checked: bool = checked();
    let contents: String = contents();

    let transposed = store.transpose();
    let checked = transposed.checked;
    let contents = transposed.contents;
    let checked: bool = checked();
    let contents: String = contents();
}

#[derive(Store, PartialEq, Clone, Debug)]
struct Item(bool, String);

fn access_todos_tuple(store: Store<Item>) {
    let first = store.field_0();
    let first: bool = first();

    let transposed = store.transpose();
    let first = transposed.0;
    let second = transposed.1;
    let first: bool = first();
    let second: String = second();
}
