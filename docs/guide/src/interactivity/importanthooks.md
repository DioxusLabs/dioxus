# `use_state` and `use_ref`

Most components you will write in Dioxus will need to store state somehow. For local state, we provide two very convenient hooks:

- [use_state](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_state.html)
- [use_ref](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_ref.html)

Both of these hooks are extremely powerful and flexible, so we've dedicated this section to understanding them properly.

> These two hooks are not the only way to store state. You can always build your own hooks!
