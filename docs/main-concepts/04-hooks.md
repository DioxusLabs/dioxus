```rust
fn Example(cx: &mut Context<()>) -> VNode {
    let service = use_combubulator(cx);
    let Status { name, pending, count } = service.info();
    html! {
        <div>
            <p> "Hello, {name}!" </p>
            <p> "Status: {pending}!" </p>
            <p> "Count {count}!" </p>
            <button onclick=|_| service.update()>
                "Refresh services"
            </button>
        </div>
    }
}
```
