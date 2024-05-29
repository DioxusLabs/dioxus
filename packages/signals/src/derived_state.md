Memos let you derive state from other state

A huge part of state management is deriving state from other state. In dioxus, the main way you derive state is through memos. Memos are functions that take state as input and return a new state.

The closure you pass into memos will be called whenever the state you read inside the memo changes, but the memo you get will not rerun other parts of your app unless the output changes (`PartialEq` returns false).

That is a lot, lets dig into some examples to see how this works:

```rust
let count = use_state(|| 1);
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

That was a lot! In summary, memos let you derive state in your app that updates automatically. They memorize the output of the closure and only rerun other parts of your app when the output changes.

The good news is this is the core of the dioxus reactive system. Memos, Resources, and Effects all rerun in a very similar way. If you have a good grasp of how memos work, understanding the other two will be very easy.
