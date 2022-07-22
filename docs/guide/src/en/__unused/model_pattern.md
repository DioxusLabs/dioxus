

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

Our component is really simple – we just call `use_ref` to get an initial calculator state.

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

