# A Simple Component

```rust
#[derive(PartialEq, Properties)]
struct Props { name: &'static str }

static HelloMessage: FC<Props> = |cx| {
    cx.render(rsx!{
        div { "Hello {cx.props.name}" }
    })
}
```

# Two syntaxes: html! and rsx!

Choose from a close-to-html syntax or the standard rsx! syntax

```rust
static HelloMessage: FC<()> = |cx| {
    cx.render(html!{
        <div> Hello World! </div>
    })
}
```

# A Stateful Component

Store state with hooks!

```rust
enum LightState {
    Green
    Yellow,
    Red,
}
static HelloMessage: FC<()> = |cx| {
    let (color, set_color) = use_state(&cx, || LightState::Green);

    let title = match color {
        Green => "Green means go",
        Yellow => "Yellow means slow down",
        Red => "Red means stop",
    };

    cx.render(rsx!{
        h1 { "{title}" }
        button { "tick"
            onclick: move |_| set_color(match color {
                Green => Yellow,
                Yellow => Red,
                Red => Green,
            })
        }
    })
}
```
