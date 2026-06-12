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
`fuzz_mutator!` hook decodes the current case (starting from an empty case
when decoding fails), calls this crate's structured mutator, and writes the
encoded case back to libFuzzer's input buffer.

The structured mutator (`src/mutator.rs`) combines field-level mutations of
encoded ops with a table of model-aware op strategies: it replays the ops
before a splice point, summarizes the resulting model, and inserts op
sequences that target vnodes, fragments, attribute slots, and suspense
boundaries that actually exist at that point. Strategies whose target
structure is missing emit their own prerequisite ops first.

Cases are capped at the crate-internal step limit so mutated corpus inputs
cannot produce unbounded replay work.

Crate layout:

- `src/case.rs` — encoded op stream, replay, failure reporting
- `src/ops.rs` — the operation grammar and how ops apply to the model
- `src/model.rs` — the spec tree the generated app renders from
- `src/mutator.rs` — structure-aware mutation (the op strategy table)
- `src/reducer.rs` — structured shrinking of failing cases
- `src/harness.rs` — incremental-vs-fresh renderer oracle and lifecycle checks
- `src/vdom.rs` — compiles model specs into real `VNode`s/`Template`s
- `src/warmup.rs` — one-shot scenarios for paths replay cannot reach
- `src/targeted.rs` — hand-built recipes replayed as tests and exportable as
  corpus seeds

## Failures

On divergence, the fuzz target prints an SSR replay trace for the failing
operation sequence and then panics. LibFuzzer stores the crashing input under
`fuzz/artifacts/vdom_ops/`; use `cargo fuzz tmin` to minimize it and rerun the
target on the minimized artifact to reproduce the trace.
