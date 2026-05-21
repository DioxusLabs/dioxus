# Dioxus VirtualDom Fuzzer

This crate provides the structured operation model and renderer oracle used by
the local `cargo-fuzz` target in `fuzz/`. LibFuzzer handles coverage guidance,
corpus scheduling, crash storage, and minimization. Mutatis provides the custom
structure-aware mutator for encoded `FuzzCase` values.

The fuzzer drives Dioxus `VirtualDom` updates with template, dynamic-node,
dynamic-attribute, fragment, event-listener, portal/multi-renderer, and suspense
operations. Each case is applied to per-target incremental renderers and checked
against stable re-render and fresh rebuild snapshots.

## Running

Install `cargo-fuzz` if needed:

```sh
cargo install cargo-fuzz
```

Run a short smoke session from this package directory:

```sh
cargo +nightly fuzz run vdom_ops -- -runs=256
```

To replay the package-local corpus from this package directory:

```sh
cargo +nightly fuzz run vdom_ops fuzz/corpus/vdom_ops -- -runs=256
```

From the workspace root, pass the nested fuzz project explicitly:

```sh
cargo +nightly fuzz run --fuzz-dir packages/fuzz/fuzz vdom_ops packages/fuzz/fuzz/corpus/vdom_ops -- -runs=256
```

Run a longer session:

```sh
cargo +nightly fuzz run vdom_ops
```

Minimize a crashing input. This still uses `cargo fuzz tmin`, but the
`vdom_ops` custom mutator detects libFuzzer minimization mode and first runs the
structured operation reducer before falling back to Mutatis shrink candidates:

```sh
cargo +nightly fuzz tmin vdom_ops fuzz/artifacts/vdom_ops/<crash-file>
```

Generate coverage using cargo-fuzz's built-in command:

```sh
cargo +nightly fuzz coverage vdom_ops
```

## How It Works

`fuzz/fuzz_targets/vdom_ops.rs` decodes the raw libFuzzer bytes as a postcard
encoded `FuzzCase`. Invalid raw inputs are ignored by the target. The custom
`fuzz_mutator!` hook decodes the current case, falls back to a valid iterator
branch-sweep seed when decoding fails, mutates the structured case with
`mutatis::Session::new().seed(seed.into())`, and writes the encoded case back to
libFuzzer's input buffer.

Cases are capped at `MAX_STEPS` operations so mutated corpus inputs cannot
produce unbounded replay work.

## Failures

On divergence, the fuzz target prints an SSR replay trace for the failing
operation sequence and then panics. LibFuzzer stores the crashing input under
`fuzz/artifacts/vdom_ops/`; use `cargo fuzz tmin` to minimize it and rerun the
target on the minimized artifact to reproduce the trace.
