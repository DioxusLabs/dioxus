# Component

The component macro turns a function with arguments that are [`Clone`] and [`PartialEq`] into a component. This is the recommended way of creating most components. If you want more fine grained control over how the overall prop struct implements the `Properties` trait, you can use an explicit props struct with the [`Props`] derive macro instead.

## Arguments

- `no_case_check` - Doesn't enforce `PascalCase` on your component names.
  **This will be removed/deprecated in a future update in favor of a more complete Clippy-backed linting system.**
  The reasoning behind this is that Clippy allows more robust and powerful lints, whereas
  macros are extremely limited.

## Features

This attribute:

- Enforces that your component uses `PascalCase` or `snake_case` with at least one underscore.
- Automatically creates a prop struct for your component if the function has arguments.
- Verifies the function signature is valid as a component.

## Examples

- Without props:

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn GreetBob() -> Element {
    rsx! { "hello, bob" }
}
```

- With props:

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn GreetBob(bob: String) -> Element {
    rsx! { "hello, {bob}" }
}
```

## Prop Modifiers

You can use the `#[props()]` attribute to modify the behavior of the props the component macro creates:

- [`#[props(default)]`](#default-props) - Makes the field optional in the component and uses the default value if it is not set when creating the component.
- [`#[props(!optional)]`](#optional-props) - Makes a field with the type `Option<T>` required.
- [`#[props(into)]`](#converting-props) - Converts a field into the correct type by using the [`Into`] trait.
- [`#[props(extends = GlobalAttributes)]`](#extending-elements) - Extends the props with all the attributes from an element or the global element attributes.

Props also act slightly differently when used with:

- [`Option<T>`](#optional-props) - The field is automatically optional with a default value of `None`.
- [`ReadOnlySignal<T>`](#reactive-props) - The props macro will automatically convert `T` into `ReadOnlySignal<T>` when it is passed as a prop.
- [`String`](#formatted-props) - The props macro will accept formatted strings for any prop field with the type `String`.
- [`children`](#children-props) - The props macro will accept child elements if you include the `children` prop.

### Default Props

The `default` attribute lets you define a default value for a field if it isn't set when creating the component

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn Button(
    // The default attributes makes your field optional in the component and uses the default value if it is not set.
    #[props(default)]
    text: String,
    // You can also set an explicit default value instead of using the `Default` implementation.
    #[props(default = "red".to_string())]
    color: String,
) -> Element {
    rsx! {
        button {
            color: color,
            "{text}"
        }
    }
}

rsx! {
    // You can skip setting props that have a default value when you use the component.
    Button {}
};
```

### Optional Props

When defining a component, you may want to make a prop optional without defining an explicit default value. Any fields with the type `Option<T>` are automatically optional with a default value of `None`.

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn Button(
    // Since the `text` field is optional, you don't need to set it when you use the component.
    text: Option<String>,
) -> Element {
    rsx! {
        button { {text.unwrap_or("button".to_string())} }
    }
}

rsx! {
    Button {}
};
```

If you want to make your `Option<T>` field required, you can use the `!optional` attribute:

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn Button(
    // You can use the `!optional` attribute on a field with the type `Option<T>` to make it required.
    #[props(!optional)]
    text: Option<String>,
) -> Element {
    rsx! {
        button { {text.unwrap_or("button".to_string())} }
    }
}

rsx! {
    Button {
        text: None
    }
};
```

### Converting Props

You can automatically convert a field into the correct type by using the `into` attribute. Any type you pass into the field will be converted with the [`Into`] trait:

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn Button(
    // You can use the `into` attribute on a field to convert types you pass in with the Into trait.
    #[props(into)]
    number: u64,
) -> Element {
    rsx! {
        button { "{number}" }
    }
}

rsx! {
    Button {
        // Because we used the into attribute, we can pass in any type that implements Into<u64>
        number: 10u8
    }
};
```

### Formatted Props

You can use formatted strings in attributes just like you would in an element. Any prop field with the type `String` can accept a formatted string:

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn Button(text: String,) -> Element {
    rsx! {
        button { "{text}" }
    }
}

let name = "Bob";
rsx! {
    Button {
        // You can use formatted strings in props that accept String just like you would in an element.
        text: "Hello {name}!"
    }
};
```

### Children Props

Rather than passing the RSX through a regular prop, you may wish to accept children similarly to how elements can have children. The "magic" children prop lets you achieve this:

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn Clickable(
    href: String,
    children: Element,
) -> Element {
    rsx! {
        a {
            href: "{href}",
            class: "fancy-button",
            {children}
        }
    }
}
```

This makes providing children to the component much simpler: simply put the RSX inside the {} brackets:

```rust, no_run
# use dioxus::prelude::*;
# #[component]
# fn Clickable(
#     href: String,
#     children: Element,
# ) -> Element {
#     rsx! {
#         a {
#             href: "{href}",
#             class: "fancy-button",
#             {children}
#         }
#     }
# }
rsx! {
    Clickable {
        href: "https://www.youtube.com/watch?v=C-M2hs3sXGo",
        "How to "
        i { "not" }
        " be seen"
    }
};
```

### Reactive Props

In dioxus, when a prop changes, the component will rerun with the new value to update the UI. For example, if count changes from 0 to 1, this component will rerun and update the UI to show "Count: 1":

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn Counter(count: i32) -> Element {
    rsx! {
        div {
            "Count: {count}"
        }
    }
}
```

Generally, just rerunning the component is enough to update the UI. However, if you use your prop inside reactive hooks like `use_memo` or `use_resource`, you may also want to restart those hooks when the prop changes:

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn Counter(count: i32) -> Element {
    // We can use a memo to calculate the doubled count. Since this memo will only be created the first time the component is run and `count` is not reactive, it will never update when `count` changes.
    let doubled_count = use_memo(move || count * 2);
    rsx! {
        div {
            "Count: {count}"
            "Doubled Count: {doubled_count}"
        }
    }
}
```

To fix this issue you can either:

1. Make the prop reactive by wrapping it in `ReadOnlySignal` (recommended):

`ReadOnlySignal` is a `Copy` reactive value. Dioxus will automatically convert any value into a `ReadOnlySignal` when it is passed as a prop.

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn Counter(count: ReadOnlySignal<i32>) -> Element {
    // Since we made count reactive, the memo will automatically rerun when count changes.
    let doubled_count = use_memo(move || count() * 2);
    rsx! {
        div {
            "Count: {count}"
            "Doubled Count: {doubled_count}"
        }
    }
}
```

2. Explicitly add the prop as a dependency to the reactive hook with [`use_reactive`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/macro.use_reactive.html):

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn Counter(count: i32) -> Element {
    // We can add the count prop as an explicit dependency to every reactive hook that uses it with use_reactive.
    // The use_reactive macro takes a closure with explicit dependencies as its argument.
    let doubled_count = use_memo(use_reactive!(|count| count * 2));
    rsx! {
        div {
            "Count: {count}"
            "Doubled Count: {doubled_count}"
        }
    }
}
```

### Extending Elements

The `extends` attribute lets you extend your props with all the attributes from an element or the global element attributes.

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn Button(
    // You can use the `extends` attribute on a field with the type `Vec<Attribute>` to extend the props with all the attributes from an element or the global element attributes.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
) -> Element {
    rsx! {
        // Instead of copying over every single attribute, we can just spread the attributes from the props into the button.
        button { ..attributes, "button" }
    }
}

rsx! {
    // Since we extend global attributes, you can use any attribute that would normally appear on the button element.
    Button {
        width: "10px",
        height: "10px",
        color: "red",
    }
};
```
