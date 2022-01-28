# Special Attributes

Dioxus tries its hardest to stay close to React, but there are some divergences and "special behavior" that you should review before moving on.

In this section, we'll cover special attributes built into Dioxus:

- `dangerous_inner_html`
- Boolean attributes
- `prevent_default`
<!-- - `..Attributes` -->
- event handlers as string attributes
- `value`, `checked`, and `selected`

## The HTML escape hatch: `dangerous_inner_html`

One thing you might've missed from React is the ability to render raw HTML directly to the DOM. If you're working with pre-rendered assets, output from templates, or output from a JS library, then you might want to pass HTML directly instead of going through Dioxus. In these instances, reach for `dangerous_inner_html`.

For example, shipping a markdown-to-Dioxus converter might significantly bloat your final application size. Instead, you'll want to pre-render your markdown to HTML and then include the HTML directly in your output. We use this approach for the `http://dioxuslabs.com` site:


```rust
fn BlogPost(cx: Scope) -> Element {
    let contents = include_str!("../post.html");
    cx.render(rsx!{
        div {
            class: "markdown",
            dangerous_inner_html: "{contents}",
        }
    })
}
```

> Note! This attribute is called "dangerous_inner_html" because it is DANGEROUS. If you're not careful, you can easily expose cross-site-scripting (XSS) attacks to your users. If you're handling untrusted input, make sure to escape your HTML before passing it into `dangerous_inner_html`.


## Boolean Attributes

Most attributes, when rendered, will be rendered exactly as the input you provided. However, some attributes are considered "boolean" attributes and just their presence determines whether or not they affect the output. For these attributes, a provided value of `"false"` will cause them to be removed from the target element.

So the input of:

```rust
rsx!{
    div {
        hidden: "false",
        "hello"
    }
}
```
would actually render an output of 
```html
<div>hello</div> 
```

Notice how `hidden` is not present in the final output?

Not all attributes work like this however. Only *these specific attributes* are whitelisted to have this behavior:

- `allowfullscreen`
- `allowpaymentrequest`
- `async`
- `autofocus`
- `autoplay`
- `checked`
- `controls`
- `default`
- `defer`
- `disabled`
- `formnovalidate`
- `hidden`
- `ismap`
- `itemscope`
- `loop`
- `multiple`
- `muted`
- `nomodule`
- `novalidate`
- `open`
- `playsinline`
- `readonly`
- `required`
- `reversed`
- `selected`
- `truespeed`

For any other attributes, a value of `"false"` will be sent directly to the DOM.

## Stopping form input and navigation with `prevent_default`

Currently, calling `prevent_default` on events in EventHandlers is not possible from Desktop/Mobile. Until this is supported, it's possible to prevent default using the `prevent_default` attribute. 

> Note: you cannot conditionally prevent default with this approach. This is a limitation until synchronous event handling is available across the Webview boundary 

To use `prevent_default`, simply attach the `prevent_default` attribute to a given element and set it to the name of the event handler you want to prevent default on. We can attach this attribute multiple times for multiple attributes.

```rust
rsx!{
    input {
        oninput: move |_| {},
        prevent_default: "oninput",

        onclick: move |_| {},
        prevent_default: "onclick",
    }
}
```
<!-- 
## Passing attributes into children: `..Attributes`

> Note: this is an experimental, unstable feature not available in released versions of Dioxus. Feel free to skip this section.

Just like Dioxus supports spreading component props into components, we also support spreading attributes into elements. This lets you pass any arbitrary attributes through components into elements.


```rust
#[derive(Props)]
pub struct InputProps<'a> {
    pub children: Element<'a>,
    pub attributes: Attribute<'a>
}

pub fn StateInput<'a>(cx: Scope<'a, InputProps<'a>>) -> Element {
    cx.render(rsx! (
        input {
            ..cx.props.attributes,
            &cx.props.children,
        }
    ))
}
``` -->

## Controlled inputs and `value`, `checked`, and `selected`

In Dioxus, there is a distinction between controlled and uncontrolled inputs. Most inputs you'll use are "controlled," meaning we both drive the `value` of the input and react to the `oninput`.

Controlled components:
```rust
let value = use_state(&cx, || String::from("hello world"));

rsx! {
    input {
        oninput: move |evt| value.set(evt.value.clone()),
        value: "{value}",
    }
}
```

With uncontrolled inputs, we won't actually drive the value from the component. This has its advantages when we don't want to re-render the component when the user inputs a value. We could either select the element directly - something Dioxus doesn't support across platforms - or we could handle `oninput` and modify a value without causing an update:

```rust
let value = use_ref(&cx, || String::from("hello world"));

rsx! {
    input {
        oninput: move |evt| *value.write_silent() = evt.value.clone(),
        // no "value" is driven
    }
}
```

## Strings for handlers like `onclick`

For element fields that take a handler like `onclick` or `oninput`, Dioxus will let you attach a closure. Alternatively, you can also pass a string using normal attribute syntax and assign this attribute on the DOM.

This lets you escape into JavaScript (only if your renderer can execute JavaScript).

```rust
rsx!{
    div {
        // handle oninput with rust
        oninput: move |_| {},

        // or handle oninput with javascript
        oninput: "alert('hello world')",
    }
}

```

## Wrapping up

We've reached just about the end of what you can do with elements without venturing into "advanced" territory.

In this chapter, we learned:
- How to declare elements
- How to conditionally render parts of your UI
- How to render lists
- Which attributes are "special"

There's more to elements! For further reading, check out:

- [Custom Elements]()
