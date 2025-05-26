harness for testing cross-crate TLS imports.

TLS is the hardest to get right with binary patchinig since we need to rewrite initializers.

this crate relies on two crates - one as a dylib and the other as an rlib

the dylib should make it into our bundle and the rlib should just be linked.

both shouldn't cause access errors or segfaults during hotpatch.

```sh
cargo run --package dioxus-cli -- serve --hotpatch
```
