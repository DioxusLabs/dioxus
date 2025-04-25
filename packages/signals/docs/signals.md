Signals are a Copy state management solution with automatic dependency tracking.

You may have noticed that this struct doesn't have many methods. Most methods for Signal are defined on the [`Readable`] and [`Writable`] traits.

# Reading and Writing to a Signal

Signals are similar to a copy version of `Rc<RefCell<T>>` built for UIs. You can read and write to a signal like a RefCell:

```rust, no_run
# use dioxus::prelude::*;
let mut signal = use_signal(|| 0);

{
    // This will read the value (we use a block to make sure the read is dropped before the write. You can read more about this in the next section)
    let read = signal.read();
    // Just like refcell, read you can deref the read to get the inner &T reference
    match &*read {
        &0 => println!("read is 0"),
        &1 => println!("read is 1"),
        _ => println!("read is something else ({read})"),
    }
}

// This will write to the value
let mut write = signal.write();
// Again, we can deref the write to get the inner &mut T reference
*write += 1;
```

Signals also have a bunch of helper methods to make it easier to use. Calling it like a function will clone the inner value. You can also call a few traits like AddAssign on it directly without writing to it manually:

```rust, no_run
# use dioxus::prelude::*;
let mut signal = use_signal(|| 0);
// This will clone the value
let clone: i32 = signal();

// You can directly display the signal
println!("{}", signal);

let signal_vec = use_signal(|| vec![1, 2, 3]);
// And use vec methods like .get and .len without reading the signal explicitly
let first = signal_vec.get(0);
let last = signal_vec.last();
let len = signal_vec.len();

// You can also iterate over signals directly
for i in signal_vec.iter() {
    println!("{}", i);
}
```

For a full list of all the helpers available, check out the [`Readable`], [`ReadableVecExt`], [`ReadableOptionExt`], [`Writable`], [`WritableVecExt`], and [`WritableOptionExt`] traits.

Just like `RefCell<T>`, Signal checks borrows at runtime. If you read and write to the signal at the same time, it will panic:

```rust, no_run
# use dioxus::prelude::*;
let mut signal = use_signal(|| 0);
// If you create a read and hold it while you write to the signal, it will panic
let read = signal.read_unchecked();
// This will panic
signal += 1;
println!("{}", read);
```

To avoid issues with overlapping reads and writes, you can use the `with_*` variants of methods to read and write to the signal in a single scope or wrap your reads and writes inside a block:

```rust, no_run
# use dioxus::prelude::*;
let mut signal = use_signal(|| 0);
{
    // Since this read is inside a block that ends before we write to the signal, the signal will be dropped before the write and it will not panic
    let read = signal.read();
    println!("{}", read);
}
signal += 1;

// Or you can use the with and with_write methods which only read or write to the signal inside the closure
signal.with(|read| println!("{}", read));
// Since the read only lasts as long as the closure, this will not panic
signal.with_mut(|write| *write += 1);
```

# Signals with Async

Because signals check borrows at runtime, you need to be careful when reading and writing to signals inside of async code. If you hold a read or write to a signal over an await point, that read or write may still be open while you run other parts of your app:

```rust, no_run
# use dioxus::prelude::*;
# async fn sleep(delay: u32) {}
async fn double_me_async(value: &mut u32) {
    sleep(100).await;
    *value *= 2;
}
let mut signal = use_signal(|| 0);

use_future(move || async move {
    // Don't hold reads or writes over await points
    let mut write = signal.write();
    // While the future is waiting for the async work to finish, the write will be open
    double_me_async(&mut write).await;
});

rsx!{
    // This read may panic because the write is still active while the future is waiting for the async work to finish
    "{signal}"
};
```

Instead of holding a read or write over an await point, you can clone whatever values you need out of your signal and then set the signal to the result once the async work is done:

```rust, no_run
# use dioxus::prelude::*;
# async fn sleep(delay: u32) {}
async fn double_me_async(value: u32) -> u32 {
    sleep(100).await;
    value * 2
}
let mut signal = use_signal(|| 0);

use_future(move || async move {
    // Clone the value out of the signal
    let current_value = signal();
    // Run the async work
    let new_value = double_me_async(current_value).await;
    // Set the signal to the new value
    signal.set(new_value);
});

rsx! {
    // This read will not panic because the write is never held over an await point
    "{signal}"
};
```

# Signals lifecycle

Signals are implemented with [generational-box](https://crates.io/crates/generational-box) which makes all values Copy even if the inner value is not Copy.

This is incredibly convenient for UI development, but it does come with some tradeoffs. The lifetime of the signal is tied to the lifetime of the component it was created in. If you drop the component that created the signal, the signal will be dropped as well. You might run into this if you try to pass a signal from a child component to a parent component and drop the child component. To avoid this you can create your signal higher up in your component tree, use global signals, or create a signal in a specific scope (like the `ScopeId::ROOT`) with [`Signal::new_in_scope`](https://docs.rs/dioxus/latest/dioxus/prelude/struct.Signal.html#method.new_in_scope)

TLDR **Don't pass signals up in the component tree**. It will cause issues:

```rust
# use dioxus::prelude::*;
fn MyComponent() -> Element {
    let child_signal = use_signal(|| None);

    rsx! {
        IncrementButton {
            child_signal
        }
    }
}

#[component]
fn IncrementButton(mut child_signal: Signal<Option<Signal<i32>>>) -> Element {
    let signal_owned_by_child = use_signal(|| 0);
    // Don't do this: it may cause issues if you drop the child component
    child_signal.set(Some(signal_owned_by_child));

    todo!()
}
```
