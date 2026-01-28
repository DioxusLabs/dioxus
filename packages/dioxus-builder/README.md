# dioxus-builder

Fully Rust typed builder API for [Dioxus](https://dioxuslabs.com/).

This crate provides a fluent builder interface for constructing HTML elements with full IDE autocomplete support, as an alternative to the `rsx!` macro.

## Features

- **Full IDE Autocomplete** - Type-safe builder methods with complete IntelliSense support
- **Fluent API** - Chain methods naturally: `div().class("foo").id("bar").child(...).build()`
- **Component Builders** - Use `#[component(builder)]` with `#[props(into)]` for typed builders
- **80+ HTML Elements** - All standard HTML elements with proper namespaces for SVG/MathML
- **SVG Attributes** - Typed helpers like `.viewBox()`, `.stroke()`, `.d()`, and more
- **Smart Class Merging** - Multiple `.class()` calls are automatically merged
- **Conditional Helpers** - `.class_if()`, `.attr_if()`, `.child_if()`, `.child_if_else()`
- **Key Support** - `.key()` for efficient list reconciliation
- **Hybrid Templates** - Mix static and dynamic content for optimal performance
- **Style Helpers** - `.style_prop()` and `.with()` for composable styling
- **ARIA + data Helpers** - `.aria_*()` methods and the `data!` macro
- **Document Helpers** - Easy document head management (requires `document` feature)
- **Fragment Support** - Build multiple root nodes with `fragment()`
- **Debug Support** - `Debug` impls for builders and static nodes

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dioxus-builder = { version = "0.7" }

# For document head helpers (title, stylesheet, meta tags):
dioxus-builder = { version = "0.7", features = ["document"] }
```

## Quick Start

```rust
use dioxus::prelude::*;
use dioxus_builder::*;

fn app() -> Element {
    div()
        .class("container mx-auto p-4")
        .id("main")
        .child(
            h1().class("text-2xl font-bold")
                .child("Hello, World!")
        )
        .child(
            button()
                .class("btn btn-primary")
                .onclick(|_| println!("Clicked!"))
                .child("Click me")
        )
        .build()
}
```

## Component Integration

### Component Macro: `#[component(builder)]`

```rust,ignore
use dioxus::prelude::*;
use dioxus_builder::FunctionComponent;

#[component(builder)]
fn Counter(initial: i32, #[props(into)] label: String) -> Element {
    let count = use_signal(|| initial);
    div().text(format!("{}: {}", label, count())).build()
}

// Usage:
// Counter.new().initial(0).label("Clicks").build()
```

## API Overview

### Basic Element Construction

```rust
// Create elements with constructor functions
div()
    .class("my-class")
    .id("my-id")
    .child("Hello, World!")
    .build()

// Nested elements
ul()
    .children((0..5).map(|i| {
        li().child(format!("Item {}", i))
    }))
    .build()
```

### Attributes

```rust
div()
    // Common attributes have dedicated methods
    .class("container")
    .id("main")
    .style("color: red")
    .title("Hover text")
    .hidden(false)
    .tabindex(0)
    .role("button")
    .draggable(true)

    // Custom attributes
    .attr("data-custom", "value")
    .attr_ns("xlink:href", "http://www.w3.org/1999/xlink", "#icon")

    // Spread multiple attributes
    .attrs(vec![
        Attribute::new("data-a", "1", None, false),
        Attribute::new("data-b", "2", None, false),
    ])
    .build()
```

### ARIA and Data Helpers

```rust
use dioxus_builder::{data, BuilderExt};

button()
    .aria_label("Close dialog")
    .aria_expanded(false)
    .build();

div()
    .pipe(|builder| data!(builder, "testid", "dialog"))
    .build();
```

### Conditional Attributes and Children

```rust
let is_active = true;
let user_name: Option<&str> = Some("Alice");

div()
    // Conditional classes
    .class("base-class")
    .class_if(is_active, "active")

    // Conditional attributes
    .attr_if(is_active, "data-active", "true")

    // Conditional children
    .child_if(is_active, span().child("Active!"))

    // If-else children
    .child_if_else(
        is_active,
        span().class("text-green").child("Online"),
        span().class("text-gray").child("Offline"),
    )

    // Optional children
    .child_option(user_name.map(|name| span().child(name)))
    .build()
```

### Class Merging

Multiple `.class()` calls are automatically merged:

```rust
div()
    .class("px-4 py-2")
    .class("bg-blue-500")
    .class_if(is_active, "ring-2")
    .class_list(["rounded", "shadow"])
    .build()
// Results in: class="px-4 py-2 bg-blue-500 ring-2 rounded shadow"
```

### Style Helpers

```rust
use dioxus_builder::ElementBuilder;

fn card_styles(builder: ElementBuilder) -> ElementBuilder {
    builder.class("p-4 rounded shadow-sm").style_prop("gap", "0.75rem")
}

div()
    .with(card_styles)
    .style_prop("display", "flex")
    .child("Styled content")
    .build()
```

### Event Handlers

All standard DOM events are supported:

```rust
button()
    .onclick(|event| println!("Clicked!"))
    .onmouseenter(|_| println!("Mouse entered"))
    .onkeydown(|event| {
        if event.key() == "Enter" {
            println!("Enter pressed");
        }
    })
    .onfocus(|_| println!("Focused"))
    .build()
```

### Keys for List Reconciliation

Use keys for efficient updates in dynamic lists:

```rust
ul().children_keyed(
    items,
    |item| item.id.to_string(),  // Key function
    |item| li().child(&item.name),  // Child builder
).build()

// Or manually:
ul().children(items.iter().map(|item| {
    li().key(&item.id).child(&item.name)
})).build()
```

### Fragments

Build multiple root nodes:

```rust
fragment()
    .child("Text node")
    .child(div().child("Div node"))
    .child(span().child("Span node"))
    .build()
```

## Static vs Dynamic Content (Performance)

For optimal performance, dioxus-builder supports hybrid templates that mix static and dynamic content. Static content is embedded directly in the template and **skips diffing entirely**.

### When to Use Static Content

- Labels and decorative text that never change
- Icons and static UI elements
- Static structural elements

### Example

```rust
div()
    // Static text - embedded in template, no diffing
    .static_text("Welcome, ")

    // Dynamic content - will be diffed on updates
    .child(user_name)

    // More static text
    .static_text("!")
    .build()
```

### Guaranteed Const with `static_str!` Macro

For **guaranteed** compile-time const evaluation, use the `static_str!` macro:

```rust
use dioxus_builder::{div, static_str, BuilderExt};

// Using pipe with closure form
div()
    .pipe(static_str!("Hello, "))  // Guaranteed const
    .child(user_name)               // Dynamic
    .pipe(static_str!("!"))         // Guaranteed const
    .build()

// Or using the two-argument form
let builder = div();
static_str!(builder, "Hello, World!")
    .build()
```

The macro ensures the string is evaluated in a `const` context, guaranteeing it will be embedded in the template.

### Static Elements

For more complex static structures:

```rust
use dioxus_builder::{ChildNode, StaticAttribute, StaticElement};

div()
    .static_element(StaticElement {
        tag: "span",
        namespace: None,
        attrs: &[StaticAttribute {
            name: "class",
            value: "icon text-blue-500",
            namespace: None,
        }],
        children: vec![ChildNode::StaticText("★")],
    })
    .child(dynamic_content)
    .build()
```

### Performance Comparison

| Method | Diffing | Const Guaranteed | Best For |
|--------|---------|------------------|----------|
| `.child("text")` | Yes | N/A | Dynamic content that may change |
| `.text(value)` | Yes | N/A | Dynamic text from variables |
| `.static_text("text")` | No | No* | Static labels, decorative text |
| `static_str!(builder, "text")` | No | Yes | Static text with const verification |
| `.static_element(...)` | No | No* | Static icons, badges, decorations |

*`static_text` accepts `&'static str` which must be a string literal, but the macro provides additional compile-time verification.

## Document Helpers

Enable the `document` feature for document head management:

```rust
use dioxus_builder::document::*;

fn app() -> Element {
    fragment()
        .child(doc_title("My App"))
        .child(doc_stylesheet("/assets/style.css"))
        .child(doc_meta()
            .name("viewport")
            .content("width=device-width, initial-scale=1")
            .build())
        .child(doc_meta()
            .property("og:title")
            .content("My App")
            .build())
        .child(doc_link()
            .rel("icon")
            .href("/favicon.ico")
            .build())
        .child(body_content())
        .build()
}
```

## Form Elements

```rust
form()
    .child(
        input()
            .r#type("text")
            .name("username")
            .placeholder("Enter username")
            .required(true)
            .maxlength(50)
            .value(current_value)
            .oninput(|e| set_value(e.value()))
    )
    .child(
        input()
            .r#type("checkbox")
            .checked(is_checked)
            .onchange(|e| set_checked(e.checked()))
    )
    .child(
        button()
            .r#type("submit")
            .disabled(is_submitting)
            .child("Submit")
    )
    .build()
```

## SVG Support

SVG elements use the correct namespace automatically:

```rust
svg()
    .viewBox("0 0 24 24")
    .fill("none")
    .stroke("currentColor")
    .stroke_width("2")
    .child(path().d("M12 6v6l4 2"))
    .build()
```

## Debugging

Builder types and static nodes implement `Debug`, which helps when inspecting
builder state or template structure in tests and logs.

## Comparison with RSX Macro

| Feature | RSX Macro | Builder API |
|---------|-----------|-------------|
| IDE Autocomplete | Limited | Full |
| Syntax | DSL | Pure Rust |
| For Loops | `for x in items {}` | `.children(items.map(...))` |
| String Interpolation | `"Hello {name}"` | `format!("Hello {}", name)` |
| Conditional Classes | `class: if cond { "x" }` | `.class_if(cond, "x")` |
| Static Text | Automatic | `.static_text()` |
| Hot Reload | Supported | Not supported |
| Learning Curve | Dioxus-specific | Standard Rust |

## Available Elements

### Document Metadata
`head`, `title`, `base`, `link`, `meta`, `style`

### Sectioning
`body`, `article`, `section`, `nav`, `aside`, `header`, `footer`, `h1`-`h6`, `main`, `address`, `hgroup`

### Content
`div`, `p`, `blockquote`, `pre`, `ol`, `ul`, `li`, `dl`, `dt`, `dd`, `figure`, `figcaption`, `hr`, `menu`

### Inline Text
`a`, `span`, `strong`, `em`, `b`, `i`, `u`, `s`, `code`, `kbd`, `samp`, `var`, `mark`, `small`, `sub`, `sup`, `br`, `wbr`, `q`, `cite`, `abbr`, `dfn`, `time`, `data`, `ruby`, `rt`, `rp`, `bdi`, `bdo`

### Media
`img`, `audio`, `video`, `picture`, `source`, `track`, `map`, `area`

### Embedded
`iframe`, `embed`, `object`, `param`, `portal`

### SVG/MathML
`svg`, `math`

### Tables
`table`, `caption`, `thead`, `tbody`, `tfoot`, `tr`, `th`, `td`, `col`, `colgroup`

### Forms
`form`, `input`, `button`, `select`, `option`, `optgroup`, `textarea`, `label`, `fieldset`, `legend`, `datalist`, `output`, `progress`, `meter`

### Interactive
`details`, `summary`, `dialog`

### Scripting
`script`, `noscript`, `canvas`, `template`, `slot`

### Edits
`ins`, `del`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
