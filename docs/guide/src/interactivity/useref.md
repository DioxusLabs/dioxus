# UseRef

You might've noticed a fundamental limitation to `use_state`: to modify the value in-place, it must be cheaply cloneable. But what if your type is not cheap to clone?

In these cases, you should reach for `use_ref` which is essentially just a glorified `Rc<RefCell<T>>` (Rust [smart pointers](https://doc.rust-lang.org/book/ch15-04-rc.html)).

This provides us some runtime locks around our data, trading reliability for performance. For most cases though, you will find it hard to make `use_ref` crash.

> Note: this is *not* exactly like its React counterpart since calling `write` will cause a re-render. For more React parity, use the `write_silent` method.

To use the hook:

```rust
let names = use_ref(&cx, || vec!["susie", "calvin"]);
```

To read the hook values, use the `read()` method:

```rust
cx.render(rsx!{
    ul {
        names.read().iter().map(|name| {
            rsx!{ "{name}" }
        })
    }
})
```

And then to write into the hook value, use the `write` method:

```rust
names.write().push("Tiger");
```

If you don't want to re-render the component when names is updated, then we can use the `write_silent` method:

```rust
names.write_silent().push("Transmogrifier");
```

Again, like `UseState`, the `UseRef` handle is clonable into async contexts:


```rust
// infinitely push calvin into the list
cx.spawn({
    to_owned![names];
    async move {
        loop {
            wait_ms(100).await;
            names.write().push("Calvin");
        }
    }
})
```

## Using UseRef for complex state

The best use case of `use_ref` is to manage "complex" data local to your component. This should be things like large structs, collections, queues, state machines, and other data where cloning would be expensive.


```rust
let val = use_state(&cx, || vec![1, 3, 3, 7]);
let val = use_state(&cx, || (0..10000).collect::<Vec<_>x>());
let val = use_state(&cx, || Configuration {
    a: "asdasd",
    // .. more complex fields
});
```

UseRef can be sent down into child components too.

UseRef memoizes with itself, performing a cheap pointer comparison. If the UseRef handle is the same, then the component can be memoized.

```rust

fn app(cx: Scope) -> Element {
    let val = use_ref(&cx, || 0);

    cx.render(rsx!{
        Child { val: val.clone() }
    })
}

fn Child(cx: Scope, val: UseRef<i32>) -> Element {
    // ...
}
```

## Using UseRef with "models"

One option for state management that UseRef enables is the development of a "model" for your components. This particular pattern enables you to model your state with regular structs.

For instance, our calculator example uses a struct to model the state.

```rust

struct Calculator {
    display_value: String,
    operator: Option<Operator>,
    waiting_for_operand: bool,
    cur_val: f64,
}
```

Our component is really simple - we just call `use_ref` to get an initial calculator state.

```rust
fn app(cx: Scope) -> Element {
    let state = use_ref(&cx, Calculator::new);

    cx.render(rsx!{
        // the rendering code
    })
}
```

In our UI, we can then use `read` and a helper method to get data out of the model.

```rust
// Our accessor method
impl Calculator {
    fn formatted_display(&self) -> String {
        self.display_value
            .parse::<f64>()
            .unwrap()
            .separated_string()
    }
}

// And then in the UI
cx.render(rsx!{
    div { [state.read().formatted_display()] }
})
```

To modify the state, we can setup a helper method and then attach it to a callback.

```rust
// our helper
impl Calculator {
    fn clear_display(&mut self) {
        self.display_value = "0".to_string()
    }
}

// our callback
cx.render(rsx!{
    button {
        onclick: move |_| state.write().clear_display(),
    }
})
```

