# Passing children and attributes

Often times, you'll want to wrap some important functionality *around* your state, not directly nested *inside* another component. In these cases, you'll want to pass elements and attributes into a component and let the component place them appropriately.

In this chapter, you'll learn about:
- Passing elements into components
- Passing attributes into components


## The use case

Let's say you're building a user interface and want to make some part of it a clickable link to another website. You would normally start with the HTML `<a>` tag, like so:

```rust
rsx!(
    a {
        href: "https://google.com"
        "Link to google"
    }
)
```

But, what if we wanted to style our `<a>` tag? Or wrap it with some helper icon? We could abstract our RSX into its own component:


```rust
#[derive(Props)]
struct ClickableProps<'a> {
    href: &'a str,
    title: &'a str
}

fn Clickable(cx: Scope<ClickableProps>) -> Element {
    cx.render(rsx!(
        a {
            href: "{cx.props.href}"
            "{cx.props.title}"
        }
    ))
}
```

And then use it in our code like so:

```rust
rsx!(
    Clickable {
        href: "https://google.com"
        title: "Link to Google"
    }
)
```

Let's say we don't just want the text to be clickable, but we want another element, like an image, to be clickable. How do we implement that?

## Passing children

If we want to pass an image into our component, we can just adjust our props and component to allow any `Element`.

```rust
#[derive(Props)]
struct ClickableProps<'a> {
    href: &'a str,
    body: Element<'a>
}

fn Clickable(cx: Scope<ClickableProps>) -> Element {
    cx.render(rsx!(
        a {
            href: "{cx.props.href}",
            &cx.props.body
        }
    ))
}
```

Then, at the call site, we can render some nodes and pass them in:

```rust
rsx!(
    Clickable {
        href: "https://google.com"
        body: cx.render(rsx!(
            img { src: "https://www.google.com/logos/doodles/..." }
        ))
    }
)
```

## Auto Conversion of the `Children` field

This pattern can become tedious in some instances, so Dioxus actually performs an implicit conversion of any `rsx` calls inside components into `Elements` at the `children` field. This means you must explicitly declare if a component can take children.

```rust
#[derive(Props)]
struct ClickableProps<'a> {
    href: &'a str,
    children: Element<'a>
}

fn Clickable(cx: Scope<ClickableProps>) -> Element {
    cx.render(rsx!(
        a {
            href: "{cx.props.href}",
            &cx.props.children
        }
    ))
}
```

Now, whenever we use `Clickable` in another component, we don't need to call `render` on child nodes - it will happen automatically!
```rust
rsx!(
    Clickable {
        href: "https://google.com"
        img { src: "https://www.google.com/logos/doodles/...." }
    }
)
```

> Note: Passing children into components will break any memoization due to the associated lifetime.

While technically allowed, it's an antipattern to pass children more than once in a component and will probably cause your app to crash.

However, because the `Element` is transparently a `VNode`, we can actually match on it to extract the nodes themselves, in case we are expecting a specific format:

```rust
fn clickable(cx: Scope<ClickableProps>) -> Element {
    match cx.props.children {
        Some(VNode::Text(text)) => {
            // ...
        }
        _ => {
            // ...
        }
    }
}
```

<!-- ## Passing attributes

In the cases where you need to pass arbitrary element properties into a component - say to add more functionality to the `<a>` tag, Dioxus will accept any quoted fields. This is similar to adding arbitrary fields to regular elements using quotes.

```rust

rsx!(
    Clickable {
        "class": "blue-button",
        "style": "background: red;"
    }
)

```

For a component to accept these attributes, you must add an `attributes` field to your component's properties. We can use the spread syntax to add these attributes to whatever nodes are in our component.

```rust
#[derive(Props)]
struct ClickableProps<'a> {
    attributes: Attributes<'a>
}

fn clickable(cx: Scope<ClickableProps>) -> Element {
    cx.render(rsx!(
        a {
            ..cx.props.attributes,
            "Any link, anywhere"
        }
    ))
}
```

The quoted escapes are a great way to make your components more flexible.
 -->

## Passing handlers

Dioxus also provides some implicit conversions from listener attributes into an `EventHandler` for any field on components that starts with `on`. IE `onclick`, `onhover`, etc. For properties, we want to define our `on` fields as an event handler:


```rust
#[derive(Props)]
struct ClickableProps<'a> {
    onclick: EventHandler<'a, MouseEvent>
}

fn clickable(cx: Scope<ClickableProps>) -> Element {
    cx.render(rsx!(
        a {
            onclick: move |evt| cx.props.onclick.call(evt)
        }
    ))
}
```

Then, we can attach a listener at the call site:

```rust
rsx!(
    Clickable {
        onclick: move |_| log::info!("Clicked"),
    }
)
```

Currently, Dioxus does not support an arbitrary amount of listeners - they must be strongly typed in `Properties`. If you need this use case, you can pass in an element with these listeners, or dip down into the `NodeFactory` API.


## Wrapping up

In this chapter, we learned:
- How to pass arbitrary nodes through the tree
- How the `children` field works on component properties
- How the `attributes` field works on component properties
- How to convert `listeners` into `EventHandlers` for components
- How to extend any node with custom attributes and children

Next chapter, we'll talk about conditionally rendering parts of your user interface.

