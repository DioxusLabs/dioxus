# Generational Box

Generational Box is a runtime for Rust that allows any static type to implement `Copy`. It can be combined with a global runtime to create an ergonomic state solution like `dioxus-signals`. This crate contains no `unsafe` code.

Three main types manage state in Generational Box:

- Store: Handles recycling generational boxes that have been dropped. Your application should have one store or one store per thread.
- Owner: Handles dropping generational boxes. The owner acts like a runtime lifetime guard. Any states that you create with an owner will be dropped when that owner is dropped.
- GenerationalBox: The core Copy state type. The generational box will be dropped when the owner is dropped.

Example:

```rust
use generational_box::{UnsyncStorage, AnyStorage};

// Create an owner for some state for a scope
let owner = UnsyncStorage::owner();

// Create some non-copy data, move it into a owner, and work with copy data
let data: String = "hello world".to_string();
let key = owner.insert(data);

// The generational box can be read from and written to like a RefCell
let value = key.read();
assert_eq!(*value, "hello world");
```

## How it works

Internally, `generational-box` creates an arena of generational `RefCell`'s that are recycled when the owner is dropped. You can think of the cells as something like `&'static RefCell<Box<dyn Any>>` with a generational check to make recycling a cell easier to debug. Then GenerationalBox's are `Copy` because the `&'static` pointer is `Copy`
