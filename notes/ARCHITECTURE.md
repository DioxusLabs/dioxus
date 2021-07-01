# Signals

Signals provide a way of forcing updates directly through Dioxus without having to go through the diffing phase.

When diffing is too slow for your use-case, signals can be faster. Signals run at a higher priority than regular diffing, acting as a hint to Dioxus that a signal update needs to take precedence over a subtree update. This can be useful in real-time systems where getting data from a websocket to the screen ASAP is extremely important.

- High
- Medium
- Low

## Signals:

Producer -> Receiver

- The Dioxus VirtualDOM provides built-in receivers for signals.
- Elements themselves act as receivers.
- Any use of a signal schedules the current element and its children for updates.
- Attributes are valid receivers
- Text nodes are valid receivers
- Receivers may not be passed into child components (must be de-referenced)
- When receivers are derefed in a component's properties, the props will be updated in place and the component will re-render with the new value.

```rust
let sig = use_signal(|| 0);

// any updates to the signal will cause the child to re-render completely
Comp {
    prop: *sig
}
```

Using 3 separate signals

```rust
let width = use_signal(|| 0);

cx.request_next_frame(move |frame| async {
    sig1 += 1;
    frame.again();
})

div {
    h2 { "{sig1}" }
    h3 { "{sig2}" }
    h4 { "{sig3}" }
}
```
