# Reactivity

The core of dioxus relies on the concept of reactivity. Reactivity is the system that updates your app when state changes.

There are two parts to reactivity: **Reactive Contexts** and **Tracked Values**.

## Reactive Contexts

Reactive Contexts keep track of what state different parts of your app rely on. Reactive Context show up throughout dioxus: Component, use_effect, use_memo and use_resource all have their own reactive contexts:

```rust, no_run
# use dioxus::prelude::*;
let count = use_signal(|| 0);
// The reactive context in the memo knows that the memo depends on the count signal
use_memo(move || count() * 2);
```

## Tracked Values

Tracked values are values that reactive contexts know about. When you read a tracked value, the reactive context will rerun when the value changes. Signals, Memos, and Resources are all tracked values:

```rust, no_run
# use dioxus::prelude::*;
// The count signal is tracked
let count = use_signal(|| 0);
// When you read the count signal, the reactive context subscribes to the count signal
let double_count = use_memo(move || count() * 2);
```

## Reactivity

Reactivity is the system that combines reactive context and tracked values to update your app when state changes.

You can use reactivity to create derived state and update your app when state changes.

You can derive state from other state with [`use_memo`](https://docs.rs/dioxus/latest/dioxus/prelude/fn.use_memo.html).

```rust, no_run
use dioxus::prelude::*;

let mut count = use_signal(|| 0);
let double_count = use_memo(move || count() * 2);

// Now whenever we read double_count, we know it is always twice the value of count
println!("{}", double_count); // Prints "2"

// After we write to count, the reactive context will rerun and double_count will be updated automatically
count += 1;

println!("{}", double_count); // Prints "4"
```

You can also use reactivity to create derive state asynchronously. For example, you can use [`use_resource`](https://docs.rs/dioxus/latest/dioxus/prelude/fn.use_resource.html) to load data from a server:

```rust, no_run
use dioxus::prelude::*;

let count = use_signal(|| 0);
let double_count = use_resource(move || async move {
    // Start a request to the server. We are reading the value of count to format it into the url
    // Since we are reading count, this resource will "subscribe" to changes to count (when count changes, the resource will rerun)
    let response = reqwest::get(format!("https://myserver.com/doubleme?count={count}")).await.unwrap();
    response.text().await.unwrap()
});
```

## Non Reactive State

You can use plain Rust types in Dioxus, but you should be aware that they are not reactive. If you read the non-reactive state, reactive scopes will not subscribe to the state.

You can make non-reactive state reactive by using the `Signal` type instead of a plain Rust type or by using the `use_reactive` hook.

```rust, no_run
use dioxus::prelude::*;

// ❌ Don't create non-reactive state
let state = use_hook(|| std::cell::RefCell::new(0));

// Computed values will get out of date if the state they depend on is not reactive
let doubled = use_memo(move || *state.borrow() * 2);

// ✅ Create reactive state
let state = use_signal(|| 0);

// Computed values will automatically keep up to date with the latest reactive state
let doubled = use_memo(move || state() * 2);

// ❌ Don't depend on non-reactive prop state in memos/resources
#[component]
fn MyComponent(state: i32) -> Element {
    let doubled = use_memo(move || state * 2);
    todo!()
}

// ✅ Wrap your props in ReadOnlySignal to make them reactive
#[component]
fn MyReactiveComponent(state: ReadOnlySignal<i32>) -> Element {
    let doubled = use_memo(move || state() * 2);
    todo!()
}
```

If your state can't be reactive, you can use the `use_reactive` hook to make it reactive.

```rust, no_run
use dioxus::prelude::*;

let state = rand::random::<i32>();

// You can make the state reactive by wrapping it in use_reactive
let doubled = use_memo(use_reactive!(|state| state * 2));
```
