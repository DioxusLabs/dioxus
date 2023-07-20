In the cases where you need to pass arbitrary element properties into a component – say to add more functionality to the `<a>` tag, Dioxus will accept any quoted fields. This is similar to adding arbitrary fields to regular elements using quotes.

```rust, no_run

rsx!(
    Clickable {
        "class": "blue-button",
        "style": "background: red;"
    }
)

```

For a component to accept these attributes, you must add an `attributes` field to your component's properties. We can use the spread syntax to add these attributes to whatever nodes are in our component.

```rust, no_run
#[derive(Props)]
struct ClickableProps<'a> {
    attributes: Attributes<'a>
}

fn clickable(cx: Scope<ClickableProps<'a>>) -> Element {
    cx.render(rsx!(
        a {
            ..cx.props.attributes,
            "Any link, anywhere"
        }
    ))
}
```
