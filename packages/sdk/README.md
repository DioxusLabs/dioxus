a "mini" sdk for papering-over common things like

- spawning futures
- networking
- time
- storage

mostly because tokio doesn't provide cross-platform versions of these things and we want to use them in a variety of places
