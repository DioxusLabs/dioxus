# Dioxus-RSX

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-rsx.svg
[crates-url]: https://crates.io/crates/dioxus-rsx
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.5/) |
[API Docs](https://docs.rs/dioxus-rsx/latest/dioxus_rsx) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

This crate provides the actual DSL that Dioxus uses in the `rsx!` macro. This crate is separate from the macro crate to enable tooling like autoformat, translation, and AST manipulation (extract to component).

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## Design

The architecture of this crate has gone through several redesigns and we're hoping that we've finally landed on a final design that we're happy with.

There is still some thinking that we could approach this by flattening the AST to have better incremental parsing and performance without having to add metadata to the data structures.

For now, however, the design we've settled on:

- A top-level CallBody containing a single TemplateBody
- Each TemplateBody has its dynamic mapping on it, filled in after parsing (...not during)
- Hotreload Information for things like literals is propagated as a final pass into nodes on the tree

This lets us incrementally build up the tree to make it modular/easier to test in exchange for extra "bookkeeping passes".

The good part of bookkeeping passes is:
- They're optional
- They're generally pretty quick
- They can be enforced as part of tree construction in the various levels

Of course the major downside is they're 1) slower and 2) need a way to store metadata, which they do on the nodes themselves. Sometimes you might not even want to collect metadata which we currently have no way of opting-out of.

This can make global reasoning about the tree a bit harder since you need to dig into each node for its metadata, but the local reasoning is much better. Previously we stored the metadata on a secondary "view" type item - and this was fine... but it's not ideal for long-lived programs that persist a queryable tree-object.

We want to query the tree:
- For its dynamic mappings
- For its hot-reloadable contents

An alternative approach would be
- flattened AST
- single "parser" IE `self.parse_element` / `self.parse_component`

I've done a bunch of research on parsers and cannot find many (any?) parsers that are implemented statefully.

We *do* want a reference to these items - currently its their path, but it could be done via just an ID for each node. Unfortunately there's no easy way to "pool" these up other than Arc/Rc.

Anyways, until our applications are limited by query performance, keeping the tree unflattened seems fine. The JS DOM has a similar API where it flushes changes after modifications, but also has an ID system via pointers. We could try adding IDs with something like generational-box or just Arc/Rc.

Note that we still don't get the "goodies" of a proper tree-sitter type parser (skipping tokenization incrementally) but that really isn't achievable without being a part of the LSP. In theory, we can eventually use the LSP to help modify the tree in place and then "fix" it if you wanted to modify trees in place. That being said, we still need to perform diffing of two rsx! trees even when we know their shape has changed.

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT, without any additional
terms or conditions.
