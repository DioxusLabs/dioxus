# Suspense

Suspense in Dioxus is enabled through placeholder nodes.

just a component that renders nodes into the placeholder after the future is finished?

in react, suspense is just completely pausing diffing while a 

```rust
let n = use_suspense(cx || {
    cx.render(rsx!{
        Suspense {
            prom: fut,
            callback: || {}
        }
    })
})

suspense () {
    let value = use_state();
    if first_render {
        push_task({
            value.set(fut.await);
        });
    } else {
        callback(value)
    }
}


let name = fetch_name().await;


function ProfileDetails() {
  // Try to read user info, although it might not have loaded yet
  const user = resource.read();
  return <h1>{user.name}</h1>;
}


fn ProfileDteails() {
    let user = resource.suspend(cx, |l| rsx!("{l}"));

    // waits for the resource to be ready and updates the placeholder with the tree
    let name = resource.suspend_with(cx, |val| rsx!( div { "hello" "{user}" } ));

    cx.render(rsx!(
        div {
            {user}
            {name}
        }
    ))
}
```
