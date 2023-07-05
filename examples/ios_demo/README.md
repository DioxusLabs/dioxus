# wry

## iOS

Must run Xcode on rosetta. Goto Application > Right Click Xcode > Get Info > Open in Rosetta.

If you are using M1, you will have to run `cargo build --target x86_64-apple-ios` instead of `cargo apple build` if you want to run in simulator.

Otherwise, it's all `cargo apple run` when running in actual device.
