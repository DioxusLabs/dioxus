## Testing

To test your Rust code, you can annotate any function with the `#[test]` block. In VSCode with RA, this will provide a lens to click and run the test.

```rust
#[test]
fn component_runs() {
    assert!(true)
}
```

This will test your Rust code _without_ going through the browser. This is ideal for squashing logic bugs and ensuring components render appropriately when the browsers's DOM is not needed. If you need to run tests in the browser, you can annotate your blocks with the `#[dioxus::test]` block.

```rust
#[dioxus::test]
fn runs_in_browser() {
    // ...
}
```

Then, when you run

```console
$ dioxus test --chrome
```

Dioxus will build and test your code using the Chrome browser as a harness.

There's a lot more to testing if you dive in, so check out the [Testing]() guide for more information
