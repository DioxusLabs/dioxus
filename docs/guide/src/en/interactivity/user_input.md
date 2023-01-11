# User Input

Interfaces often need to provide a way to input data: e.g. text, numbers, checkboxes, etc. In Dioxus, there are two ways you can work with user input.

## Controlled Inputs

With controlled inputs, you are directly in charge of the state of the input. This gives you a lot of flexibility, and makes it easy to keep things in sync. For example, this is how you would create a controlled text input:

```rust
{{#include ../../../examples/input_controlled.rs:component}}
```

Notice the flexibility – you can:
- Also display the same contents in another element, and they will be in sync
- Transform the input every time it is modified (e.g. to make sure it is upper case)
- Validate the input every time it changes
- Have custom logic happening when the input changes (e.g. network request for autocompletion)
- Programmatically change the value (e.g. a "randomize" button that fills the input with nonsense)

## Uncontrolled Inputs

As an alternative to controlled inputs, you can simply let the platform keep track of the input values. If we don't tell a HTML input what content it should have, it will be editable anyway (this is built into the browser). This approach can be more performant, but less flexible. For example, it's harder to keep the input in sync with another element.

Since you don't necessarily have the current value of the uncontrolled input in state, you can access it either by listening to `oninput` events (similarly to controlled components), or, if the input is part of a form, you can access the form data in the form events (e.g. `oninput` or `onsubmit`):

```rust
{{#include ../../../examples/input_uncontrolled.rs:component}}
```
```
Submitted! UiEvent { data: FormData { value: "", values: {"age": "very old", "date": "1966", "name": "Fred"} } }
```