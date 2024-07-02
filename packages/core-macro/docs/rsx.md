The rsx! macro makes it easy for developers to write jsx-style markup in their components.

## Elements

You can render elements with rsx! with the element name and then braces surrounding the attributes and children.

```rust, no_run
# use dioxus::prelude::*;
rsx! {
    div {
        div {}
    }
};
```

<details>
<summary>Web Components</summary>

Dioxus will automatically render any elements with `-` as a untyped web component:

```rust, no_run
# use dioxus::prelude::*;
rsx! {
    div-component {
        div {}
    }
};
```

You can wrap your web component in a custom component to add type checking:

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn MyDivComponent(width: i64) -> Element {
    rsx! {
        div-component {
            "width": width
        }
    }
}
```

</details>

## Attributes

You can add attributes to any element inside the braces. Attributes are key-value pairs separated by a colon.

```rust, no_run
# use dioxus::prelude::*;
let width = 100;
rsx! {
    div {
        // Set the class attribute to "my-class"
        class: "my-class",
        // attribute strings are automatically formatted with the format macro
        width: "{width}px",
    }
};
```

### Optional Attributes

You can include optional attributes with an unterminated if statement as the value of the attribute:

```rust, no_run
# use dioxus::prelude::*;
# let first_boolean = true;
# let second_boolean = false;
rsx! {
    div {
        // Set the class attribute to "my-class" if true
        class: if first_boolean {
            "my-class"
        },
        // Set the class attribute to "my-other-class" if false
        class: if second_boolean {
            "my-other-class"
        }
    }
};
```

### Raw Attributes

Dioxus defaults to attributes that are type checked as html. If you want to include an attribute that is not included in the html spec, you can use the `raw` attribute surrounded by quotes:

```rust, no_run
# use dioxus::prelude::*;
rsx! {
    div {
        // Set the data-count attribute to "1"
        "data-count": "1"
    }
};
```

## Text

You can include text in your markup as a string literal:

```rust, no_run
# use dioxus::prelude::*;
let name = "World";
rsx! {
    div {
        "Hello World"
        // Just like attributes, you can included formatted segments inside your text
        "Hello {name}"
    }
};
```

## Components

You can render any [`macro@crate::component`]s you created inside your markup just like elements. Components must either start with a capital letter or contain a `_` character.

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn HelloWorld() -> Element {
    rsx! { "hello world!" }
}

rsx! {
    div {
        HelloWorld {}
    }
};
```

## If statements

You can use if statements to conditionally render children. The body of the for if statement is parsed as rsx markup:

```rust, no_run
# use dioxus::prelude::*;
let first_boolean = true;
let second_boolean = false;
rsx! {
    if first_boolean {
        div {
            "first"
        }
    }

    if second_boolean {
        "second"
    }
};
```

## For loops

You can also use for loops to iterate over a collection of items. The body of the for loop is parsed as rsx markup:

```rust, no_run
# use dioxus::prelude::*;
let numbers = vec![1, 2, 3];
rsx! {
    for number in numbers {
        div {
            "{number}"
        }
    }
};
```

## Raw Expressions

You can include raw expressions inside your markup inside curly braces. Your expression must implement the [`IntoDynNode`](https://docs.rs/dioxus-core/latest/dioxus_core/trait.IntoDynNode.html) trait:

```rust, no_run
# use dioxus::prelude::*;
let name = "World";
rsx! {
    div {
        // Text can be converted into a dynamic node in rsx
        {name}
    }
    // Iterators can also be converted into dynamic nodes
    {(0..10).map(|n| n * n).map(|number| rsx! { div { "{number}" } })}
};
```
