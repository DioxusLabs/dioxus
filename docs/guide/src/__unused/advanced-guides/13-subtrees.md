# Subtrees

One way of extending the Dioxus VirtualDom is through the use of "Subtrees." Subtrees are chunks of the VirtualDom tree distinct from the rest of the tree. They still participate in event bubbling, diffing, etc, but will have a separate set of edits generated during the diff phase.

For a VirtualDom that has a root tree with two subtrees, the edits follow a pattern of:

Root
-> Tree 1
-> Tree 2
-> Original root tree

- Root edits
- Tree 1 Edits
- Tree 2 Edits
- Root Edits

The goal of this functionality is to enable things like Portals, Windows, and inline alternative renderers without needing to spin up a new VirtualDom.

With the right renderer plugins, a subtree could be rendered as anything - a 3D scene, SVG, or even as the contents of a new window or modal. This functionality is similar to "Portals" in React, but much more "renderer agnostic." Portals, by nature, are not necessarily cross-platform and rely on renderer functionality, so it makes sense to abstract their purpose into the subtree concept.

The desktop renderer comes pre-loaded with the window and notification subtree plugins, making it possible to render subtrees into entirely different windows.

Subtrees also solve the "bridging" issues in React where two different renderers need two different VirtualDoms to work properly. In Dioxus, you only ever need one VirtualDom and the right renderer plugins.


## API

Due to their importance in the hierarchy, Components - not nodes - are treated as subtree roots.


```rust

fn Subtree<P>(cx: Scope<P>) -> DomTree {

}

fn Window() -> DomTree {
    Subtree {
        onassign: move |e| {
            // create window
        }
        children()
    }
}

fn 3dRenderer -> DomTree {
    Subtree {
        onassign: move |e| {
            // initialize bevy
        }
    }
}

```
