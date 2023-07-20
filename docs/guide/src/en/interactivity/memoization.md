# Memoization

[`use_memo`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_memo.html) let's you memorize values and thus save computation time. This is useful for expensive calculations.

```rust, no_run
#[inline_props]
fn Calculator(cx: Scope, number: usize) -> Element {
    let bigger_number = use_memo(cx, (number,), |(number,)| {
        // This will only be calculated when `number` has changed.
        number * 100
    });
    render!(
        p { "{bigger_number}" }
    )
}
fn app(cx: Scope) -> Element {
    render!(Calculator { number: 0 })
}
```
