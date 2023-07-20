# The router book

## How to run the tests
- Navigate your terminal to this directory
- Run `cargo clean`
- Run `cargo build --all --F regex -F serde -F web`
- Run `mdbook test -L ../../target/debug/deps/`
