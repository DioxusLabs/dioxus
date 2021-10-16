### Immutability by default?

---

Rust, like JS and TS, supports both mutable and immutable data. With JS, `const` would be used to signify immutable data, while in rust, the absence of `mut` signifies immutable data.

Mutability:

```rust
let mut val = 10; // rust
let val = 10;     // js
```

Immutability

```rust
let val = 10;    // rust
const val = 10;  // js
```

However, `const` in JS does not prohibit you from modify the value itself only disallowing assignment. In Rust, immutable **is immutable**. You _never_ have to work about accidentally mutating data; mutating immutable data in Rust requires deliberate advanced datastructures that you won't find in your typical frontend code.

## How do strings work?

---

In rust, we have `&str`, `&'static str` `String`, and `Rc<str>`. It's a lot, yes, and it might be confusing at first. But it's actually not too bad.

In Rust, UTF-8 is supported natively, allowing for emoji and extended character sets (like Chinese and Arabic!) instead of the typical ASCII. The primitive `str` can be seen as a couple of UTF-8 code points squished together with a dynamic size. Because this size is variable (not known at compile time for any single character), we reference an array of UTF-8 code points as `&str`. Essentially, we're referencing (the & symbol) some dynamic `str` (a collection of UTF-8 points).

For text encoded directly in your code, this collection of UTF-8 code points is given the `'static` reference lifetime - essentially meaning the text can be safely referenced for the entire runtime of your program. Contrast this with JS, where a string will only exist for as long as code references it before it gets cleaned up by the garbage collector.

For text that needs to have characters added, removed, sorted, uppercased, formatted, accessed for mutation, etc, Rust has the `String` type, which is essentially just a dynamically sized `str`. In JS, if you add a character to your string, you actually create an entirely new string (completely cloning the old one first). In Rust, you can safely added characters to strings _without_ having to clone first, making string manipulation in Rust very efficient.

Finally, we have `Rc<str>`. This is essentially Rust's version of JavaScript's `string`. In JS, whenever you pass a `string` around (and don't mutate it), you don't actually clone it, but rather just increment a counter that says "this code is using this string." This counter prevents the garbage collector from deleting the string before your code is done using it. Only when all parts of your code are done with the string, will the string be deleted. `Rc<str>` works exactly the same way in Rust, but requires a deliberate `.clone()` to get the same behavior. In most instances, Dioxus will automatically do this for you, saving the trouble of having to `clone` when you pass an `Rc<str>` into child components. `Rc<str>` is typically better than `String` for Rust - it allows cheap sharing of strings, and through `make_mut` you can always produce your own mutable copy for modifying. You might not see `Rc<str>` in other Rust libraries as much, but you will see it in Dioxus due to Dioxus' aggressive memoization and focus on efficiency and performance.

If you run into issues with `&str`, `String`, `Rc<str>`, just try cloning and `to_string` first. For the vast majority of apps, the slight performance hit will be unnoticeable. Once you get better with Strings, it's very easy to go back and remove all the clones for more efficient alternatives, but you will likely never need to.
