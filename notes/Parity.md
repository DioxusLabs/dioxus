# Parity with React

Parity has moved to the homepage

## Required services:

---

Gloo is covering a lot of these. We want to build hooks around these and provide examples on how to use them.
https://github.com/rustwasm/gloo

For example, resize observer would function like this:

```rust
static Example: FC<()> = |cx| {
    let observer = use_resize_observer();

    cx.render(rsx!(
        div { ref: observer.node_ref
            "Size, x: {observer.x} y: {observer.y}"
        }
    ))
}
```

However, resize observing is _not_ cross-platform, so this hook (internally) needs to abstract over the rendering platform.

For other services, we shell out to gloo. If the gloo service doesn't exist, then we need to contribute to the project to make sure it exists.

| Service                      | Hook examples | Current Projects |
| ---------------------------- | ------------- | ---------------- |
| Fetch                        | 👀            | Reqwest/surf     |
| Local storage (cache)        | 👀            | Gloo             |
| Persistent storage (IndexDB) | 👀            | 👀               |
| WebSocket                    | 👀            | Gloo             |
| 3D Renderer / WebGL          | 👀            | Gloo             |
| Web Worker                   | 👀            | 👀               |
| Router                       | 👀            | 👀               |
| Notifications                | 👀            | 👀               |
| WebRTC Client                | 👀            | 👀               |
| Service Workers              | 👀            | 👀               |
| Resize Observer              | 👀            | 👀               |
| Canvas                       | 👀            | 👀               |
| Clipboard                    | 👀            | 👀               |
| Fullscreen                   | 👀            | 👀               |
| History API                  | 👀            | 👀               |
