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


## Subtree memoization

The rsx! macro needs to be *really* smart. If it detects that no dynamics are pumped into the macro, then it opts to use the "const" flavors of the element build functions we know and love. This has to be done at build time rather than runtime since components may return basically anything. Using the const flavor enables is_static which encourages Dioxus to do a ptr compare instead of a value compare to short circuit through the diffing. Due to const folding in Rust, entire subtrees can be ruled out at compile time.

It would be interesting to fix the issue of dynamic subtrees by hashing each structure (or just the const structures) or the macro call itself. That way, each call gets its own identifier and we can make sure that two unique structures have different IDs and aren't just opaque to dioxus.

```rust
let s1 = LazyNodes::new("1", move |_| {
    if rand() {
        f.element()
    } else {
        f.element()
    }
});
let s2 = LazyNodes::new("1", move |f| {
    if rand() {
        f.element()
    } else {
        f.element()
    }
});
// produces the same ID with different structures
// perhaps just make this
```
