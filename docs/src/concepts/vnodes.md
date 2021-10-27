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

Every element has a set of properties that can be rendered in different ways. In particular, each Element may contain other Elements. To achieve this, we can simply declare new Elements within the parent:

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
With the default configuration, any Element defined within the `dioxus-html` crate can be declared in this way. To create your own new elements, see the `Custom Elements` Advanced Guide.

## Text Elements

Dioxus also supports a special type of Element: Text. Text Elements do not accept children, but rather just text denoted with double quotes.

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
Text can also be formatted with any value that implements `Display`. We use f-string formatting - a "coming soon" feature for stable Rust that is familiar for Python and JavaScript users:

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
        "custom_attr": "important data here"
    }
)
```

## Listeners

## Arbitrary Tokens
