# Dioxus3d: declarative framework for using the `Canvas`

Declarative wrapper over wgpu for creating interactive gpu-enabled visualizations in the browser with Dioxus. This crate's analog is ThreeJS and react-three-fiber. Here, we expose a set of hooks for using the `Canvas` imperatively, and then provide a set of components that interact with this canvas context. From there, declaring scenes is as easy as:

```rust
use dioxus3d::{Canvas};
use dioxus::prelude::*;

static HelloWorld = |ctx, props| {
    ctx.render(rsx! {
        Canvas {
            Text {
                "Hello world"
                rel_pos: (0,0,1)
                size: (1,1,1)
            }
            Cube {
                size: (1,1,1)
            }
        }
    })
};
```

// dioxus bevy: wrap a bevy instance with reactive controls. 
