
```rust
fn Example(ctx: &mut Context<()>) -> VNode {
    let service = use_combubulator(ctx);
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
