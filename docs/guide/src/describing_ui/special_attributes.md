# Special Attributes

While most attributes are simply passed on to the HTML, some have special behaviors:

- `dangerous_inner_html`
- Boolean attributes
- Event handlers as string attributes
- `value`, `checked`, and `selected`

## The HTML escape hatch: `dangerous_inner_html`

One thing you might've missed from React is the ability to render raw HTML directly to the DOM. If you're working with pre-rendered assets, output from templates, or output from a JS library, then you might want to pass HTML directly instead of going through Dioxus. In these instances, reach for `dangerous_inner_html`.

For example, shipping a markdown-to-Dioxus converter might significantly bloat your final application size. Instead, you'll want to pre-render your markdown to HTML and then include the HTML directly in your output. We use this approach for the [Dioxus homepage](https://dioxuslabs.com):


```rust
{{#include ../../examples/dangerous_inner_html.rs:dangerous_inner_html}}
```

> Note! This attribute is called "dangerous_inner_html" because it is **dangerous** to pass it data you don't trust. If you're not careful, you can easily expose cross-site-scripting (XSS) attacks to your users.
>
> If you're handling untrusted input, make sure to sanitize your HTML before passing it into `dangerous_inner_html` – or just pass it to a Text Element to escape any HTML tags.


## Boolean Attributes

Most attributes, when rendered, will be rendered exactly as the input you provided. However, some attributes are considered "boolean" attributes and just their presence determines whether they affect the output. For these attributes, a provided value of `"false"` will cause them to be removed from the target element.

So this RSX:

```rust
{{#include ../../examples/boolean_attribute.rs:boolean_attribute}}
```
wouldn't actually render the `hidden` attribute:
```html
<div>hello</div>
```

Not all attributes work like this however. *Only the following attributes* have this behavior:

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

In Dioxus, there is a distinction between controlled and uncontrolled inputs. Most inputs you'll use are controlled, meaning we both drive the `value` of the input and react to the `oninput`.

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
        // no "value" is driven here – the input keeps track of its own value, and you can't change it
    }
}
```

## Strings for handlers like `onclick`

For element fields that take a handler like `onclick` or `oninput`, Dioxus will let you attach a closure. Alternatively, you can also pass a string using normal attribute syntax and assign this attribute on the DOM.

This lets you use JavaScript (only if your renderer can execute JavaScript).

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

In this chapter, we learned:
- How to declare elements
- How to conditionally render parts of your UI
- How to render lists
- Which attributes are "special"

<!-- todo
There's more to elements! For further reading, check out:

- [Custom Elements]()
-->
