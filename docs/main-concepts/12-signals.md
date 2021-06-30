# Signals: Skipping the Diff

In most cases, the traditional VirtualDOM diffing pattern is plenty fast. Dioxus will compare trees of VNodes, find the differences, and then update the Renderer's DOM with the updates. However, this can generate a lot of overhead for certain types of components. In apps where reducing visual latency is a top priority, you can opt into the `Signals` api to entirely disable diffing of hot-path components. Dioxus will then automatically construct a state machine for your component, making updates nearly instant.

What does this look like?

## How does it work?
