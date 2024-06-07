Creates a new Memo. The memo will be run immediately and whenever any signal it reads is written to. Memos can be used to efficiently compute derived data from signals.

```rust
use dioxus::prelude::*;
use dioxus_signals::*;

fn App() -> Element {
    let mut count = use_signal(|| 0);
    // the double memo will always be equal to two times the value of count, even after count changes
    let double = use_memo(move || count * 2);

    rsx! {
        "{double}"
        button {
            // When count changes, the memo will rerun and double will be updated
            // memos rerun any time you write to a signal they read. They will only rerun values/component that depend on them if the value of the memo changes
            onclick: move |_| count += 1,
            "Increment"
        }
    }
}
```

The closure you pass into memos will be called whenever the state you read inside the memo is written to even if the value hasn't actually changed, but the memo you get will not rerun other parts of your app unless the output changes (`PartialEq` returns false).

Lets dig into some examples to see how this works:

```rust, no_run
# use dioxus::prelude::*;
let mut count = use_signal(|| 1);
// double_count will rerun when state we read inside the memo changes (count)
let double_count = use_memo(move || count() * 2);

// memos act a lot like a read only version of a signal. You can read them, display them, and move them around like any other signal
println!("{}", double_count); // Prints "2"

// But you can't write to them directly
// Instead, any time you write to a value the memo reads, the memo will rerun
count += 1;

println!("{}", double_count); // Prints "4"

// Lets create another memo that reads the value of double_count
let double_count_plus_one = use_memo(move || double_count() + 1);

println!("{}", double_count_plus_one); // Prints "5"

// Now if we write to count the double_count memo will rerun
// If that the output of double_count changes, then it will cause double_count_plus_one to rerun
count += 1;

println!("{}", double_count); // Prints "6"
println!("{}", double_count_plus_one); // Prints "7"

// However if the value of double_count doesn't change after a write, then it won't trigger double_count_plus_one to rerun
// Since we just write the same value, the doubled value is still 6 and we don't rerun double_count_plus_one
*count.write() = 3;

println!("{}", double_count); // Prints "6"
println!("{}", double_count_plus_one); // Prints "7"
```

## With non-reactive dependencies

To add non-reactive dependencies, you can use the [`crate::use_reactive()`] hook.

Signals will automatically be added as dependencies, so you don't need to call this method for them.

```rust
# use dioxus::prelude::*;
#[component]
fn Comp(count: u32) -> Element {
// Because the memo subscribes to `count` by adding it as a dependency, the memo will rerun every time `count` changes.
let new_count = use_memo(use_reactive((&count,), |(count,)| count + 1));
    todo!()
}
```
