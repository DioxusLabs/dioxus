# User Input and Controlled Components

Handling user input is one of the most common things your app will do, but it *can* be tricky.

The reactive paradigm and one-way-data-flow models were all derived to solve problems that can crop up around user input handling. This section will teach you about two effective patterns for handling user input: controlled and uncontrolled inputs.

## Controlled Inputs

The most common approach to handling input from elements is through "controlled" inputs. With this pattern, we drive the value of the input from our state, while simultaneously updating our state from new values.

This is the most common form of input handling and is widely used because it prevents the UI from being out of sync with your local state.

```rust
let name = use_state(&cx, || "bob".to_string());

cx.render(rsx!{
    input {
        value: "{name}",
        oninput: move |evt| name.set(evt.value.clone()),
    }
})
```

Why not just "bind" like in other frameworks?

In a handful of cases, you don't want the inputted value to match what's rendered to the screen. Let's say we want to implement an input that converts the input to uppercase when the input matches a certain value. With binding, we're forced to share the same input and state value. By explicitly handling the oninput case, we're given the opportunity to set a *new* value.


```rust
let name = use_state(&cx, || "bob".to_string());

cx.render(rsx!{
    input {
        value: "{name}",
        oninput: move |evt| {
            if evt.value == "tim" {
                name.set("TIM");
            }
        },
    }
})
```


## Binding

>! Note - binding is currently not implemented in Dioxus. This section represents a future in-progress feature.

Because the above pattern is so common, we have an additional attribute called "bind" which is essentially a shorthand for our code above.

Bind just connects an oninput to a `UseState` and is implemented through the signal system.

```rust
let name = use_state(&cx, || "bob".to_string());

cx.render(rsx!{
    input { bind: name }
})
```

## Uncontrolled Inputs

When working with large sets of inputs, you might be quickly tired of creating `use_state` for each value. Additionally, the pattern of one `use_state` per interaction might deteriorate when you need to have a flexible number of inputs. In these cases, we use "uncontrolled" inputs. Here, we don't drive the value of the input from the `use_state`, choosing to leave it in an "uncontrolled" state.

This approach can be more performant and more flexible, but more prone to UI inconsistencies than its controlled counterpart.

To use the "uncontrolled" pattern, we simply omit setting the value of the input. Instead, we can react to the change directly on the input itself, or from a form element higher up in the tree.

For this example, we don't attach any `use_state` handles into the labels. Instead, we simply attach an `oninput` handler to the form element. This will run each time any of the child inputs change, allowing us to perform tasks like form validation.

```rust
form {
    oninput: move |evt| {
        if !validate_input(evt.values) {
            // display what errors validation resulted in
        }
    },
    input { name: "name", }
    input { name: "age", }
    input { name: "date", }
}
```
