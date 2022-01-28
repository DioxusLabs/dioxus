# Declaring your first UI with Elements

Every user interface you've ever used is just a symphony of tiny widgets working together to abstract over larger complex functions. In Dioxus, we call these tiny widgets "Elements." Using Components, you can easily compose Elements into larger groups to form even larger structures: Apps.

In this chapter, we'll cover:
- Declaring our first Element
- Composing Elements together
- Element properties

## Declaring our first Element
Because Dioxus is mostly used with HTML/CSS renderers, the default Element "collection" is HTML. Provided the `html` feature is not disabled, we can declare Elements using the `rsx!` macro:

```rust
rsx!(
    div {}
)
```
As you might expect, we can render this call using Dioxus-SSR to produce valid HTML:

```rust
dioxus::ssr::render_lazy(rsx!(
    div {}
))
```

Produces:
```html
<div></div>
```

We can construct any valid HTML tag with the `tag {}` pattern and expect the resulting HTML structure to resemble our declaration.
## Composing Elements

Of course, we need more complex structures to make our apps actually useful! Just like HTML, the `rsx!` macro lets us nest Elements inside of each other.

```rust
#use dioxus::prelude::*;
rsx!(
    div {
        h1 {}
        h2 {}
        p {}
    }
)
```
As you might expect, the generated HTML for this structure would look like:
```html
<div>
    <h1></h1>
    <h2></h2>
    <p></p>
</div>
```

With the default configuration, any Element defined within the `dioxus-html` crate can be declared in this way. To create your own new elements, see the `Custom Elements` Advanced Guide.

## Text Elements

Dioxus also supports a special type of Element: Text. Text Elements do not accept children, but rather just string literals denoted with double quotes.

```rust
rsx! (
    "hello world"
)
```

Text Elements can be composed within other Elements:
```rust
rsx! (
    div {
        h1 { "hello world" }
        p { "Some body content" }
    }
)
```

Text can also be formatted with any value that implements `Display`. We use [f-string formatting](https://docs.rs/fstrings/0.2.3/fstrings/) - a "coming soon" feature for stable Rust that is familiar for Python and JavaScript users:

```rust
let name = "Bob";
rsx! ( "hello {name}" )
```

Unfortunately, you cannot drop in arbitrary expressions directly into the string literal. In the cases where we need to compute a complex value, we'll want to use `format_args!` directly. Due to specifics of how the `rsx!` macro (we'll cover later), our call to `format_args` must be contained within  square braces.

```rust
rsx!( {format_args!("Hello {}", if enabled { "Jack" } else { "Bob" } )] )
```

Alternatively, `&str` can be included directly, though it must be inside of square braces:

```rust
rsx!( "Hello ",  [if enabled { "Jack" } else { "Bob" }] )
```

This is different from React's way of generating arbitrary markup but fits within idiomatic Rust.

Typically, with Dioxus, you'll just want to compute your substrings outside of the `rsx!` call and leverage the f-string formatting:

```rust
let name = if enabled { "Jack" } else { "Bob" };
rsx! ( "hello {name}" )
```

## Attributes

Every Element in your User Interface will have some sort of properties that the renderer will use when drawing to the screen. These might inform the renderer if the component should be hidden, what its background color should be, or to give it a specific name or ID.

To do this, we use the familiar struct-style syntax that Rust provides:

```rust
rsx!(
    div {
        hidden: "true",
        background_color: "blue",
        class: "card color-{mycolor}"
    }
)
```

Each field is defined as a method on the element in the `dioxus-html` crate. This prevents you from misspelling a field name and lets us provide inline documentation. When you need to use a field not defined as a method, you have two options:

1) file an issue if the attribute _should_ be enabled
2) add a custom attribute on-the-fly

To use custom attributes, simply put the attribute name in quotes followed by a colon:

```rust
rsx!(
    div {
        "customAttr": "important data here"
    }
)
```

> Note: the name of the custom attribute must match exactly what you want the renderer to output. All attributes defined as methods in `dioxus-html` follow the snake_case naming convention. However, they internally translate their snake_case convention to HTML's camelCase convention. When using custom attributes, make sure the name of the attribute **exactly** matches what the renderer is expecting.

All element attributes must occur *before* child elements. The `rsx!` macro will throw an error if your child elements come before any of your attributes. If you don't see the error, try editing your Rust-Analyzer IDE setting to ignore macro-errors. This is a temporary workaround because Rust-Analyzer currently throws *two* errors instead of just the one we care about.

```rust
// settings.json
{
  "rust-analyzer.diagnostics.disabled": [
    "macro-error"
  ],
}
```

## Listeners

Listeners are a special type of Attribute that only accept functions. Listeners let us attach functionality to our Elements by running a provided closure whenever the specified Listener is triggered.

We'll cover listeners in more depth in the Listeners chapter, but for now, just know that every listener must start with the `on` keyword and accepts closures.

```rust
rsx!(
    div {
        onclick: move |_| log::debug!("div clicked!"),
    }
)
```

## Moving On

This chapter just scratches the surface on how Elements can be defined.

We learned:
- Elements are the basic building blocks of User Interfaces
- Elements can contain other elements
- Elements can either be a named container or text
- Some Elements have properties that the renderer can use to draw the UI to the screen

Next, we'll compose Elements together using Rust-based logic.
