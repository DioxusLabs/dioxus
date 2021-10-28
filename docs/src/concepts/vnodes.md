# Declaring your first UI with Elements

Every user interface you've ever used is just a symphony of tiny widgets working together to abstract over larger complex functions. In Dioxus, we call these tiny widgets "Elements." Using Components, you can easily compose Elements into larger groups to form even larger structures: Apps.

Because Dioxus is mostly used with HTML/CSS renderers, the default Element "collection" is HTML. Provided the `html` feature is not disabled, we can declare Elements using the `rsx!` macro:

```rust
#use dioxus::prelude::*;
rsx!(
    div {}
)
```
As you might expect, we can render this call using Dioxus-SSR to produce valid HTML:

```rust
#use dioxus::prelude::*;
dioxus::ssr::render_lazy(rsx!(
    div {}
))
```
Produces:
```html
<div></div>
```

## Composing Elements

Every element has a set of properties that can be rendered in different ways. In particular, each Element may contain other Elements. To achieve this, we can simply declare new Elements contained within the parent's curly braces:

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

## Attributes

Every Element in your User Interface will have some sort of properties that the renderer will use when drawing to the screen. These might inform the renderer if the component should be hidden, what its background color should be, or to give it a specific name or ID.

To do this, we simply use the familiar struct-style syntax that Rust provides us. Commas are optional:

```rust
rsx!(
    div {
        hidden: true,
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

Note: the name of the custom attribute must match exactly what you want the renderer to output. All attributes defined as methods in `dioxus-html` follow the snake_case naming convention. However, they internally translate their snake_case convention to HTML's camelCase convention.

## Listeners

Listeners are a special type of Attribute that only accept functions. Listeners let us attach functionality to our Elements by running a provided closure whenever the specified Listener is triggered.

We'll cover listeners in more depth in the Listeners chapter, but for now, just know that every listener must start with the `on` keyword and can accept either a closure or an expression wrapped in curly braces.

```rust
rsx!(
    div {
        onclick: move |_| {}
        onmouseover: {handler},
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

Next, we'll compose Elements together to form components.
