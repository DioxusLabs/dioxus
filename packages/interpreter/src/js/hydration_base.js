// Intentionally empty: prepended verbatim into the inline JS of the
// `hydration_bindings.rs` sledgehammer block. The `HydrationChannel` JS class
// is self-contained (no `extends`); state shared with the mutation interpreter
// lives on `this.base`, populated by `bind_hydration_channel` (Rust) before the
// first flush.
